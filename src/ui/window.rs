use gtk4::prelude::*;
use gtk4::{gio, glib, FileDialog, FileFilter, Image};
use libadwaita::prelude::*;
use libadwaita::{Application, ApplicationWindow, ButtonContent, HeaderBar, ToolbarView};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::ui::reader::ReaderView;

pub struct ManhwaWindow {
    pub window: ApplicationWindow,
    pub reader: Rc<ReaderView>,
}

impl ManhwaWindow {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Continuum — Manhwa Reader")
            .icon_name("dev.continuum.ManhwaReader")
            .build();

        // Calculate screen height & screen width <= 60%
        if let Some(display) = gdk4::Display::default() {
            let monitors = display.monitors();
            if let Some(monitor_item) = monitors.item(0) {
                if let Ok(monitor) = monitor_item.downcast::<gdk4::Monitor>() {
                    let geom = monitor.geometry();
                    let screen_w = geom.width();
                    let screen_h = geom.height();

                    // Max 60% width, full height by default
                    let target_w = ((screen_w as f64) * 0.58) as i32;
                    let target_h = screen_h;

                    window.set_default_size(target_w, target_h);
                }
            }
        }

        let toolbar_view = ToolbarView::new();
        let header_bar = HeaderBar::new();

        let app_icon = Image::from_icon_name("dev.continuum.ManhwaReader");
        app_icon.set_icon_size(gtk4::IconSize::Large);

        let open_btn_content = ButtonContent::builder()
            .icon_name("document-open-symbolic")
            .label("Open .cbz")
            .build();

        let open_btn = gtk4::Button::builder()
            .child(&open_btn_content)
            .tooltip_text("Open Manhwa CBZ Archive (Ctrl+O)")
            .css_classes(vec!["flat".to_string()])
            .build();

        let mode_btn = gtk4::Button::builder()
            .icon_name("view-column-symbolic")
            .tooltip_text("Toggle Reading Mode: Manhwa (Vertical) / Manga (Horizontal) (m)")
            .css_classes(vec!["flat".to_string()])
            .build();

        let help_btn = gtk4::Button::builder()
            .icon_name("help-browser-symbolic")
            .tooltip_text("Keyboard Shortcuts (?)")
            .css_classes(vec!["flat".to_string()])
            .build();

        header_bar.pack_start(&app_icon);
        header_bar.pack_start(&open_btn);
        header_bar.pack_start(&mode_btn);
        header_bar.pack_end(&help_btn);
        toolbar_view.add_top_bar(&header_bar);

        let reader = Rc::new(ReaderView::new());
        toolbar_view.set_content(Some(&reader.container));
        window.set_content(Some(&toolbar_view));

        let reader_mode = reader.clone();
        mode_btn.connect_clicked(move |_| {
            reader_mode.toggle_reading_mode();
        });

        let win_help = window.clone();
        help_btn.connect_clicked(move |_| {
            show_shortcuts_dialog(&win_help);
        });

        let header_bar_clone = header_bar.clone();
        window.connect_fullscreened_notify(move |win| {
            let is_fullscreen = win.is_fullscreen();
            header_bar_clone.set_visible(!is_fullscreen);
        });

        let window_struct = Self { window, reader };
        window_struct.setup_actions(open_btn);
        window_struct
    }

    fn setup_actions(&self, open_btn: gtk4::Button) {
        let win_clone = self.window.clone();
        let reader_clone = self.reader.clone();

        let open_file_fn = move || {
            let filter = FileFilter::new();
            filter.add_pattern("*.cbz");
            filter.add_pattern("*.zip");
            filter.set_name(Some("Comic Archives (*.cbz, *.zip)"));

            let filters = gio::ListStore::new::<FileFilter>();
            filters.append(&filter);

            let dialog = FileDialog::builder()
                .title("Select Manhwa CBZ File")
                .modal(true)
                .filters(&filters)
                .default_filter(&filter)
                .build();

            let reader = reader_clone.clone();
            dialog.open(Some(&win_clone), gio::Cancellable::NONE, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        if let Err(e) = reader.load_initial_file(path) {
                            eprintln!("Error loading file: {}", e);
                        }
                    }
                }
            });
        };

        let open_file_rc = Rc::new(open_file_fn);
        let open_cb1 = open_file_rc.clone();
        open_btn.connect_clicked(move |_| {
            open_cb1();
        });

        let open_cb2 = open_file_rc.clone();
        let chapters = self.reader.chapters.clone();
        let gesture = gtk4::GestureClick::new();
        gesture.connect_pressed(move |_, _, _, _| {
            if chapters.borrow().is_empty() {
                open_cb2();
            }
        });
        self.reader.container.add_controller(gesture);

        let key_controller = gtk4::EventControllerKey::new();
        let open_cb3 = open_file_rc.clone();
        let reader_key = self.reader.clone();
        let win_key = self.window.clone();
        let last_g_time = Rc::new(RefCell::new(None::<std::time::Instant>));

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum ScrollDir {
            FineDown,
            FineUp,
            FastDown,
            FastUp,
        }

        impl ScrollDir {
            fn delta(self) -> f64 {
                match self {
                    ScrollDir::FineDown => 40.0,
                    ScrollDir::FineUp => -40.0,
                    ScrollDir::FastDown => 160.0,
                    ScrollDir::FastUp => -160.0,
                }
            }
        }

        let held_dir = Rc::new(RefCell::new(None::<ScrollDir>));
        let is_repeating = Rc::new(RefCell::new(false));

        let held_dir_press = held_dir.clone();
        let is_repeating_press = is_repeating.clone();
        let reader_press = reader_key.clone();

        key_controller.connect_key_pressed(move |_, keyval, _code, state| {
            let has_ctrl = state.contains(gdk4::ModifierType::CONTROL_MASK);
            let has_shift = state.contains(gdk4::ModifierType::SHIFT_MASK);

            let reading_mode = *reader_key.reading_mode.borrow();

            // Toggle Reading Mode (m / M)
            if !has_ctrl && (keyval == gdk4::Key::m || keyval == gdk4::Key::M) {
                reader_key.toggle_reading_mode();
                return glib::Propagation::Stop;
            }

            if reading_mode == crate::ui::page_widget::ReadingMode::ContinuousHorizontal {
                // Manga Mode Navigation (Horizontal Left-to-Right page stepping)
                if !has_ctrl
                    && (keyval == gdk4::Key::j
                        || keyval == gdk4::Key::J
                        || keyval == gdk4::Key::l
                        || keyval == gdk4::Key::Right
                        || keyval == gdk4::Key::Page_Down)
                {
                    reader_key.next_page();
                    return glib::Propagation::Stop;
                } else if !has_ctrl
                    && (keyval == gdk4::Key::k
                        || keyval == gdk4::Key::K
                        || keyval == gdk4::Key::h
                        || keyval == gdk4::Key::Left
                        || keyval == gdk4::Key::Page_Up)
                {
                    reader_key.prev_page();
                    return glib::Propagation::Stop;
                }
            } else {
                // Manhwa Mode Navigation (Vertical continuous line scrolling)
                let maybe_dir = if !has_ctrl
                    && (keyval == gdk4::Key::J
                        || (keyval == gdk4::Key::Down && has_shift)
                        || (keyval == gdk4::Key::j && has_shift))
                {
                    Some(ScrollDir::FastDown)
                } else if !has_ctrl && (keyval == gdk4::Key::j || keyval == gdk4::Key::Down) {
                    Some(ScrollDir::FineDown)
                } else if !has_ctrl
                    && (keyval == gdk4::Key::K
                        || (keyval == gdk4::Key::Up && has_shift)
                        || (keyval == gdk4::Key::k && has_shift))
                {
                    Some(ScrollDir::FastUp)
                } else if !has_ctrl && (keyval == gdk4::Key::k || keyval == gdk4::Key::Up) {
                    Some(ScrollDir::FineUp)
                } else {
                    None
                };

                if let Some(dir) = maybe_dir {
                    *held_dir_press.borrow_mut() = Some(dir);

                    if !*is_repeating_press.borrow() {
                        *is_repeating_press.borrow_mut() = true;
                        reader_press.smooth_scroll_by(dir.delta());

                        let held_dir_tick = held_dir_press.clone();
                        let is_repeating_tick = is_repeating_press.clone();
                        let reader_tick = reader_press.clone();

                        glib::timeout_add_local(Duration::from_millis(22), move || {
                            if let Some(d) = *held_dir_tick.borrow() {
                                reader_tick.smooth_scroll_by(d.delta() * 0.4);
                                glib::ControlFlow::Continue
                            } else {
                                *is_repeating_tick.borrow_mut() = false;
                                glib::ControlFlow::Break
                            }
                        });
                    }
                    return glib::Propagation::Stop;
                }
            }

            if has_ctrl && keyval == gdk4::Key::o {
                open_cb3();
                glib::Propagation::Stop
            } else if !has_ctrl && (keyval == gdk4::Key::q || keyval == gdk4::Key::Q) {
                win_key.close();
                glib::Propagation::Stop
            } else if !has_ctrl
                && (keyval == gdk4::Key::question || (keyval == gdk4::Key::slash && has_shift))
            {
                show_shortcuts_dialog(&win_key);
                glib::Propagation::Stop
            } else if !has_ctrl && (keyval == gdk4::Key::f || keyval == gdk4::Key::F) && !has_shift
            {
                if win_key.is_fullscreen() {
                    win_key.unfullscreen();
                } else {
                    win_key.fullscreen();
                }
                glib::Propagation::Stop
            } else if has_ctrl && (keyval == gdk4::Key::f || keyval == gdk4::Key::F) {
                let page_size = reader_key.scrolled_window.vadjustment().page_size();
                reader_key.smooth_scroll_by(page_size * 0.9);
                glib::Propagation::Stop
            } else if has_ctrl && (keyval == gdk4::Key::b || keyval == gdk4::Key::B) {
                let page_size = reader_key.scrolled_window.vadjustment().page_size();
                reader_key.smooth_scroll_by(-page_size * 0.9);
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::d || (has_ctrl && keyval == gdk4::Key::d) {
                let page_size = reader_key.scrolled_window.vadjustment().page_size();
                reader_key.smooth_scroll_by(page_size * 0.5);
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::u || (has_ctrl && keyval == gdk4::Key::u) {
                let page_size = reader_key.scrolled_window.vadjustment().page_size();
                reader_key.smooth_scroll_by(-page_size * 0.5);
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::G {
                // G (Shift+G): Jump to absolute bottom of last page image
                reader_key.smooth_scroll_to(f64::MAX);
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::g {
                let now = std::time::Instant::now();
                let is_double_g = last_g_time
                    .borrow()
                    .is_some_and(|t| now.duration_since(t).as_millis() < 400);
                if is_double_g {
                    *last_g_time.borrow_mut() = None;
                    reader_key.jump_to_current_chapter_top();
                } else {
                    *last_g_time.borrow_mut() = Some(now);
                }
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::h
                || keyval == gdk4::Key::bracketleft
                || keyval == gdk4::Key::Left
                || keyval == gdk4::Key::Page_Up
            {
                reader_key.prev_chapter();
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::l
                || keyval == gdk4::Key::bracketright
                || keyval == gdk4::Key::Right
                || keyval == gdk4::Key::Page_Down
            {
                reader_key.next_chapter();
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::space {
                let page_size = reader_key.scrolled_window.vadjustment().page_size();
                if has_shift {
                    reader_key.smooth_scroll_by(-page_size * 0.9);
                } else {
                    reader_key.smooth_scroll_by(page_size * 0.9);
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });

        let held_dir_release = held_dir.clone();
        key_controller.connect_key_released(move |_, keyval, _code, _state| {
            if keyval == gdk4::Key::j
                || keyval == gdk4::Key::J
                || keyval == gdk4::Key::Down
                || keyval == gdk4::Key::k
                || keyval == gdk4::Key::K
                || keyval == gdk4::Key::Up
            {
                *held_dir_release.borrow_mut() = None;
            }
        });

        self.window.add_controller(key_controller);
    }
}

pub fn show_shortcuts_dialog(parent: &impl IsA<gtk4::Window>) {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<interface>
  <object class="GtkShortcutsWindow" id="shortcuts_window">
    <property name="modal">True</property>

    <child>
      <object class="GtkShortcutsSection">
        <property name="visible">True</property>
        <property name="section-name">shortcuts</property>

        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">Reading Mode &amp; Navigation</property>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">m</property>
                <property name="title">Toggle Reading Mode (Manhwa / Manga)</property>
              </object>
            </child>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">j k</property>
                <property name="title">Next / Previous Page (Manga) or Scroll Down / Up (Manhwa)</property>
              </object>
            </child>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">&lt;Shift&gt;j &lt;Shift&gt;k</property>
                <property name="title">Fast Step Scroll Down / Up (200px)</property>
              </object>
            </child>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">d u</property>
                <property name="title">Half Page Down / Up</property>
              </object>
            </child>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">space &lt;Shift&gt;space</property>
                <property name="title">Full Page Down / Up</property>
              </object>
            </child>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">g g / &lt;Shift&gt;g</property>
                <property name="title">Jump to Top / Bottom</property>
              </object>
            </child>
          </object>
        </child>

        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">Chapter Navigation</property>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">h l</property>
                <property name="title">Previous / Next Chapter</property>
              </object>
            </child>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">bracketleft bracketright</property>
                <property name="title">Previous / Next Chapter ([ / ])</property>
              </object>
            </child>
          </object>
        </child>

        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">Application Controls</property>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">&lt;Primary&gt;o</property>
                <property name="title">Open .cbz Archive</property>
              </object>
            </child>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">f</property>
                <property name="title">Toggle Fullscreen</property>
              </object>
            </child>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">question</property>
                <property name="title">Keyboard Shortcuts Cheat Sheet</property>
              </object>
            </child>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">q</property>
                <property name="title">Quit Application</property>
              </object>
            </child>
          </object>
        </child>

      </object>
    </child>
  </object>
</interface>
"#;

    let builder = gtk4::Builder::from_string(xml);
    if let Some(dialog) = builder.object::<gtk4::ShortcutsWindow>("shortcuts_window") {
        dialog.set_transient_for(Some(parent));
        dialog.present();
    }
}
