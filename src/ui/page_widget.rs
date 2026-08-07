use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation, Picture, Spinner, ContentFit, Align};
use gdk4::Texture;
use std::rc::Rc;
use std::cell::RefCell;
use crate::cache::PageKey;

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
        }
    }

    pub fn set_loaded(&self, texture: &Texture, _width: u32, _height: u32) {
        if *self.is_loaded.borrow() {
            return;
        }

        self.picture.set_paintable(Some(texture));
        self.container.set_height_request(-1);
        self.picture.set_height_request(-1);

        self.placeholder.set_visible(false);
        self.picture.set_visible(true);
        self.spinner.set_spinning(false);

        *self.is_loaded.borrow_mut() = true;
        *self.is_loading.borrow_mut() = false;
    }

    pub fn set_unloaded(&self) {
        self.picture.set_paintable(None::<&gdk4::Texture>);
        self.picture.set_visible(false);
        self.container.set_height_request(self.expected_height);
        self.placeholder.set_height_request(self.expected_height);
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
