use crate::cache::{MemoryManager, PageKey};
use qmetaobject::*;
use std::sync::{Arc, Mutex};

cpp!({
    #include <QtGui/QImage>
    #include <QtQml/QQmlEngine>
    #include <QtQuick/QQuickImageProvider>
});

static mut GLOBAL_MEMORY_MANAGER: Option<Arc<Mutex<MemoryManager>>> = None;

pub fn set_global_memory_manager(mem: Arc<Mutex<MemoryManager>>) {
    unsafe {
        GLOBAL_MEMORY_MANAGER = Some(mem);
    }
}

pub fn get_page_qimage(chap_idx: usize, page_idx: usize, out_width: &mut i32, out_height: &mut i32) -> QImage {
    unsafe {
        if let Some(ref mem_arc) = GLOBAL_MEMORY_MANAGER {
            let key = PageKey {
                chapter_idx: chap_idx,
                page_idx,
            };
            if let Ok(mut mem) = mem_arc.lock() {
                if let Some(data) = mem.get(&key) {
                    *out_width = data.width as i32;
                    *out_height = data.height as i32;
                    let ptr = data.rgba_bytes.as_ptr();
                    let w = data.width as i32;
                    let h = data.height as i32;
                    return cpp!(unsafe [ptr as "const uint8_t*", w as "int", h as "int"] -> QImage as "QImage" {
                        return QImage((const uchar*)ptr, w, h, QImage::Format_RGBA8888);
                    });
                }
            }
        }
    }

    *out_width = 1;
    *out_height = 1;
    cpp!(unsafe [] -> QImage as "QImage" {
        QImage fallback(1, 1, QImage::Format_RGBA8888);
        fallback.fill(Qt::transparent);
        return fallback;
    })
}

pub fn register_image_provider(qml_engine: &QmlEngine, memory_manager: Arc<Mutex<MemoryManager>>) {
    set_global_memory_manager(memory_manager);

    let engine_ptr = qml_engine.cpp_ptr();

    cpp!(unsafe [engine_ptr as "QQmlEngine*"] {
        class CbzImageProvider : public QQuickImageProvider {
        public:
            CbzImageProvider() : QQuickImageProvider(QQuickImageProvider::Image) {}

            QImage requestImage(const QString &id, QSize *size, const QSize &/*requestedSize*/) override {
                QStringList parts = id.split('/');
                if (parts.size() == 2) {
                    uintptr_t chapIdx = parts[0].toULongLong();
                    uintptr_t pageIdx = parts[1].toULongLong();
                    int w = 1, h = 1;
                    QImage img = rust!(Rust_Cbz_GetImage [chapIdx: usize as "uintptr_t", pageIdx: usize as "uintptr_t", w: &mut i32 as "int&", h: &mut i32 as "int&"] -> QImage as "QImage" {
                        get_page_qimage(chapIdx, pageIdx, w, h)
                    });
                    if (size) {
                        *size = QSize(w, h);
                    }
                    return img;
                }
                if (size) {
                    *size = QSize(1, 1);
                }
                QImage fallback(1, 1, QImage::Format_RGBA8888);
                fallback.fill(Qt::transparent);
                return fallback;
            }
        };

        if (engine_ptr) {
            engine_ptr->addImageProvider(QStringLiteral("cbz"), new CbzImageProvider());
        }
    });
}
