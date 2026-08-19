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
        let path = env::args().nth(1).map(PathBuf::from);
        open_window(app, path);
    });

    app.connect_open(|app, files, _| {
        let path = files.first().and_then(|f| f.path());
        open_window(app, path);
    });

    app.run()
}

fn open_window(app: &libadwaita::Application, path: Option<PathBuf>) {
    if let Some(active_win) = app.active_window() {
        active_win.present();
        return;
    }

    let manhwa_window = ManhwaWindow::new(app);
    if let Some(p) = path.filter(|p| p.exists()) {
        let _ = manhwa_window.reader.load_initial_file(p);
    }
    manhwa_window.window.present();
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

        gtk4::Window::set_default_icon_name("dev.continuum.ManhwaReader");
    }
}
