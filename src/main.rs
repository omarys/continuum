mod cache;
mod cbz;
mod ui;

use gio::ApplicationFlags;
use gtk4::prelude::*;
use std::env;
use std::path::PathBuf;
use ui::ManhwaWindow;

fn main() -> glib::ExitCode {
    libadwaita::init().expect("Failed to initialize Libadwaita");

    let app = libadwaita::Application::builder()
        .application_id("dev.continuum.ManhwaReader")
        .flags(ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(|_| {
        load_custom_styles();
    });

    app.connect_activate(|app| {
        let manhwa_window = ManhwaWindow::new(app);

        let args: Vec<String> = env::args().collect();
        if args.len() > 1 {
            let path = PathBuf::from(&args[1]);
            if path.exists() {
                let _ = manhwa_window.reader.load_initial_file(path);
            }
        }

        manhwa_window.window.present();
    });

    app.connect_open(|app, files, _| {
        let manhwa_window = ManhwaWindow::new(app);
        if let Some(file) = files.first() {
            if let Some(path) = file.path() {
                let _ = manhwa_window.reader.load_initial_file(path);
            }
        }
        manhwa_window.window.present();
    });

    app.run()
}

fn load_custom_styles() {
    let provider = gtk4::CssProvider::new();
    let css = r#"
        window {
            background-color: #121212;
        }

        .page-container {
            margin: 0;
            padding: 0;
            background-color: #121212;
        }

        picture {
            margin: 0;
            padding: 0;
            border: none;
        }

        .page-placeholder {
            background-color: #1a1a1a;
            border-radius: 8px;
            margin: 0;
            padding: 24px;
            border: 1px dashed #333333;
        }

        .chapter-banner {
            background-color: #161616;
            padding: 16px 0;
        }

        .chapter-line {
            background-color: #3b3b4f;
            min-height: 2px;
            margin: 12px 0;
            opacity: 0.8;
        }

        .accent-card {
            background-color: #242424;
            padding: 12px 24px;
            border-radius: 12px;
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
        }

        scrolledwindow {
            background-color: #121212;
        }

        .manga-page {
            margin: 0 16px;
            border-left: 2px solid #2e2e42;
            border-right: 2px solid #2e2e42;
            border-radius: 6px;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.7);
        }

        .manga-fade-overlay {
            mask-image: linear-gradient(to right, transparent 0%, black 14%, black 86%, transparent 100%);
            -webkit-mask-image: linear-gradient(to right, transparent 0%, black 14%, black 86%, transparent 100%);
        }
    "#;

    provider.load_from_string(css);

    if let Some(display) = gdk4::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let icon_theme = gtk4::IconTheme::for_display(&display);
        if let Ok(cwd) = env::current_dir() {
            icon_theme.add_search_path(&cwd);
        }

        gtk4::Window::set_default_icon_name("dev.continuum.ManhwaReader");
    }
}
