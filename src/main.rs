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

    let opts = CliOptions::parse_from_args(env::args().skip(1));

    // Set Qt Application Name and Organization for Breeze / KDE Plasma integration
    env::set_var("QT_QUICK_CONTROLS_STYLE", "org.kde.desktop");

    qmetaobject::qml_register_type::<ContinuumEngine>(c"ContinuumEngine", 1, 0, c"ContinuumEngine");

    let memory_manager = Arc::new(Mutex::new(MemoryManager::new()));
    let continuum_engine = QObjectBox::new(ContinuumEngine::new(memory_manager.clone()));
    let archives = continuum_engine.pinned().borrow().archives.clone();

    if opts.tui_mode {
        continuum_engine.pinned().borrow_mut().set_tui_mode(true);
    }

    if let Some(mode) = &opts.mode {
        continuum_engine
            .pinned()
            .borrow_mut()
            .set_initial_reading_mode(mode);
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
