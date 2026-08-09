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
    #include <QtGui/QImage>
    #include <QtQml/QQmlEngine>
    #include <QtQuick/QQuickImageProvider>
});

fn main() {
    // Set Qt Application Name and Organization for Breeze / KDE Plasma integration
    env::set_var("QT_QUICK_CONTROLS_STYLE", "org.kde.desktop");

    qmetaobject::qml_register_type::<ContinuumEngine>(c"ContinuumEngine", 1, 0, c"ContinuumEngine");

    let memory_manager = Arc::new(Mutex::new(MemoryManager::new()));

    let mut qml_engine = QmlEngine::new();

    ui::register_image_provider(&qml_engine, memory_manager.clone());

    let continuum_engine = QObjectBox::new(ContinuumEngine::new(memory_manager));

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let path = PathBuf::from(&args[1]);
        if path.exists() {
            let path_str = path.to_string_lossy().to_string();
            continuum_engine.pinned().borrow_mut().open_file(QString::from(path_str.as_str()));
        }
    }

    qml_engine.set_object_property("engine".into(), continuum_engine.pinned());

    let qml_path = env::current_dir()
        .unwrap_or_default()
        .join("qml/main.qml");
    qml_engine.load_file(QString::from(qml_path.to_string_lossy().to_string().as_str()));

    qml_engine.exec();
}
