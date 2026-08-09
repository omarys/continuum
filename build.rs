fn main() {
    let mut config = cpp_build::Config::new();
    config
        .include("/usr/include/qt6")
        .include("/usr/include/qt6/QtGui")
        .include("/usr/include/qt6/QtQml")
        .include("/usr/include/qt6/QtCore")
        .include("/usr/include/qt6/QtQuick")
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
