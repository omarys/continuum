use crate::cache::{MemoryManager, PageKey};
use crate::cbz::archive::{CbzArchive, MAX_PAGE_FILE_SIZE};
use cpp::cpp;
use qmetaobject::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

cpp!({
    #include <QtGui/QImage>
    #include <QtQml/QQmlEngine>
    #include <QtQuick/QQuickImageProvider>
});

pub type ArchiveMap = HashMap<usize, Arc<CbzArchive>>;
pub type SharedArchives = Arc<Mutex<ArchiveMap>>;
pub type SharedMemoryManager = Arc<Mutex<MemoryManager>>;

static mut GLOBAL_MEMORY_MANAGER: Option<SharedMemoryManager> = None;
static mut GLOBAL_ARCHIVES: Option<SharedArchives> = None;

pub fn set_global_handles(mem: SharedMemoryManager, archs: SharedArchives) {
    unsafe {
        GLOBAL_MEMORY_MANAGER = Some(mem);
        GLOBAL_ARCHIVES = Some(archs);
    }
}

pub fn get_page_qimage(
    chap_idx: usize,
    page_idx: usize,
    out_width: &mut i32,
    out_height: &mut i32,
) -> QImage {
    let key = PageKey {
        chapter_idx: chap_idx,
        page_idx,
    };

    // 1. Try in-memory cache first
    unsafe {
        if let Some(ref mem_arc) = GLOBAL_MEMORY_MANAGER {
            if let Ok(mut mem) = mem_arc.lock() {
                if let Some(data) = mem.get(&key) {
                    *out_width = data.width as i32;
                    *out_height = data.height as i32;
                    let ptr = data.rgba_bytes.as_ptr();
                    let w = data.width as i32;
                    let h = data.height as i32;
                    return cpp!(unsafe [ptr as "const uint8_t*", w as "int", h as "int"] -> QImage as "QImage" {
                        return QImage((const uchar*)ptr, w, h, (qsizetype)w * 4, QImage::Format_RGBA8888).copy();
                    });
                }
            }
        }
    }

    // 2. Fallback: decode directly from archive with zip-bomb mitigation
    unsafe {
        if let Some(ref archs_arc) = GLOBAL_ARCHIVES {
            if let Ok(archs) = archs_arc.lock() {
                if let Some(archive) = archs.get(&chap_idx) {
                    if page_idx < archive.image_entries.len() {
                        let inner_name = &archive.image_entries[page_idx];
                        if let Ok(file) = std::fs::File::open(&archive.path) {
                            let reader = std::io::BufReader::new(file);
                            if let Ok(mut zip) = zip::ZipArchive::new(reader) {
                                if let Ok(mut entry) = zip.by_name(inner_name) {
                                    // Reject spoofed header sizes
                                    if entry.size() <= MAX_PAGE_FILE_SIZE {
                                        let initial_cap = (entry.size() as usize)
                                            .min(MAX_PAGE_FILE_SIZE as usize);
                                        let mut bytes = Vec::with_capacity(initial_cap);
                                        let mut limited_reader =
                                            std::io::Read::take(&mut entry, MAX_PAGE_FILE_SIZE + 1);
                                        if std::io::Read::read_to_end(
                                            &mut limited_reader,
                                            &mut bytes,
                                        )
                                        .is_ok()
                                            && bytes.len() as u64 <= MAX_PAGE_FILE_SIZE
                                        {
                                            if let Ok((w, h, rgba)) =
                                                CbzArchive::decode_page_bytes(&bytes)
                                            {
                                                if let Ok(page_data) =
                                                    CbzArchive::create_page_data(w, h, rgba)
                                                {
                                                    *out_width = page_data.width as i32;
                                                    *out_height = page_data.height as i32;
                                                    let ptr = page_data.rgba_bytes.as_ptr();
                                                    let pw = page_data.width as i32;
                                                    let ph = page_data.height as i32;

                                                    let qimg = cpp!(unsafe [ptr as "const uint8_t*", pw as "int", ph as "int"] -> QImage as "QImage" {
                                                        return QImage((const uchar*)ptr, pw, ph, (qsizetype)pw * 4, QImage::Format_RGBA8888).copy();
                                                    });

                                                    if let Some(ref mem_arc) = GLOBAL_MEMORY_MANAGER
                                                    {
                                                        if let Ok(mut mem) = mem_arc.lock() {
                                                            mem.insert(key, page_data, 0, &|_| 0);
                                                        }
                                                    }
                                                    return qimg;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    eprintln!(
        "[Continuum] Warning: Failed to load page image for chap_idx={}, page_idx={}",
        chap_idx, page_idx
    );
    *out_width = 1;
    *out_height = 1;
    cpp!(unsafe [] -> QImage as "QImage" {
        QImage fallback(1, 1, QImage::Format_RGBA8888);
        fallback.fill(Qt::transparent);
        return fallback;
    })
}

pub fn register_image_provider(
    qml_engine: &QmlEngine,
    memory_manager: SharedMemoryManager,
    archives: SharedArchives,
) {
    set_global_handles(memory_manager, archives);

    let engine_ptr = qml_engine.cpp_ptr();

    cpp!(unsafe [engine_ptr as "QQmlEngine*"] {
        class CbzImageProvider : public QQuickImageProvider {
        public:
            CbzImageProvider() : QQuickImageProvider(QQuickImageProvider::Image) {}

            QImage requestImage(const QString &id, QSize *size, const QSize &/*requestedSize*/) override {
                QString cleanId = id;
                while (cleanId.startsWith(QLatin1Char('/'))) {
                    cleanId.remove(0, 1);
                }
                QStringList parts = cleanId.split(QLatin1Char('/'));
                if (parts.size() >= 2) {
                    bool ok1 = false;
                    bool ok2 = false;
                    uintptr_t chapIdx = parts[0].toULongLong(&ok1);
                    uintptr_t pageIdx = parts[1].toULongLong(&ok2);
                    if (ok1 && ok2) {
                        int w = 1, h = 1;
                        QImage img = rust!(Rust_Cbz_GetImage [chapIdx: usize as "uintptr_t", pageIdx: usize as "uintptr_t", w: &mut i32 as "int&", h: &mut i32 as "int&"] -> QImage as "QImage" {
                            get_page_qimage(chapIdx, pageIdx, w, h)
                        });
                        if (size) {
                            *size = QSize(w, h);
                        }
                        return img;
                    }
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
