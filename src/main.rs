#![recursion_limit = "256"]

#[macro_use]
extern crate cpp;

mod cache;
mod cbz;
mod cli;
mod ui;

use cache::MemoryManager;
use cli::CliOptions;
use qmetaobject::*;
use std::env;
use std::sync::{Arc, Mutex};
use ui::ContinuumEngine;

cpp!({
    #include <QtGui/QGuiApplication>
    #include <QtGui/QIcon>
    #include <QtGui/QImage>
    #include <QtQml/QQmlEngine>
    #include <QtQuick/QQuickImageProvider>
});

qrc!(init_resources,
    "" {
        "qml/main.qml",
        "qml/ReaderView.qml",
        "qml/ShortcutsSheet.qml",
        "continuum.png",
    },
);

fn main() {
    init_resources();

<<<<<<< HEAD
    let opts = CliOptions::parse_from_args(env::args().skip(1));

    // Set Qt Application Name and Organization for Breeze / KDE Plasma integration
    env::set_var("QT_QUICK_CONTROLS_STYLE", "org.kde.desktop");

    qmetaobject::qml_register_type::<ContinuumEngine>(c"ContinuumEngine", 1, 0, c"ContinuumEngine");
||||||| 4b8bf32
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
=======
    app.connect_activate(|app| {
        let path = env::args().nth(1).map(PathBuf::from);
        open_window(app, path);
    });
>>>>>>> b37efd4b3e516a765036ba7a99a0eb027855edc2

<<<<<<< HEAD
    let memory_manager = Arc::new(Mutex::new(MemoryManager::new()));
    let continuum_engine = QObjectBox::new(ContinuumEngine::new(memory_manager.clone()));
    let archives = continuum_engine.pinned().borrow().archives.clone();
||||||| 4b8bf32
    app.connect_open(|app, files, _| {
        let manhwa_window = ManhwaWindow::new(app);
        if let Some(file) = files.first() {
            if let Some(path) = file.path() {
                let _ = manhwa_window.reader.load_initial_file(path);
            }
        }
        manhwa_window.window.present();
    });
=======
    app.connect_open(|app, files, _| {
        let path = files.first().and_then(|f| f.path());
        open_window(app, path);
    });
>>>>>>> b37efd4b3e516a765036ba7a99a0eb027855edc2

<<<<<<< HEAD
    if opts.tui_mode {
        continuum_engine.pinned().borrow_mut().set_tui_mode(true);
||||||| 4b8bf32
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
=======
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
>>>>>>> b37efd4b3e516a765036ba7a99a0eb027855edc2
    }

    let mut qml_engine = QmlEngine::new();

    cpp!(unsafe [] {
        QGuiApplication::setApplicationName(QString::fromUtf8("continuum"));
        QGuiApplication::setApplicationDisplayName(QString::fromUtf8("Continuum"));
        QGuiApplication::setDesktopFileName(QString::fromUtf8("dev.continuum.ManhwaReader"));
        QGuiApplication::setOrganizationName(QString::fromUtf8("Continuum"));
        QGuiApplication::setOrganizationDomain(QString::fromUtf8("continuum.dev"));
        QGuiApplication::setWindowIcon(QIcon::fromTheme(QString::fromUtf8("dev.continuum.ManhwaReader"), QIcon(QString::fromUtf8(":/continuum.png"))));
    });

    ui::register_image_provider(&qml_engine, memory_manager, archives);

    if let Some(path) = opts.file {
        if path.exists() {
            let path_str = path.to_string_lossy().to_string();
            let initial_page = opts.page.unwrap_or(1);
            continuum_engine
                .pinned()
                .borrow_mut()
                .open_file_with_page(QString::from(path_str.as_str()), initial_page);
        }
    }

    qml_engine.set_object_property("engine".into(), continuum_engine.pinned());

    qml_engine.load_file(QString::from("qrc:/qml/main.qml"));

    qml_engine.exec();

    continuum_engine.pinned().borrow().emit_exit_payload();
}
