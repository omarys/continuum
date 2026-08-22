use crate::cache::PageKey;
use gdk4::Texture;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, ContentFit, Label, Orientation, Picture, Spinner};
use std::cell::RefCell;
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
    pub orig_width: u32,
    pub orig_height: u32,
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

        let picture = Picture::builder()
            .content_fit(ContentFit::Contain)
            .can_shrink(false)
            .hexpand(true)
            .vexpand(false)
            .visible(false)
            .build();

        let placeholder = GtkBox::new(Orientation::Vertical, 8);
        placeholder.add_css_class("page-placeholder");
        placeholder.set_height_request(calc_height);
        placeholder.set_valign(Align::Center);
        placeholder.set_halign(Align::Center);

        let spinner = Spinner::new();
        spinner.set_spinning(false);
        spinner.set_size_request(32, 32);

        let page_label = Label::builder()
            .label(format!("Page {} / {}", page_num + 1, total_pages))
            .css_classes(vec!["caption".to_string(), "dim-label".to_string()])
            .build();

        placeholder.append(&spinner);
        placeholder.append(&page_label);

        container.append(&picture);
        container.append(&placeholder);

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
            orig_width: width,
            orig_height: height,
        }
    }

    pub fn update_layout_for_mode(&self, mode: ReadingMode) {
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
                self.container.set_width_request(-1);

                self.picture.set_can_shrink(true);
                self.picture.set_content_fit(ContentFit::Contain);
                self.picture.set_vexpand(true);
                self.picture.set_hexpand(false);
                self.picture.set_valign(Align::Fill);

                self.placeholder.set_vexpand(true);
                self.placeholder.set_valign(Align::Fill);
                self.placeholder.set_height_request(-1);

                let calc_width = if self.orig_width > 0 && self.orig_height > 0 {
                    let aspect = self.orig_height as f64 / self.orig_width as f64;
                    ((900.0 / aspect) as i32).max(300)
                } else {
                    600
                };
                self.placeholder.set_width_request(calc_width);
            }
        }
    }

    pub fn set_loaded(&self, texture: &Texture, _width: u32, _height: u32) {
        if *self.is_loaded.borrow() {
            return;
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
            self.container.set_height_request(-1);
            self.container.set_width_request(-1);
            self.container.set_vexpand(true);
            self.container.set_hexpand(false);
            self.container.set_valign(Align::Fill);

            self.picture.set_content_fit(ContentFit::Contain);
            self.picture.set_can_shrink(true);
            self.picture.set_vexpand(true);
            self.picture.set_hexpand(false);
            self.picture.set_valign(Align::Fill);
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
            self.container.set_height_request(-1);
            self.placeholder.set_height_request(-1);
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
