use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Orientation, ScrolledWindow};
use libadwaita::{Clamp, StatusPage};

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::cache::{MemoryManager, PageKey};
use crate::cbz::{CbzArchive, DecodedImagePayload, DirectorySeries};
use crate::ui::chapter_banner::create_chapter_banner;
use crate::ui::page_widget::{PageWidget, ReadingMode};

pub struct ChapterState {
    pub chapter_id: usize,
    pub archive: CbzArchive,
    pub banner_widget: GtkBox,
}

/// Progress for one chapter as read in this session. Every number is local to
/// its own archive — never a global page offset across chapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterProgressEntry {
    pub file: String,
    pub last_page: i64,
    pub completed: bool,
}

/// The dewey integration payload emitted on window close. `chapters` enumerates
/// every archive actually read; the legacy top-level fields mirror the first
/// chapter for backward compatibility with older consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitPayload {
    pub last_page: i64,
    pub completed: bool,
    pub mode: String,
    pub chapters: Vec<ChapterProgressEntry>,
}

#[derive(Clone)]
pub struct ReaderView {
    pub container: GtkBox,
    pub clamp: Clamp,
    pub scrolled_window: ScrolledWindow,
    pub content_box: GtkBox,
    pub status_page: StatusPage,

    // State
    pub series: Rc<RefCell<Option<DirectorySeries>>>,
    pub chapters: Rc<RefCell<Vec<ChapterState>>>,
    pub page_widgets: Rc<RefCell<HashMap<PageKey, PageWidget>>>,
    pub global_to_key: Rc<RefCell<Vec<PageKey>>>,
    pub memory_manager: Rc<RefCell<MemoryManager>>,
    pub first_loaded_series_idx: Rc<RefCell<usize>>,
    pub last_loaded_series_idx: Rc<RefCell<usize>>,
    pub next_chapter_id: Rc<RefCell<usize>>,
    pub last_vadj_value: Rc<RefCell<f64>>,
    pub in_flight: Rc<RefCell<HashSet<PageKey>>>,
    pub target_y: Rc<RefCell<Option<f64>>>,
    pub target_x: Rc<RefCell<Option<f64>>>,
    pub is_animating: Rc<RefCell<bool>>,
    pub reading_mode: Rc<RefCell<ReadingMode>>,
    pub generation_id: Rc<RefCell<usize>>,
    pub storage_profile: Rc<RefCell<String>>,
    pub focused_page_idx: Rc<RefCell<usize>>,

    // Channels
    pub tx: Sender<(PageKey, usize, Option<DecodedImagePayload>)>,
    pub rx: Receiver<(PageKey, usize, Option<DecodedImagePayload>)>,
}

impl ReaderView {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);

        // Empty Status Page
        let status_page = StatusPage::builder()
            .icon_name("dev.continuum.ManhwaReader")
            .title("No Comic Open")
            .description("Click anywhere or press Ctrl+O to select a .cbz Manhwa file")
            .vexpand(true)
            .hexpand(true)
            .build();

        let open_button = Button::builder()
            .label("Open .cbz File")
            .css_classes(vec!["suggested-action".to_string(), "pill".to_string()])
            .halign(Align::Center)
            .build();

        status_page.set_child(Some(&open_button));

        // Manhwa content vertical box
        let content_box = GtkBox::new(Orientation::Vertical, 0);
        content_box.set_margin_start(0);
        content_box.set_margin_end(0);
        content_box.set_visible(false);

        // Clamp content width to max 60% / 900px
        let clamp = Clamp::builder()
            .maximum_size(900)
            .tightening_threshold(700)
            .child(&content_box)
            .build();

        let scrolled_window = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .child(&clamp)
            .build();

        container.append(&status_page);
        container.append(&scrolled_window);

        let series = Rc::new(RefCell::new(None));
        let chapters = Rc::new(RefCell::new(Vec::new()));
        let page_widgets = Rc::new(RefCell::new(HashMap::new()));
        let global_to_key = Rc::new(RefCell::new(Vec::new()));
        let memory_manager = Rc::new(RefCell::new(MemoryManager::new()));
        let first_loaded_series_idx = Rc::new(RefCell::new(0));
        let last_loaded_series_idx = Rc::new(RefCell::new(0));
        let next_chapter_id = Rc::new(RefCell::new(0));
        let last_vadj_value = Rc::new(RefCell::new(0.0));
        let in_flight = Rc::new(RefCell::new(HashSet::new()));
        let target_y = Rc::new(RefCell::new(None));
        let target_x = Rc::new(RefCell::new(None));
        let is_animating = Rc::new(RefCell::new(false));
        let reading_mode = Rc::new(RefCell::new(ReadingMode::ContinuousVertical));
        let generation_id = Rc::new(RefCell::new(0));
        let storage_profile = Rc::new(RefCell::new("fast".to_string()));
        let focused_page_idx = Rc::new(RefCell::new(0));

        let (tx, rx) = unbounded::<(PageKey, usize, Option<DecodedImagePayload>)>();

        let reader = Self {
            container,
            clamp,
            scrolled_window,
            content_box,
            status_page,
            series,
            chapters,
            page_widgets,
            global_to_key,
            memory_manager,
            first_loaded_series_idx,
            last_loaded_series_idx,
            next_chapter_id,
            last_vadj_value,
            in_flight,
            target_y,
            target_x,
            is_animating,
            reading_mode,
            generation_id,
            storage_profile,
            focused_page_idx,
            tx,
            rx,
        };

        reader.setup_scroll_listener();

        // Re-fit page widths and margins whenever the viewport changes
        // (window resize or adjustment changes) while in horizontal/manga mode.
        let reader_refit_h = reader.clone();
        reader
            .scrolled_window
            .hadjustment()
            .connect_page_size_notify(move |_| {
                reader_refit_h.refit_manga_layout();
                reader_refit_h.center_focused_page();
            });

        let reader_refit_v = reader.clone();
        reader
            .scrolled_window
            .vadjustment()
            .connect_page_size_notify(move |_| {
                reader_refit_v.refit_manga_layout();
                reader_refit_v.center_focused_page();
            });

        let reader_refit_ch = reader.clone();
        reader
            .scrolled_window
            .vadjustment()
            .connect_changed(move |_| {
                reader_refit_ch.refit_manga_layout();
            });

        reader
    }

    pub fn set_storage_profile(&self, profile: &str) {
        *self.storage_profile.borrow_mut() = profile.to_string();
    }

    pub fn is_usb_mode(&self) -> bool {
        self.storage_profile.borrow().eq_ignore_ascii_case("usb")
    }

    pub fn load_initial_file(&self, path: PathBuf) -> Result<(), String> {
        let series = DirectorySeries::new(&path);
        let archive = CbzArchive::open(&path)?;

        // Reset state
        self.clear();

        let initial_idx = series.current_index;
        *self.first_loaded_series_idx.borrow_mut() = initial_idx;
        *self.last_loaded_series_idx.borrow_mut() = initial_idx;

        *self.series.borrow_mut() = Some(series);
        self.status_page.set_visible(false);
        self.content_box.set_visible(true);

        // Load current chapter chosen by user (e.g. Chapter 70)
        self.append_chapter(archive, initial_idx)?;

        // Reset scroll position to top (Page 1) — immediate + idle safety net
        let vadj = self.scrolled_window.vadjustment();
        let last_vadj = self.last_vadj_value.clone();
        vadj.set_value(0.0);
        *last_vadj.borrow_mut() = 0.0;

        let vadj2 = self.scrolled_window.vadjustment();
        let last_vadj2 = self.last_vadj_value.clone();
        glib::idle_add_local(move || {
            vadj2.set_value(0.0);
            *last_vadj2.borrow_mut() = 0.0;
            glib::ControlFlow::Break
        });

        // Force initial lazy load pass starting from Page 1 (global_idx 0)
        self.request_pages_around(0);

        Ok(())
    }

    fn append_chapter(&self, archive: CbzArchive, series_idx: usize) -> Result<(), String> {
        let chapter_id = *self.next_chapter_id.borrow();
        *self.next_chapter_id.borrow_mut() += 1;

        let total_pages = archive.page_count();

        let banner = create_chapter_banner(&archive.filename, total_pages);
        self.content_box.append(&banner);

        let mode = *self.reading_mode.borrow();
        let viewport_h = self.get_viewport_height();
        let viewport_w = self.get_viewport_width();
        for page_idx in 0..total_pages {
            let (w, h) = archive.get_dimensions(page_idx);
            let key = PageKey {
                chapter_idx: chapter_id,
                page_idx,
            };
            let page_widget = PageWidget::new(key.clone(), page_idx, total_pages, w, h);
            page_widget.update_layout_for_mode(mode, viewport_w, viewport_h);
            self.content_box.append(&page_widget.container);
            self.page_widgets
                .borrow_mut()
                .insert(key.clone(), page_widget);
            self.global_to_key.borrow_mut().push(key);
        }

        self.chapters.borrow_mut().push(ChapterState {
            chapter_id,
            archive,
            banner_widget: banner,
        });

        *self.last_loaded_series_idx.borrow_mut() = series_idx;

        if mode == ReadingMode::ContinuousHorizontal {
            self.refit_manga_layout();
        }

        Ok(())
    }

    fn prepend_chapter(&self, archive: CbzArchive, series_idx: usize) -> Result<f64, String> {
        let prev_chap_id = self.chapters.borrow().first().map_or(1, |c| c.chapter_id) + 1;
        let total_pages = archive.page_count();

        let new_keys: Vec<PageKey> = (0..total_pages)
            .map(|page_idx| PageKey {
                chapter_idx: prev_chap_id,
                page_idx,
            })
            .collect();

        self.global_to_key
            .borrow_mut()
            .splice(0..0, new_keys.clone());

        let mode = *self.reading_mode.borrow();
        let viewport_h = self.get_viewport_height();
        let viewport_w = self.get_viewport_width();
        let mut prepended_h = 100.0;
        for page_idx in (0..total_pages).rev() {
            let (w, h) = archive.get_dimensions(page_idx);
            let key = &new_keys[page_idx];
            let pw = PageWidget::new(key.clone(), page_idx, total_pages, w, h);
            pw.update_layout_for_mode(mode, viewport_w, viewport_h);
            prepended_h += pw.expected_height as f64;
            self.content_box.prepend(&pw.container);
            self.page_widgets.borrow_mut().insert(key.clone(), pw);
        }

        let banner = create_chapter_banner(&archive.filename, total_pages);
        self.content_box.prepend(&banner);

        self.chapters.borrow_mut().insert(
            0,
            ChapterState {
                chapter_id: prev_chap_id,
                archive,
                banner_widget: banner,
            },
        );

        *self.first_loaded_series_idx.borrow_mut() = series_idx;

        if mode == ReadingMode::ContinuousHorizontal {
            self.refit_manga_layout();
        }

        Ok(prepended_h)
    }

    pub fn smooth_scroll_to(&self, dest_y: f64) {
        let vadj = self.scrolled_window.vadjustment();
        let max_scroll = (vadj.upper() - vadj.page_size()).max(0.0);
        let target = dest_y.clamp(0.0, max_scroll);

        *self.target_y.borrow_mut() = Some(target);

        if *self.is_animating.borrow() {
            return;
        }

        *self.is_animating.borrow_mut() = true;

        let target_y = self.target_y.clone();
        let is_animating = self.is_animating.clone();

        let global_to_key = self.global_to_key.clone();
        let page_widgets = self.page_widgets.clone();
        let chapters = self.chapters.clone();
        let in_flight = self.in_flight.clone();
        let tx = self.tx.clone();
        let generation_id = self.generation_id.clone();

        self.scrolled_window.add_tick_callback(move |_, _| {
            let cur = vadj.value();
            let tgt = match *target_y.borrow() {
                Some(t) => t,
                None => {
                    *is_animating.borrow_mut() = false;
                    return glib::ControlFlow::Break;
                }
            };

            let upr = vadj.upper();
            let psz = vadj.page_size();
            let total_global = global_to_key.borrow().len();

            // Continuously request image decodes for live viewport on EVERY VSYNC frame
            if total_global > 0 {
                let current_global_idx = if cur <= 10.0 || upr <= psz {
                    0
                } else {
                    let max_s = (upr - psz).max(1.0);
                    let prog = (cur / max_s).clamp(0.0, 1.0);
                    ((prog * (total_global as f64)) as usize).min(total_global.saturating_sub(1))
                };
                let gen = *generation_id.borrow();
                Self::dispatch_requests_around(
                    current_global_idx,
                    &global_to_key,
                    &page_widgets,
                    &chapters,
                    &in_flight,
                    &tx,
                    gen,
                );
            }

            let diff = tgt - cur;
            if diff.abs() < 0.25 {
                vadj.set_value(tgt);
                *target_y.borrow_mut() = None;
                *is_animating.borrow_mut() = false;
                return glib::ControlFlow::Break;
            }

            let step = diff * 0.26;
            let next = cur + step;
            vadj.set_value(next);
            glib::ControlFlow::Continue
        });
    }

    pub fn smooth_scroll_to_x(&self, dest_x: f64) {
        let hadj = self.scrolled_window.hadjustment();
        let max_scroll = (hadj.upper() - hadj.page_size()).max(0.0);
        let target = dest_x.clamp(0.0, max_scroll);

        *self.target_x.borrow_mut() = Some(target);

        if *self.is_animating.borrow() {
            return;
        }

        *self.is_animating.borrow_mut() = true;

        let target_x = self.target_x.clone();
        let is_animating = self.is_animating.clone();

        let global_to_key = self.global_to_key.clone();
        let page_widgets = self.page_widgets.clone();
        let chapters = self.chapters.clone();
        let in_flight = self.in_flight.clone();
        let tx = self.tx.clone();
        let generation_id = self.generation_id.clone();

        self.scrolled_window.add_tick_callback(move |_, _| {
            let cur = hadj.value();
            let tgt = match *target_x.borrow() {
                Some(t) => t,
                None => {
                    *is_animating.borrow_mut() = false;
                    return glib::ControlFlow::Break;
                }
            };

            let upr = hadj.upper();
            let psz = hadj.page_size();
            let total_global = global_to_key.borrow().len();

            // Continuously request image decodes for live viewport on EVERY VSYNC frame
            if total_global > 0 {
                let current_global_idx = if cur <= 10.0 || upr <= psz {
                    0
                } else {
                    let max_s = (upr - psz).max(1.0);
                    let prog = (cur / max_s).clamp(0.0, 1.0);
                    ((prog * (total_global as f64)) as usize).min(total_global.saturating_sub(1))
                };
                let gen = *generation_id.borrow();
                Self::dispatch_requests_around(
                    current_global_idx,
                    &global_to_key,
                    &page_widgets,
                    &chapters,
                    &in_flight,
                    &tx,
                    gen,
                );
            }

            let diff = tgt - cur;
            if diff.abs() < 0.25 {
                hadj.set_value(tgt);
                *target_x.borrow_mut() = None;
                *is_animating.borrow_mut() = false;
                return glib::ControlFlow::Break;
            }

            let step = diff * 0.26;
            let next = cur + step;
            hadj.set_value(next);
            glib::ControlFlow::Continue
        });
    }

    pub fn get_current_global_page_idx(&self) -> usize {
        let global_to_key = self.global_to_key.borrow();
        let total = global_to_key.len();
        if total == 0 {
            return 0;
        }

        let mode = *self.reading_mode.borrow();
        match mode {
            ReadingMode::ContinuousVertical => {
                let vadj = self.scrolled_window.vadjustment();
                let val = vadj.value();
                let upr = vadj.upper();
                let psz = vadj.page_size();
                if val <= 10.0 || upr <= psz {
                    0
                } else {
                    let max_s = (upr - psz).max(1.0);
                    let prog = (val / max_s).clamp(0.0, 1.0);
                    ((prog * (total as f64)) as usize).min(total.saturating_sub(1))
                }
            }
            ReadingMode::ContinuousHorizontal => {
                let hadj = self.scrolled_window.hadjustment();
                let val = hadj.value();
                let psz = hadj.page_size();
                let upr = hadj.upper();
                let center_x = val + psz / 2.0;

                let page_widgets = self.page_widgets.borrow();
                let mut closest_idx = None;
                let mut min_dist = f64::MAX;

                for (idx, key) in global_to_key.iter().enumerate() {
                    if let Some(pw) = page_widgets.get(key) {
                        if let Some(rect) = pw.container.compute_bounds(&self.clamp) {
                            let pw_center = rect.x() as f64 + rect.width() as f64 / 2.0;
                            let dist = (pw_center - center_x).abs();
                            if dist < min_dist {
                                min_dist = dist;
                                closest_idx = Some(idx);
                            }
                        }
                    }
                }

                if let Some(idx) = closest_idx {
                    idx
                } else if val <= 10.0 || upr <= psz {
                    0
                } else {
                    let max_scroll = (upr - psz).max(1.0);
                    let progress = (val / max_scroll).clamp(0.0, 1.0);
                    ((progress * (total as f64)) as usize).min(total.saturating_sub(1))
                }
            }
        }
    }

    /// Jumps to a 1-based page once the layout is available. `page 0/1` maps
    /// to the top (already the default), so this is a no-op for them.
    /// Clamped to the initially opened chapter's page count.
    pub fn jump_to_page(&self, page: usize) {
        let initial_page_count = self
            .chapters
            .borrow()
            .first()
            .map(|c| c.archive.page_count())
            .unwrap_or_else(|| self.global_to_key.borrow().len());

        if initial_page_count == 0 || page <= 1 {
            return;
        }

        let clamped = page.min(initial_page_count);
        let idx = (clamped - 1).min(initial_page_count.saturating_sub(1));

        *self.focused_page_idx.borrow_mut() = idx;
        self.request_pages_around(idx);

        // If jumping to near or at the end of the initial chapter, also pre-load the next chapter
        if clamped >= initial_page_count.saturating_sub(1) {
            let last_idx = *self.last_loaded_series_idx.borrow();
            if let Some(ref series) = *self.series.borrow() {
                if last_idx + 1 < series.dir_files.len() {
                    let next_idx = last_idx + 1;
                    if let Ok(archive) = CbzArchive::open(&series.dir_files[next_idx]) {
                        let _ = self.append_chapter(archive, next_idx);
                    }
                }
            }
        }

        let reader = self.clone();
        let attempts = Rc::new(Cell::new(0u8));
        glib::idle_add_local(move || {
            if reader.try_jump_to_idx(idx) || attempts.get() >= 120 {
                glib::ControlFlow::Break
            } else {
                attempts.set(attempts.get() + 1);
                glib::ControlFlow::Continue
            }
        });
    }

    /// One attempt at scrolling to the given global page index. Returns true
    /// once widget bounds are computable (i.e. layout has happened).
    fn try_jump_to_idx(&self, idx: usize) -> bool {
        let key = match self.global_to_key.borrow().get(idx) {
            Some(k) => k.clone(),
            None => return true,
        };
        let pw = match self.page_widgets.borrow().get(&key) {
            Some(p) => p.clone(),
            None => return true,
        };

        match *self.reading_mode.borrow() {
            ReadingMode::ContinuousVertical => {
                if let Some(rect) = pw.container.compute_bounds(&self.clamp) {
                    let vadj = self.scrolled_window.vadjustment();
                    vadj.set_value(rect.y() as f64);
                    self.request_pages_around(idx);
                    true
                } else {
                    false
                }
            }
            ReadingMode::ContinuousHorizontal => {
                if let Some(rect) = pw.container.compute_bounds(&self.clamp) {
                    *self.focused_page_idx.borrow_mut() = idx;
                    let hadj = self.scrolled_window.hadjustment();
                    let target_x =
                        rect.x() as f64 + rect.width() as f64 / 2.0 - hadj.page_size() / 2.0;
                    hadj.set_value(target_x.max(0.0));
                    self.update_focus_styles(idx);
                    self.request_pages_around(idx);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Page under the viewport center, computed from real widget bounds
    /// (exact per-page geometry), unlike the proportional estimate used by
    /// `get_current_global_page_idx` for decode dispatch.
    fn global_page_at_viewport_center(&self) -> usize {
        let global_to_key = self.global_to_key.borrow();
        let total = global_to_key.len();
        if total == 0 {
            return 0;
        }
        let widgets = self.page_widgets.borrow();
        match *self.reading_mode.borrow() {
            ReadingMode::ContinuousVertical => {
                let vadj = self.scrolled_window.vadjustment();
                let center = vadj.value() + vadj.page_size() / 2.0;
                let mut closest = 0usize;
                let mut best = f64::MAX;
                for (i, key) in global_to_key.iter().enumerate() {
                    if let Some(pw) = widgets.get(key) {
                        if let Some(rect) = pw.container.compute_bounds(&self.clamp) {
                            let top = rect.y() as f64;
                            let bottom = top + rect.height() as f64;
                            if center >= top && center < bottom {
                                return i;
                            }
                            let dist = (center - (top + rect.height() as f64 / 2.0)).abs();
                            if dist < best {
                                best = dist;
                                closest = i;
                            }
                        }
                    }
                }
                closest
            }
            ReadingMode::ContinuousHorizontal => {
                let hadj = self.scrolled_window.hadjustment();
                let center_x = hadj.value() + hadj.page_size() / 2.0;
                let mut closest = 0usize;
                let mut best = f64::MAX;
                for (i, key) in global_to_key.iter().enumerate() {
                    if let Some(pw) = widgets.get(key) {
                        if let Some(rect) = pw.container.compute_bounds(&self.clamp) {
                            let left = rect.x() as f64;
                            let right = left + rect.width() as f64;
                            if center_x >= left && center_x < right {
                                return i;
                            }
                            let dist = (center_x - (left + rect.width() as f64 / 2.0)).abs();
                            if dist < best {
                                best = dist;
                                closest = i;
                            }
                        }
                    }
                }
                closest
            }
        }
    }

    /// dewey integration contract: when a file was loaded, report the current
    /// (1-based) page and whether the reader reached the last page so the
    /// caller can persist progress / completion.
    pub fn exit_payload(&self) -> Option<ExitPayload> {
        let chapters = self.chapters.borrow();
        if chapters.is_empty() {
            return None;
        }

        let mut entries: Vec<ChapterProgressEntry> = Vec::new();
        for chap in chapters.iter() {
            let Some(reached) = self.chapter_last_page_reached(chap.chapter_id) else {
                // Chapter auto-appended but never scrolled into view: no progress.
                continue;
            };
            let (last_page, completed) =
                Self::progress_from_reached(reached, chap.archive.page_count());
            entries.push(ChapterProgressEntry {
                file: chap.archive.path.to_string_lossy().to_string(),
                last_page,
                completed,
            });
        }

        if entries.is_empty() {
            return None;
        }

        let first = &entries[0];
        let current_mode = self.reading_mode.borrow().as_str().to_string();
        Some(ExitPayload {
            last_page: first.last_page,
            completed: first.completed,
            mode: current_mode,
            chapters: entries,
        })
    }

    /// Largest page index of `chapter_id` whose page has scrolled into view
    /// (page top/left at or above the viewport bottom/right edge). None when no
    /// page of that chapter has been reached or bounds aren't laid out yet.
    fn chapter_last_page_reached(&self, chapter_id: usize) -> Option<usize> {
        let vadj = self.scrolled_window.vadjustment();
        let hadj = self.scrolled_window.hadjustment();
        let viewport_bottom = vadj.value() + vadj.page_size();
        let viewport_right = hadj.value() + hadj.page_size();
        let vertical = matches!(*self.reading_mode.borrow(), ReadingMode::ContinuousVertical);

        let global_to_key = self.global_to_key.borrow();
        let widgets = self.page_widgets.borrow();
        let mut reached: Option<usize> = None;
        for key in global_to_key.iter() {
            if key.chapter_idx != chapter_id {
                continue;
            }
            let Some(pw) = widgets.get(key) else { continue };
            let Some(rect) = pw.container.compute_bounds(&self.clamp) else {
                continue;
            };
            if vertical {
                if rect.y() as f64 <= viewport_bottom {
                    reached = Some(key.page_idx.max(reached.unwrap_or(0)));
                }
            } else if rect.x() as f64 <= viewport_right {
                reached = Some(key.page_idx.max(reached.unwrap_or(0)));
            }
        }
        reached
    }

    /// Pure mapping from the zero-based last page reached to the 1-based
    /// `last_page` and a `completed` flag scoped to that chapter's page count.
    fn progress_from_reached(reached: usize, total_pages: usize) -> (i64, bool) {
        let last_page = reached.saturating_add(1) as i64;
        let completed = total_pages > 0 && reached >= total_pages.saturating_sub(1);
        (last_page, completed)
    }

    pub fn get_viewport_height(&self) -> f64 {
        let h = self.scrolled_window.height() as f64;
        if h > 50.0 {
            h
        } else {
            let v_psz = self.scrolled_window.vadjustment().page_size();
            if v_psz > 50.0 {
                v_psz
            } else {
                900.0
            }
        }
    }

    pub fn get_viewport_width(&self) -> f64 {
        let w = self.scrolled_window.width() as f64;
        if w > 50.0 {
            w
        } else {
            let h_psz = self.scrolled_window.hadjustment().page_size();
            if h_psz > 50.0 {
                h_psz
            } else {
                1200.0
            }
        }
    }

    pub fn get_active_aspect_ratio(&self) -> Option<f64> {
        let global_to_key = self.global_to_key.borrow();
        if global_to_key.is_empty() {
            return None;
        }
        let cur_idx = self.get_current_global_page_idx();
        let key = global_to_key
            .get(cur_idx)
            .or_else(|| global_to_key.first())?;
        let page_widgets = self.page_widgets.borrow();
        let pw = page_widgets.get(key)?;
        let (w, h) = pw.get_dimensions();
        if w > 0 && h > 0 {
            Some(h as f64 / w as f64)
        } else {
            None
        }
    }

    pub fn refit_manga_layout(&self) {
        if *self.reading_mode.borrow() != ReadingMode::ContinuousHorizontal {
            return;
        }

        let viewport_h = self.get_viewport_height();
        let viewport_w = self.get_viewport_width();
        let widgets = self.page_widgets.borrow();
        for pw in widgets.values() {
            pw.update_layout_for_mode(ReadingMode::ContinuousHorizontal, viewport_w, viewport_h);
        }

        if viewport_w > 0.0 {
            let margin = (viewport_w / 2.0).ceil() as i32;
            self.content_box.set_margin_start(margin);
            self.content_box.set_margin_end(margin);
        }
        let cur_idx = *self.focused_page_idx.borrow();
        self.update_focus_styles(cur_idx);
    }

    /// Keeps the current focused page precisely centered in the viewport during window resize
    pub fn center_focused_page(&self) {
        if *self.reading_mode.borrow() != ReadingMode::ContinuousHorizontal {
            return;
        }

        let cur_idx = *self.focused_page_idx.borrow();
        let global_to_key = self.global_to_key.borrow();
        if let Some(key) = global_to_key.get(cur_idx) {
            let page_widgets = self.page_widgets.borrow();
            if let Some(pw) = page_widgets.get(key) {
                if let Some(rect) = pw.container.compute_bounds(&self.clamp) {
                    let page_x = rect.x() as f64;
                    let page_w = rect.width() as f64;
                    let viewport_w = self.scrolled_window.hadjustment().page_size();
                    if viewport_w > 0.0 {
                        let target_x = (page_x - (viewport_w - page_w) / 2.0).max(0.0);
                        self.scrolled_window.hadjustment().set_value(target_x);
                    }
                }
            }
        }
        self.update_focus_styles(cur_idx);
    }

    pub fn update_focus_styles(&self, focused_idx: usize) {
        if *self.reading_mode.borrow() != ReadingMode::ContinuousHorizontal {
            return;
        }

        let global_to_key = self.global_to_key.borrow();
        let widgets = self.page_widgets.borrow();

        for (idx, key) in global_to_key.iter().enumerate() {
            if let Some(pw) = widgets.get(key) {
                if idx == focused_idx {
                    if !pw.container.has_css_class("manga-page-focused") {
                        pw.container.remove_css_class("manga-page-side");
                        pw.container.add_css_class("manga-page-focused");
                    }
                } else if !pw.container.has_css_class("manga-page-side") {
                    pw.container.remove_css_class("manga-page-focused");
                    pw.container.add_css_class("manga-page-side");
                }
            }
        }
    }

    pub fn toggle_reading_mode(&self) {
        let current = *self.reading_mode.borrow();
        let new_mode = match current {
            ReadingMode::ContinuousVertical => ReadingMode::ContinuousHorizontal,
            ReadingMode::ContinuousHorizontal => ReadingMode::ContinuousVertical,
        };
        self.set_reading_mode(new_mode);
    }

    pub fn set_reading_mode(&self, mode: ReadingMode) {
        *self.reading_mode.borrow_mut() = mode;
        let curr_page_idx = self.get_current_global_page_idx();
        let viewport_h = self.get_viewport_height();
        let viewport_w = self.get_viewport_width();

        match mode {
            ReadingMode::ContinuousVertical => {
                self.content_box.set_orientation(Orientation::Vertical);
                self.content_box.set_vexpand(false);
                self.content_box.set_valign(Align::Fill);
                self.content_box.set_spacing(0);
                self.content_box.set_margin_start(0);
                self.content_box.set_margin_end(0);
                self.scrolled_window
                    .set_hscrollbar_policy(gtk4::PolicyType::Never);
                self.scrolled_window
                    .set_vscrollbar_policy(gtk4::PolicyType::Automatic);
                self.clamp.set_maximum_size(900);
                self.clamp.set_tightening_threshold(700);
                self.clamp.set_vexpand(false);
                self.clamp.set_valign(Align::Fill);
                self.scrolled_window.remove_css_class("manga-fade-overlay");

                let page_widgets = self.page_widgets.borrow();
                for pw in page_widgets.values() {
                    pw.container.remove_css_class("manga-page-focused");
                    pw.container.remove_css_class("manga-page-side");
                    pw.update_layout_for_mode(
                        ReadingMode::ContinuousVertical,
                        viewport_w,
                        viewport_h,
                    );
                }
            }
            ReadingMode::ContinuousHorizontal => {
                self.content_box.set_orientation(Orientation::Horizontal);
                self.content_box.set_vexpand(true);
                self.content_box.set_valign(Align::Fill);
                self.content_box.set_spacing(20);
                self.scrolled_window
                    .set_hscrollbar_policy(gtk4::PolicyType::Automatic);
                self.scrolled_window
                    .set_vscrollbar_policy(gtk4::PolicyType::Never);
                self.clamp.set_maximum_size(i32::MAX);
                self.clamp.set_tightening_threshold(i32::MAX);
                self.clamp.set_vexpand(true);
                self.clamp.set_valign(Align::Fill);
                self.scrolled_window.add_css_class("manga-fade-overlay");

                self.refit_manga_layout();
                self.update_focus_styles(curr_page_idx);
            }
        }

        let self_clone = self.clone();
        glib::idle_add_local(move || {
            self_clone.smooth_scroll_to_page(curr_page_idx);
            glib::ControlFlow::Break
        });
    }

    pub fn smooth_scroll_to_page(&self, global_idx: usize) {
        let global_to_key = self.global_to_key.borrow();
        if global_idx >= global_to_key.len() {
            return;
        }

        *self.focused_page_idx.borrow_mut() = global_idx;

        let key = &global_to_key[global_idx];
        let page_widgets = self.page_widgets.borrow();
        if let Some(pw) = page_widgets.get(key) {
            let mode = *self.reading_mode.borrow();
            match mode {
                ReadingMode::ContinuousVertical => {
                    if let Some(rect) = pw.container.compute_bounds(&self.clamp) {
                        self.smooth_scroll_to(rect.y() as f64);
                    }
                }
                ReadingMode::ContinuousHorizontal => {
                    if let Some(rect) = pw.container.compute_bounds(&self.clamp) {
                        let page_x = rect.x() as f64;
                        let page_w = rect.width() as f64;
                        let viewport_w = self.scrolled_window.hadjustment().page_size();
                        let target_x = page_x - (viewport_w - page_w) / 2.0;
                        self.smooth_scroll_to_x(target_x);
                        self.update_focus_styles(global_idx);
                    }
                }
            }
        }
    }

    pub fn next_page(&self) {
        let mode = *self.reading_mode.borrow();
        if mode == ReadingMode::ContinuousHorizontal {
            let curr_idx = self.get_current_global_page_idx();
            let total = self.global_to_key.borrow().len();
            if curr_idx + 1 < total {
                self.smooth_scroll_to_page(curr_idx + 1);
            } else {
                let last_idx = *self.last_loaded_series_idx.borrow();
                if let Some(ref series) = *self.series.borrow() {
                    if last_idx + 1 < series.dir_files.len() {
                        let next_idx = last_idx + 1;
                        let next_path = &series.dir_files[next_idx];
                        if let Ok(archive) = CbzArchive::open(next_path) {
                            if self.append_chapter(archive, next_idx).is_ok() {
                                self.smooth_scroll_to_page(curr_idx + 1);
                            }
                        }
                    }
                }
            }
        } else {
            self.next_chapter();
        }
    }

    pub fn prev_page(&self) {
        let mode = *self.reading_mode.borrow();
        if mode == ReadingMode::ContinuousHorizontal {
            let curr_idx = self.get_current_global_page_idx();
            if curr_idx > 0 {
                self.smooth_scroll_to_page(curr_idx - 1);
            } else {
                let first_idx = *self.first_loaded_series_idx.borrow();
                if first_idx > 0 {
                    if let Some(ref series) = *self.series.borrow() {
                        let prev_idx = first_idx - 1;
                        let prev_path = &series.dir_files[prev_idx];
                        if let Ok(archive) = CbzArchive::open(prev_path) {
                            if self.prepend_chapter(archive, prev_idx).is_ok() {
                                self.smooth_scroll_to_page(0);
                            }
                        }
                    }
                }
            }
        } else {
            self.prev_chapter();
        }
    }

    pub fn smooth_scroll_by(&self, delta_y: f64) {
        let vadj = self.scrolled_window.vadjustment();
        let current_val = vadj.value();

        // If at top of comic and user scrolls UP (delta_y < 0), load previous chapter
        if delta_y < 0.0 && current_val <= 10.0 {
            let first_idx = *self.first_loaded_series_idx.borrow();
            if first_idx > 0 {
                self.prev_chapter();
                return;
            }
        }

        // If at bottom of comic and user scrolls DOWN (delta_y > 0), load next chapter
        let max_scroll = (vadj.upper() - vadj.page_size()).max(0.0);
        if delta_y > 0.0 && current_val >= max_scroll - 10.0 {
            self.next_chapter();
            return;
        }

        let base_y = self.target_y.borrow().unwrap_or(current_val);
        let dest_y = base_y + delta_y;
        self.smooth_scroll_to(dest_y);
    }

    pub fn jump_to_current_chapter_top(&self) {
        let chapters = self.chapters.borrow();
        if chapters.is_empty() {
            return;
        }

        let current_y = self
            .target_y
            .borrow()
            .unwrap_or_else(|| self.scrolled_window.vadjustment().value());
        let mut current_chap_idx = 0;
        let mut current_chap_y = 0.0;
        for (i, chap) in chapters.iter().enumerate() {
            if let Some(rect) = chap.banner_widget.compute_bounds(&self.clamp) {
                let y = rect.y() as f64;
                if y <= current_y + 150.0 {
                    current_chap_idx = i;
                    current_chap_y = y;
                }
            }
        }

        if (current_y - current_chap_y).abs() < 50.0 {
            if current_chap_idx > 0 {
                if let Some(rect) = chapters[current_chap_idx - 1]
                    .banner_widget
                    .compute_bounds(&self.clamp)
                {
                    let target_y = rect.y() as f64;
                    self.smooth_scroll_to(target_y);
                    return;
                }
            } else {
                self.smooth_scroll_to(0.0);
                return;
            }
        }

        self.smooth_scroll_to(current_chap_y);
    }

    /// G: jump to the bottom of the chapter currently under the viewport.
    /// Unlike `smooth_scroll_to(f64::MAX)` this does NOT trigger the
    /// near-bottom auto-append chain, so it cannot walk the whole series.
    pub fn jump_to_current_chapter_bottom(&self) {
        let global_to_key = self.global_to_key.borrow();
        if global_to_key.is_empty() {
            return;
        }
        let cur_idx = self.global_page_at_viewport_center();
        let chapter_id = global_to_key[cur_idx].chapter_idx;

        // Last global index belonging to the current chapter.
        let last_idx = match global_to_key
            .iter()
            .rposition(|k| k.chapter_idx == chapter_id)
        {
            Some(i) => i,
            None => return,
        };

        match *self.reading_mode.borrow() {
            ReadingMode::ContinuousVertical => {
                let page_widgets = self.page_widgets.borrow();
                if let Some(pw) = page_widgets.get(&global_to_key[last_idx]) {
                    if let Some(rect) = pw.container.compute_bounds(&self.clamp) {
                        let viewport = self.scrolled_window.vadjustment().page_size();
                        let target = (rect.y() as f64 + rect.height() as f64 - viewport).max(0.0);
                        self.smooth_scroll_to(target);
                    }
                }
            }
            ReadingMode::ContinuousHorizontal => {
                self.smooth_scroll_to_page(last_idx);
            }
        }
    }

    pub fn next_chapter(&self) {
        let chapters = self.chapters.borrow();
        if chapters.is_empty() {
            return;
        }

        let current_y = self
            .target_y
            .borrow()
            .unwrap_or_else(|| self.scrolled_window.vadjustment().value());
        let mut current_chap_idx = 0;
        for (i, chap) in chapters.iter().enumerate() {
            if let Some(rect) = chap.banner_widget.compute_bounds(&self.clamp) {
                let y = rect.y() as f64;
                if y <= current_y + 150.0 {
                    current_chap_idx = i;
                }
            }
        }

        let target_chap_idx = current_chap_idx + 1;
        if target_chap_idx < chapters.len() {
            if let Some(rect) = chapters[target_chap_idx]
                .banner_widget
                .compute_bounds(&self.clamp)
            {
                let target_y = rect.y() as f64;
                self.smooth_scroll_to(target_y);
            }
        } else {
            drop(chapters);
            let last_idx = *self.last_loaded_series_idx.borrow();
            if let Some(ref series) = *self.series.borrow() {
                if last_idx + 1 < series.dir_files.len() {
                    let next_idx = last_idx + 1;
                    let next_path = &series.dir_files[next_idx];
                    if let Ok(archive) = CbzArchive::open(next_path) {
                        if self.append_chapter(archive, next_idx).is_ok() {
                            let chapters = self.chapters.borrow();
                            if let Some(last_chap) = chapters.last() {
                                if let Some(rect) =
                                    last_chap.banner_widget.compute_bounds(&self.clamp)
                                {
                                    let target_y = rect.y() as f64;
                                    self.smooth_scroll_to(target_y);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn prev_chapter(&self) {
        let chapters = self.chapters.borrow();
        if chapters.is_empty() {
            return;
        }

        let current_y = self
            .target_y
            .borrow()
            .unwrap_or_else(|| self.scrolled_window.vadjustment().value());
        let mut current_chap_idx = 0;
        let mut current_chap_y = 0.0;
        for (i, chap) in chapters.iter().enumerate() {
            if let Some(rect) = chap.banner_widget.compute_bounds(&self.clamp) {
                let y = rect.y() as f64;
                if y <= current_y + 150.0 {
                    current_chap_idx = i;
                    current_chap_y = y;
                }
            }
        }

        if current_y > current_chap_y + 150.0 {
            self.smooth_scroll_to(current_chap_y);
            return;
        }

        if current_chap_idx > 0 {
            let prev_chap_idx = current_chap_idx - 1;
            if let Some(rect) = chapters[prev_chap_idx]
                .banner_widget
                .compute_bounds(&self.clamp)
            {
                let target_y = rect.y() as f64;
                self.smooth_scroll_to(target_y);
            }
        } else {
            drop(chapters);
            let first_idx = *self.first_loaded_series_idx.borrow();
            if first_idx > 0 {
                if let Some(ref series) = *self.series.borrow() {
                    let prev_idx = first_idx - 1;
                    let prev_path = &series.dir_files[prev_idx];
                    let _ = self.load_initial_file(prev_path.clone());
                }
            } else {
                self.smooth_scroll_to(0.0);
            }
        }
    }

    fn setup_scroll_listener(&self) {
        let vadjustment = self.scrolled_window.vadjustment();
        let page_widgets = self.page_widgets.clone();
        let chapters = self.chapters.clone();
        let global_to_key = self.global_to_key.clone();
        let memory_manager = self.memory_manager.clone();

        let series_clone = self.series.clone();
        let last_loaded_idx = self.last_loaded_series_idx.clone();
        let last_vadj_value = self.last_vadj_value.clone();
        let in_flight = self.in_flight.clone();

        let rx = self.rx.clone();
        let tx = self.tx.clone();
        let reader_c = self.clone();

        // Continuously process completed background image decodes at 60FPS on GTK main loop
        let page_widgets_rx = page_widgets.clone();
        let global_to_key_rx = global_to_key.clone();
        let memory_manager_rx = memory_manager;
        let in_flight_rx = in_flight.clone();
        let vadj_rx = vadjustment.clone();
        let generation_id_rx = self.generation_id.clone();

        glib::timeout_add_local(Duration::from_millis(16), move || {
            let mut uploads_this_tick = 0;
            const MAX_UPLOADS_PER_TICK: usize = 3;

            while let Ok((key, gen, maybe_payload)) = rx.try_recv() {
                in_flight_rx.borrow_mut().remove(&key);

                // Discard stale background worker results from previous generations/comics
                if gen != *generation_id_rx.borrow() {
                    continue;
                }

                if let Some(payload) = maybe_payload {
                    match CbzArchive::create_texture(
                        payload.width,
                        payload.height,
                        payload.rgba_bytes,
                    ) {
                        Ok(data) => {
                            if let Some(pw) = page_widgets_rx.borrow().get(&key) {
                                pw.set_loaded(&data.texture, data.width, data.height);

                                let g2k_map = global_to_key_rx.borrow();
                                let page_to_global =
                                    |k: &PageKey| g2k_map.iter().position(|x| x == k).unwrap_or(0);

                                // Compute live viewport global index right now accurately for reading mode
                                let live_global_idx = {
                                    let mode = *reader_c.reading_mode.borrow();
                                    match mode {
                                        ReadingMode::ContinuousVertical => {
                                            let val = vadj_rx.value();
                                            let upr = vadj_rx.upper();
                                            let psz = vadj_rx.page_size();
                                            let total_g = g2k_map.len();

                                            if val <= 10.0 || upr <= psz || total_g == 0 {
                                                0
                                            } else {
                                                let max_s = (upr - psz).max(1.0);
                                                let prog = (val / max_s).clamp(0.0, 1.0);
                                                ((prog * (total_g as f64)) as usize)
                                                    .min(total_g.saturating_sub(1))
                                            }
                                        }
                                        ReadingMode::ContinuousHorizontal => {
                                            reader_c.get_current_global_page_idx()
                                        }
                                    }
                                };

                                let evicted = memory_manager_rx.borrow_mut().insert(
                                    key.clone(),
                                    data,
                                    live_global_idx,
                                    &page_to_global,
                                );

                                for ev_key in evicted {
                                    if let Some(ev_pw) = page_widgets_rx.borrow().get(&ev_key) {
                                        ev_pw.set_unloaded();
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[ERROR] Failed to create texture for {:?}: {}", key, e);
                            if let Some(pw) = page_widgets_rx.borrow().get(&key) {
                                pw.set_unloaded();
                            }
                        }
                    }
                } else if let Some(pw) = page_widgets_rx.borrow().get(&key) {
                    pw.set_unloaded();
                }

                uploads_this_tick += 1;
                if uploads_this_tick >= MAX_UPLOADS_PER_TICK {
                    break;
                }
            }
            glib::ControlFlow::Continue
        });

        let reader_c2 = self.clone();
        vadjustment.connect_value_changed(move |adj| {
            if *reader_c2.reading_mode.borrow() != ReadingMode::ContinuousVertical {
                return;
            }
            let value = adj.value();
            let page_size = adj.page_size();
            let upper = adj.upper();
            *last_vadj_value.borrow_mut() = value;

            // Check if near bottom to auto load next .cbz file (downward scrolling)!
            let last_idx = *last_loaded_idx.borrow();
            let lookahead = if reader_c2.is_usb_mode() {
                2500.0
            } else {
                1500.0
            };
            if upper > 0.0 && (value + page_size >= upper - lookahead) {
                if let Some(ref series) = *series_clone.borrow() {
                    if last_idx + 1 < series.dir_files.len() {
                        let next_idx = last_idx + 1;
                        if let Ok(archive) = CbzArchive::open(&series.dir_files[next_idx]) {
                            let _ = reader_c2.append_chapter(archive, next_idx);
                        }
                    }
                }
            }

            let total_global = global_to_key.borrow().len();
            if total_global == 0 {
                return;
            }

            // Calculate current global page directly from scroll progress
            let current_global_idx = if value <= 10.0 || upper <= page_size {
                0
            } else {
                let max_scroll = (upper - page_size).max(1.0);
                let progress = (value / max_scroll).clamp(0.0, 1.0);
                ((progress * (total_global as f64)) as usize).min(total_global.saturating_sub(1))
            };

            let gen = *reader_c2.generation_id.borrow();
            Self::dispatch_requests_around(
                current_global_idx,
                &global_to_key,
                &page_widgets,
                &chapters,
                &in_flight,
                &tx,
                gen,
            );
        });

        let hadjustment = self.scrolled_window.hadjustment();
        let reader_h = self.clone();
        let global_to_key_h = self.global_to_key.clone();
        let page_widgets_h = self.page_widgets.clone();
        let chapters_h = self.chapters.clone();
        let in_flight_h = self.in_flight.clone();
        let tx_h = self.tx.clone();
        let generation_id_h = self.generation_id.clone();

        hadjustment.connect_value_changed(move |_adj| {
            if *reader_h.reading_mode.borrow() != ReadingMode::ContinuousHorizontal {
                return;
            }

            let total_global = global_to_key_h.borrow().len();
            if total_global == 0 {
                return;
            }

            let current_global_idx = reader_h.get_current_global_page_idx();
            reader_h.update_focus_styles(current_global_idx);

            let gen = *generation_id_h.borrow();
            Self::dispatch_requests_around(
                current_global_idx,
                &global_to_key_h,
                &page_widgets_h,
                &chapters_h,
                &in_flight_h,
                &tx_h,
                gen,
            );
        });
    }

    pub fn request_pages_around(&self, current_global_idx: usize) {
        let gen = *self.generation_id.borrow();
        Self::dispatch_requests_around(
            current_global_idx,
            &self.global_to_key,
            &self.page_widgets,
            &self.chapters,
            &self.in_flight,
            &self.tx,
            gen,
        );
    }

    fn dispatch_requests_around(
        current_global_idx: usize,
        global_to_key: &Rc<RefCell<Vec<PageKey>>>,
        page_widgets: &Rc<RefCell<HashMap<PageKey, PageWidget>>>,
        chapters: &Rc<RefCell<Vec<ChapterState>>>,
        in_flight: &Rc<RefCell<HashSet<PageKey>>>,
        tx: &Sender<(PageKey, usize, Option<DecodedImagePayload>)>,
        generation_id: usize,
    ) {
        let total_global = global_to_key.borrow().len();
        if total_global == 0 {
            return;
        }

        let start_idx = current_global_idx.saturating_sub(4);
        let end_idx = (current_global_idx + 16).min(total_global);

        let mut priority_keys: Vec<(usize, PageKey)> = {
            let g2k = global_to_key.borrow();
            (start_idx..end_idx)
                .filter_map(|i| g2k.get(i).map(|k| (i, k.clone())))
                .collect()
        };

        priority_keys.sort_by_key(|(idx, _)| idx.abs_diff(current_global_idx));

        const MAX_CONCURRENT_IN_FLIGHT: usize = 8;

        for (_g_idx, key) in priority_keys {
            if in_flight.borrow().len() >= MAX_CONCURRENT_IN_FLIGHT {
                break;
            }

            let is_loaded = page_widgets
                .borrow()
                .get(&key)
                .is_some_and(|w| *w.is_loaded.borrow());

            let is_in_flight = in_flight.borrow().contains(&key);

            if !is_loaded && !is_in_flight {
                if let Some(pw) = page_widgets.borrow().get(&key) {
                    pw.set_loading();
                }

                in_flight.borrow_mut().insert(key.clone());

                let archive_info = {
                    let chaps = chapters.borrow();
                    chaps
                        .iter()
                        .find(|ch| ch.chapter_id == key.chapter_idx)
                        .and_then(|ch| {
                            let data = ch.archive.data.clone();
                            ch.archive
                                .image_entries
                                .get(key.page_idx)
                                .cloned()
                                .map(|entry| (data, entry))
                        })
                };

                if let Some((archive_data, entry_name)) = archive_info {
                    let tx_c = tx.clone();
                    let key_c = key.clone();
                    let gen_c = generation_id;

                    rayon::spawn(move || {
                        let res = (|| -> Result<(u32, u32, Vec<u8>), String> {
                            let buf = CbzArchive::extract_entry_bytes(&archive_data, &entry_name)?;
                            CbzArchive::decode_page_bytes(&buf)
                        })();

                        let payload = match res {
                            Ok((width, height, rgba_bytes)) => {
                                eprintln!(
                                    "[decode] {:?} ok: {}x{} ({} bytes)",
                                    key_c,
                                    width,
                                    height,
                                    rgba_bytes.len()
                                );
                                Some(DecodedImagePayload {
                                    width,
                                    height,
                                    rgba_bytes,
                                })
                            }
                            Err(e) => {
                                eprintln!("[ERROR] Failed to decode {:?}: {}", key_c, e);
                                None
                            }
                        };

                        let _ = tx_c.send((key_c, gen_c, payload));
                    });
                } else {
                    in_flight.borrow_mut().remove(&key);
                }
            }
        }
    }

    pub fn clear(&self) {
        // Reset scroll before clearing to prevent position drift
        let vadj = self.scrolled_window.vadjustment();
        vadj.set_value(0.0);
        let hadj = self.scrolled_window.hadjustment();
        hadj.set_value(0.0);

        while let Some(child) = self.content_box.first_child() {
            self.content_box.remove(&child);
        }

        self.series.borrow_mut().take();
        self.chapters.borrow_mut().clear();
        self.page_widgets.borrow_mut().clear();
        self.global_to_key.borrow_mut().clear();
        self.memory_manager.borrow_mut().clear();
        self.in_flight.borrow_mut().clear();
        *self.generation_id.borrow_mut() += 1;
        *self.first_loaded_series_idx.borrow_mut() = 0;
        *self.last_loaded_series_idx.borrow_mut() = 0;
        *self.next_chapter_id.borrow_mut() = 0;
        *self.last_vadj_value.borrow_mut() = 0.0;
        *self.target_y.borrow_mut() = None;
        *self.target_x.borrow_mut() = None;
        *self.focused_page_idx.borrow_mut() = 0;
        *self.is_animating.borrow_mut() = false;

        self.content_box.set_visible(false);
        self.status_page.set_visible(true);
    }
}

#[cfg(test)]
mod tests {
    use super::ReaderView;

    #[test]
    fn progress_from_reached_is_chapter_local() {
        // 15-page chapter: page 15 reached (idx 14) -> last page, completed.
        assert_eq!(ReaderView::progress_from_reached(14, 15), (15, true));
        // Mid-chapter.
        assert_eq!(ReaderView::progress_from_reached(5, 15), (6, false));
        // First page.
        assert_eq!(ReaderView::progress_from_reached(0, 15), (1, false));
        // Total page count for "completed" uses this chapter's own count.
        assert_eq!(ReaderView::progress_from_reached(14, 15), (15, true));
        assert_eq!(ReaderView::progress_from_reached(3, 4), (4, true));
    }
}
