use crate::cache::PageKey;
use gdk4::Texture;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, ContentFit, Label, Orientation, Picture, Spinner};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingMode {
    ContinuousVertical,
    ContinuousHorizontal,
}

impl ReadingMode {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "manga" | "horizontal" | "paged" => ReadingMode::ContinuousHorizontal,
            _ => ReadingMode::ContinuousVertical,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ReadingMode::ContinuousVertical => "webtoon",
            ReadingMode::ContinuousHorizontal => "manga",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct PageWidget {
    pub key: PageKey,
    pub container: GtkBox,
    pub picture: Picture,
    pub placeholder: GtkBox,
    pub spinner: Spinner,
    pub page_label: Label,
    pub is_loaded: Rc<RefCell<bool>>,
    pub is_loading: Rc<RefCell<bool>>,
    pub expected_height: i32,
    pub orig_width: Cell<u32>,
    pub orig_height: Cell<u32>,
    pub manga_width: Rc<RefCell<i32>>,
    pub last_viewport_w: Cell<f64>,
    pub last_viewport_h: Cell<f64>,
}

impl PageWidget {
    pub fn new(key: PageKey, page_num: usize, total_pages: usize, width: u32, height: u32) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        container.add_css_class("page-container");

        let calc_height = if width > 0 && height > 0 {
            let aspect = height as f64 / width as f64;
            ((700.0 * aspect) as i32).max(200)
        } else {
            1000 // Default height
        };

        container.set_height_request(calc_height);

        let placeholder = GtkBox::new(Orientation::Vertical, 8);
        placeholder.add_css_class("page-placeholder");
        placeholder.set_valign(Align::Center);
        placeholder.set_halign(Align::Center);

        let spinner = Spinner::builder()
            .spinning(true)
            .width_request(32)
            .height_request(32)
            .build();

        let page_label = Label::builder()
            .label(format!("Page {} / {}", page_num + 1, total_pages))
            .css_classes(vec!["dim-label".to_string()])
            .build();

        placeholder.append(&spinner);
        placeholder.append(&page_label);

        let picture = Picture::builder()
            .can_shrink(false)
            .content_fit(ContentFit::Contain)
            .hexpand(true)
            .vexpand(false)
            .valign(Align::Fill)
            .build();
        picture.set_visible(false);

        container.append(&placeholder);
        container.append(&picture);

        Self {
            key,
            container,
            picture,
            placeholder,
            spinner,
            page_label,
            is_loaded: Rc::new(RefCell::new(false)),
            is_loading: Rc::new(RefCell::new(false)),
            expected_height: calc_height,
            orig_width: Cell::new(width),
            orig_height: Cell::new(height),
            manga_width: Rc::new(RefCell::new(600)),
            last_viewport_w: Cell::new(800.0),
            last_viewport_h: Cell::new(900.0),
        }
    }

    pub fn get_dimensions(&self) -> (u32, u32) {
        let w = self.orig_width.get();
        let h = self.orig_height.get();
        if w > 0 && h > 0 {
            (w, h)
        } else {
            (800, 1200)
        }
    }

    /// Calculates the horizontal display width for this page in manga mode.
    /// Standard portrait pages scale to fill viewport height (w = viewport_h / aspect).
    /// Double-page spreads (w > h or width > viewport_w) scale down to fit within viewport width.
    pub fn calc_manga_width(&self) -> i32 {
        let (w, h) = self.get_dimensions();
        let aspect = h as f64 / w as f64;
        let v_h = self.last_viewport_h.get();
        let v_h = if v_h > 0.0 { v_h } else { 900.0 };
        let v_w = self.last_viewport_w.get();
        let v_w = if v_w > 0.0 { v_w } else { 800.0 };

        let height_fit_w = v_h / aspect;
        if w > h || height_fit_w > v_w {
            (v_w as i32).max(100)
        } else {
            (height_fit_w as i32).max(100)
        }
    }

    pub fn update_layout_for_mode(&self, mode: ReadingMode, viewport_w: f64, viewport_h: f64) {
        if viewport_w > 0.0 {
            self.last_viewport_w.set(viewport_w);
        }
        if viewport_h > 0.0 {
            self.last_viewport_h.set(viewport_h);
        }
        match mode {
            ReadingMode::ContinuousVertical => {
                self.container.remove_css_class("manga-page");
                self.container.set_vexpand(false);
                self.container.set_hexpand(true);
                self.container.set_valign(Align::Fill);
                self.picture.set_vexpand(false);
                self.picture.set_valign(Align::Fill);
                self.picture.set_can_shrink(false);
                self.picture.set_content_fit(ContentFit::Contain);
                self.picture.set_hexpand(true);
                self.container.set_width_request(-1);
                if !*self.is_loaded.borrow() {
                    self.container.set_height_request(self.expected_height);
                    self.placeholder.set_height_request(self.expected_height);
                } else {
                    self.container.set_height_request(-1);
                }
            }
            ReadingMode::ContinuousHorizontal => {
                self.container.add_css_class("manga-page");
                self.container.set_vexpand(true);
                self.container.set_hexpand(false);
                self.container.set_valign(Align::Fill);
                self.container.set_height_request(-1);

                self.picture.set_can_shrink(true);
                self.picture.set_content_fit(ContentFit::Contain);
                self.picture.set_vexpand(true);
                self.picture.set_hexpand(false);
                self.picture.set_valign(Align::Center);

                self.placeholder.set_vexpand(true);
                self.placeholder.set_valign(Align::Center);
                self.placeholder.set_height_request(-1);

                let calc_width = self.calc_manga_width();
                *self.manga_width.borrow_mut() = calc_width;
                self.container.set_width_request(calc_width);
                self.picture.set_width_request(calc_width);
                self.placeholder.set_width_request(calc_width);
            }
        }
    }

    pub fn set_loaded(&self, texture: &Texture, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.orig_width.set(width);
            self.orig_height.set(height);
        }

        self.picture.set_paintable(Some(texture));

        let is_manga = self.container.has_css_class("manga-page");
        if !is_manga {
            self.container.set_height_request(-1);
            self.container.set_width_request(-1);
            self.container.set_vexpand(false);
            self.container.set_hexpand(true);
            self.container.set_valign(Align::Fill);

            self.picture.set_content_fit(ContentFit::Contain);
            self.picture.set_can_shrink(false);
            self.picture.set_hexpand(true);
            self.picture.set_vexpand(false);
            self.picture.set_valign(Align::Fill);
        } else {
            let calc_width = self.calc_manga_width();
            *self.manga_width.borrow_mut() = calc_width;

            self.container.set_height_request(-1);
            self.container.set_width_request(calc_width);
            self.container.set_vexpand(true);
            self.container.set_hexpand(false);
            self.container.set_valign(Align::Fill);

            self.picture.set_content_fit(ContentFit::Contain);
            self.picture.set_can_shrink(true);
            self.picture.set_vexpand(true);
            self.picture.set_hexpand(false);
            self.picture.set_width_request(calc_width);
            self.picture.set_valign(Align::Center);
        }

        self.placeholder.set_visible(false);
        self.picture.set_visible(true);
        self.spinner.set_spinning(false);

        *self.is_loaded.borrow_mut() = true;
        *self.is_loading.borrow_mut() = false;
    }

    pub fn set_unloaded(&self) {
        self.picture.set_paintable(None::<&gdk4::Texture>);
        self.picture.set_visible(false);

        let is_manga = self.container.has_css_class("manga-page");
        if !is_manga {
            self.container.set_height_request(self.expected_height);
            self.placeholder.set_height_request(self.expected_height);
        } else {
            let manga_w = *self.manga_width.borrow();
            self.container.set_height_request(-1);
            self.container.set_width_request(manga_w);
            self.placeholder.set_height_request(-1);
            self.placeholder.set_width_request(manga_w);
        }
        self.placeholder.set_visible(true);
        self.spinner.set_spinning(false);

        *self.is_loaded.borrow_mut() = false;
        *self.is_loading.borrow_mut() = false;
    }

    pub fn set_loading(&self) {
        self.spinner.set_spinning(true);
        *self.is_loading.borrow_mut() = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_mode_parsing_and_str() {
        assert_eq!(
            ReadingMode::from_str_loose("manga"),
            ReadingMode::ContinuousHorizontal
        );
        assert_eq!(
            ReadingMode::from_str_loose("horizontal"),
            ReadingMode::ContinuousHorizontal
        );
        assert_eq!(
            ReadingMode::from_str_loose("paged"),
            ReadingMode::ContinuousHorizontal
        );
        assert_eq!(
            ReadingMode::from_str_loose("webtoon"),
            ReadingMode::ContinuousVertical
        );
        assert_eq!(
            ReadingMode::from_str_loose("vertical"),
            ReadingMode::ContinuousVertical
        );
        assert_eq!(
            ReadingMode::from_str_loose("unknown"),
            ReadingMode::ContinuousVertical
        );

        assert_eq!(ReadingMode::ContinuousVertical.as_str(), "webtoon");
        assert_eq!(ReadingMode::ContinuousHorizontal.as_str(), "manga");
    }

    #[test]
    fn test_page_widget_aspect_ratio_calculation() {
        if gtk4::init().is_err() {
            return;
        }

        let key = PageKey {
            chapter_idx: 0,
            page_idx: 0,
        };
        // 1200x1800 page (aspect 1.5)
        let pw = PageWidget::new(key, 0, 10, 1200, 1800);
        assert_eq!(pw.get_dimensions(), (1200, 1800));

        // In continuous horizontal mode with viewport (width 800, height 900):
        // Portrait page: Expected width = 900 / 1.5 = 600 (< 800)
        pw.update_layout_for_mode(ReadingMode::ContinuousHorizontal, 800.0, 900.0);
        assert_eq!(*pw.manga_width.borrow(), 600);

        // Double-page spread (1800x1200, width > height) in viewport (width 800, height 900):
        let spread_key = PageKey {
            chapter_idx: 0,
            page_idx: 1,
        };
        let spread_pw = PageWidget::new(spread_key, 1, 10, 1800, 1200);
        spread_pw.update_layout_for_mode(ReadingMode::ContinuousHorizontal, 800.0, 900.0);
        // Scaled to fit viewport width = 800
        assert_eq!(*spread_pw.manga_width.borrow(), 800);

        // Resizing window wider to width 1100: spread scales up cleanly to 1100
        spread_pw.update_layout_for_mode(ReadingMode::ContinuousHorizontal, 1100.0, 900.0);
        assert_eq!(*spread_pw.manga_width.borrow(), 1100);
    }
}
