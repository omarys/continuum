use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Label, Orientation};

pub fn create_chapter_banner(chapter_title: &str, page_count: usize) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 6);
    container.set_margin_top(32);
    container.set_margin_bottom(24);
    container.set_margin_start(16);
    container.set_margin_end(16);
    container.add_css_class("chapter-banner");

    let card = GtkBox::new(Orientation::Horizontal, 12);
    card.add_css_class("card");
    card.add_css_class("accent-card");
    card.set_halign(Align::Center);
    card.set_margin_top(8);
    card.set_margin_bottom(8);

    let icon = gtk4::Image::from_icon_name("book-open-symbolic");
    icon.set_icon_size(gtk4::IconSize::Large);
    card.append(&icon);

    let label_box = GtkBox::new(Orientation::Vertical, 2);

    let title_label = Label::builder()
        .label(chapter_title)
        .halign(Align::Start)
        .css_classes(vec!["title-3".to_string(), "bold".to_string()])
        .build();

    let subtitle_label = Label::builder()
        .label(format!("{} pages", page_count))
        .halign(Align::Start)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    label_box.append(&title_label);
    label_box.append(&subtitle_label);

    card.append(&label_box);
    container.append(&card);

    container
}
