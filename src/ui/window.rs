use gtk4::prelude::*;
use gtk4::{gio, glib, FileDialog, FileFilter, Image};
use libadwaita::prelude::*;
use libadwaita::{Application, ApplicationWindow, ButtonContent, HeaderBar, ToolbarView};
use std::cell::RefCell;
use std::rc::Rc;

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

        let prev_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text("Previous Chapter (h / Left Arrow / Page Up)")
            .css_classes(vec!["flat".to_string()])
            .build();

        let next_btn = gtk4::Button::builder()
            .icon_name("go-next-symbolic")
            .tooltip_text("Next Chapter (l / Right Arrow / Page Down)")
            .css_classes(vec!["flat".to_string()])
            .build();

        let nav_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        nav_box.add_css_class("linked");
        nav_box.append(&prev_btn);
        nav_box.append(&next_btn);

        let help_btn = gtk4::Button::builder()
            .icon_name("help-browser-symbolic")
            .tooltip_text("Keyboard Shortcuts (?)")
            .css_classes(vec!["flat".to_string()])
            .build();

        header_bar.pack_start(&app_icon);
        header_bar.pack_start(&open_btn);
        header_bar.pack_start(&nav_box);
        header_bar.pack_end(&help_btn);
        toolbar_view.add_top_bar(&header_bar);

        let reader = Rc::new(ReaderView::new());
        toolbar_view.set_content(Some(&reader.container));
        window.set_content(Some(&toolbar_view));

        let reader_prev = reader.clone();
        prev_btn.connect_clicked(move |_| {
            reader_prev.prev_chapter();
        });

        let reader_next = reader.clone();
        next_btn.connect_clicked(move |_| {
            reader_next.next_chapter();
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
        self.reader.set_on_open_file(move || {
            open_cb2();
        });

        // Add Vim keyboard shortcuts (j, k, J, K, d, u, Ctrl+F, Ctrl+B, gg, G, h, l, q, f, ?, Ctrl+O)
        let key_controller = gtk4::EventControllerKey::new();
        let open_cb3 = open_file_rc.clone();
        let reader_key = self.reader.clone();
        let win_key = self.window.clone();
        let last_g_time = Rc::new(RefCell::new(None::<std::time::Instant>));

        key_controller.connect_key_pressed(move |_, keyval, _code, state| {
            let has_ctrl = state.contains(gdk4::ModifierType::CONTROL_MASK);
            let has_shift = state.contains(gdk4::ModifierType::SHIFT_MASK);

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
            } else if keyval == gdk4::Key::J
                || (keyval == gdk4::Key::Down && has_shift)
                || (keyval == gdk4::Key::j && has_shift)
            {
                // Shift+J / Shift+Down: Fast step scroll down (200px)
                reader_key.smooth_scroll_by(200.0);
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::j || keyval == gdk4::Key::Down {
                // j / Down: Fine step scroll down (32px)
                reader_key.smooth_scroll_by(32.0);
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::K
                || (keyval == gdk4::Key::Up && has_shift)
                || (keyval == gdk4::Key::k && has_shift)
            {
                // Shift+K / Shift+Up: Fast step scroll up (200px)
                reader_key.smooth_scroll_by(-200.0);
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::k || keyval == gdk4::Key::Up {
                // k / Up: Fine step scroll up (32px)
                reader_key.smooth_scroll_by(-32.0);
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
                let vadj = reader_key.scrolled_window.vadjustment();
                let max_scroll = (vadj.upper() - vadj.page_size()).max(0.0);
                reader_key.smooth_scroll_to(max_scroll);
                glib::Propagation::Stop
            } else if keyval == gdk4::Key::g {
                let now = std::time::Instant::now();
                let is_double_g = last_g_time
                    .borrow()
                    .map_or(false, |t| now.duration_since(t).as_millis() < 400);
                if is_double_g {
                    *last_g_time.borrow_mut() = None;
                    reader_key.smooth_scroll_to(0.0);
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
        self.window.add_controller(key_controller);
    }
}

pub fn show_shortcuts_dialog(parent: &impl IsA<gtk4::Window>) {
    let xml = r#"
<interface>
  <object class="GtkShortcutsWindow" id="shortcuts_window">
    <property name="modal">True</property>

    <child>
      <object class="GtkShortcutsSection">
        <property name="visible">True</property>
        <property name="section-name">shortcuts</property>

        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">Vim Smooth Navigation</property>

            <child>
              <object class="GtkShortcutsShortcut">
                <property name="accelerator">j k</property>
                <property name="title">Fine Step Scroll Down / Up (32px)</property>
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
