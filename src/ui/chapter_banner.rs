use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Label, Orientation, Separator};
use std::path::Path;

pub fn create_chapter_banner(chapter_title: &str, page_count: usize) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 4);
    container.set_margin_top(28);
    container.set_margin_bottom(28);
    container.set_margin_start(16);
    container.set_margin_end(16);
    container.add_css_class("chapter-banner");

    // Top chapter separator line
    let top_line = Separator::new(Orientation::Horizontal);
    top_line.add_css_class("chapter-line");

    let card = GtkBox::new(Orientation::Horizontal, 12);
    card.add_css_class("card");
    card.add_css_class("accent-card");
    card.set_halign(Align::Center);
    card.set_margin_top(12);
    card.set_margin_bottom(12);

    let icon = gtk4::Image::from_icon_name("bookmark-symbolic");
    icon.set_icon_size(gtk4::IconSize::Large);
    card.append(&icon);

    let label_box = GtkBox::new(Orientation::Vertical, 2);

    // Present clean filename stem if full path is passed
    let clean_title = Path::new(chapter_title)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(chapter_title);

    let title_label = Label::builder()
        .label(clean_title)
        .halign(Align::Start)
        .css_classes(vec!["title-3".to_string(), "bold".to_string()])
        .build();

    let subtitle_label = Label::builder()
        .label(format!("Chapter End / Start  •  {} pages", page_count))
        .halign(Align::Start)
        .css_classes(vec!["dim-label".to_string(), "caption".to_string()])
        .build();

    label_box.append(&title_label);
    label_box.append(&subtitle_label);

    card.append(&label_box);

    // Bottom chapter separator line
    let bottom_line = Separator::new(Orientation::Horizontal);
    bottom_line.add_css_class("chapter-line");

    container.append(&top_line);
    container.append(&card);
    container.append(&bottom_line);

    container
}
