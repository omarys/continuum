#![recursion_limit = "256"]

#[macro_use]
extern crate cpp;

mod cache;
mod cbz;
mod ui;

use cache::MemoryManager;
use qmetaobject::*;
use std::env;
use std::path::PathBuf;
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

    // Set Qt Application Name and Organization for Breeze / KDE Plasma integration
    env::set_var("QT_QUICK_CONTROLS_STYLE", "org.kde.desktop");

    qmetaobject::qml_register_type::<ContinuumEngine>(c"ContinuumEngine", 1, 0, c"ContinuumEngine");

    let memory_manager = Arc::new(Mutex::new(MemoryManager::new()));
    let continuum_engine = QObjectBox::new(ContinuumEngine::new(memory_manager.clone()));
    let archives = continuum_engine.pinned().borrow().archives.clone();

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

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let path = PathBuf::from(&args[1]);
        if path.exists() {
            let path_str = path.to_string_lossy().to_string();
            continuum_engine
                .pinned()
                .borrow_mut()
                .open_file(QString::from(path_str.as_str()));
        }
    }

    qml_engine.set_object_property("engine".into(), continuum_engine.pinned());

    qml_engine.load_file(QString::from("qrc:/qml/main.qml"));

    qml_engine.exec();
}
