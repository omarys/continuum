fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=qml");
    println!("cargo:rerun-if-changed=continuum.png");
    let mut config = cpp_build::Config::new();
    config
        .include("/usr/include/qt6")
        .include("/usr/include/qt6/QtGui")
        .include("/usr/include/qt6/QtQml")
        .include("/usr/include/qt6/QtCore")
        .include("/usr/include/qt6/QtQuick")
        .flag("-include")
        .flag("QtGui/QGuiApplication")
        .flag("-include")
        .flag("QtGui/QIcon")
        .flag("-include")
        .flag("QtGui/QImage")
        .flag("-include")
        .flag("QtQml/QQmlEngine")
        .flag("-include")
        .flag("QtQuick/QQuickImageProvider")
        .flag("-fPIC")
        .flag("-std=c++17");

    config.build("src/main.rs");
}
