use crate::cache::{MemoryManager, PageKey};
use crate::cbz::archive::{CbzArchive, DirectorySeries};
use qmetaobject::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ChapterInfo {
    pub chapter_id: usize,
    pub series_idx: usize,
    pub filename: String,
    pub page_count: usize,
    pub archive_path: PathBuf,
}

#[derive(Default, QObject)]
pub struct ContinuumEngine {
    base: qt_base_class!(trait QObject),

    // QML Properties
    pub has_comic: qt_property!(bool; NOTIFY has_comic_changed),
    pub comic_title: qt_property!(QString; NOTIFY comic_title_changed),
    pub reading_mode: qt_property!(i32; NOTIFY reading_mode_changed), // 0: Vertical, 1: Horizontal
    pub current_chapter_name: qt_property!(QString; NOTIFY current_chapter_name_changed),
    pub current_page_number: qt_property!(i32; NOTIFY current_page_number_changed),
    pub total_pages_count: qt_property!(i32; NOTIFY total_pages_count_changed),
    pub current_chapter_idx: qt_property!(i32; NOTIFY current_chapter_idx_changed),
    pub total_chapters_count: qt_property!(i32; NOTIFY total_chapters_count_changed),

    // Signals
    pub has_comic_changed: qt_signal!(),
    pub comic_title_changed: qt_signal!(),
    pub reading_mode_changed: qt_signal!(),
    pub current_chapter_name_changed: qt_signal!(),
    pub current_page_number_changed: qt_signal!(),
    pub total_pages_count_changed: qt_signal!(),
    pub current_chapter_idx_changed: qt_signal!(),
    pub total_chapters_count_changed: qt_signal!(),
    pub page_loaded: qt_signal!(chapter_idx: i32, page_idx: i32),

    // Invokable QML Methods
    pub open_file: qt_method!(fn(&mut self, file_path_qstr: QString) -> bool),
    pub toggle_reading_mode: qt_method!(fn(&mut self)),
    pub request_pages_around: qt_method!(fn(&mut self, current_global_idx: i32)),
    pub get_chapter_count: qt_method!(fn(&self) -> i32),
    pub get_chapter_title: qt_method!(fn(&self, chapter_idx: i32) -> QString),
    pub get_chapter_page_count: qt_method!(fn(&self, chapter_idx: i32) -> i32),
    pub get_total_page_count: qt_method!(fn(&self) -> i32),
    pub next_chapter: qt_method!(fn(&mut self) -> bool),

    // Rust Internal State
    pub memory_manager: Arc<Mutex<MemoryManager>>,
    series: Arc<Mutex<Option<DirectorySeries>>>,
    chapters: Arc<Mutex<Vec<ChapterInfo>>>,
    archives: Arc<Mutex<HashMap<usize, Arc<CbzArchive>>>>,
    global_to_key: Arc<Mutex<Vec<PageKey>>>,
    in_flight: Arc<Mutex<HashSet<PageKey>>>,
    first_loaded_series_idx: Arc<Mutex<usize>>,
    last_loaded_series_idx: Arc<Mutex<usize>>,
    next_chapter_id: Arc<Mutex<usize>>,
}

impl ContinuumEngine {
    pub fn new(memory_manager: Arc<Mutex<MemoryManager>>) -> Self {
        Self {
            base: Default::default(),
            has_comic: false,
            comic_title: QString::from(""),
            reading_mode: 0,
            current_chapter_name: QString::from(""),
            current_page_number: 1,
            total_pages_count: 0,
            current_chapter_idx: 0,
            total_chapters_count: 0,

            has_comic_changed: Default::default(),
            comic_title_changed: Default::default(),
            reading_mode_changed: Default::default(),
            current_chapter_name_changed: Default::default(),
            current_page_number_changed: Default::default(),
            total_pages_count_changed: Default::default(),
            current_chapter_idx_changed: Default::default(),
            total_chapters_count_changed: Default::default(),
            page_loaded: Default::default(),

            open_file: Default::default(),
            toggle_reading_mode: Default::default(),
            request_pages_around: Default::default(),
            get_chapter_count: Default::default(),
            get_chapter_title: Default::default(),
            get_chapter_page_count: Default::default(),
            get_total_page_count: Default::default(),
            next_chapter: Default::default(),

            memory_manager,
            series: Arc::new(Mutex::new(None)),
            chapters: Arc::new(Mutex::new(Vec::new())),
            archives: Arc::new(Mutex::new(HashMap::new())),
            global_to_key: Arc::new(Mutex::new(Vec::new())),
            in_flight: Arc::new(Mutex::new(HashSet::new())),
            first_loaded_series_idx: Arc::new(Mutex::new(0)),
            last_loaded_series_idx: Arc::new(Mutex::new(0)),
            next_chapter_id: Arc::new(Mutex::new(0)),
        }
    }

    pub fn open_file(&mut self, file_path_qstr: QString) -> bool {
        let file_path = PathBuf::from(file_path_qstr.to_string());
        if !file_path.exists() {
            return false;
        }

        let series = DirectorySeries::new(&file_path);
        let archive = match CbzArchive::open(&file_path) {
            Ok(a) => Arc::new(a),
            Err(e) => {
                eprintln!("Error opening CBZ archive: {}", e);
                return false;
            }
        };

        if let Ok(mut mem) = self.memory_manager.lock() {
            mem.clear();
        }
        if let Ok(mut chaps) = self.chapters.lock() {
            chaps.clear();
        }
        if let Ok(mut archs) = self.archives.lock() {
            archs.clear();
        }
        if let Ok(mut keys) = self.global_to_key.lock() {
            keys.clear();
        }
        if let Ok(mut inflight) = self.in_flight.lock() {
            inflight.clear();
        }

        let initial_idx = series.current_index;
        *self.first_loaded_series_idx.lock().unwrap() = initial_idx;
        *self.last_loaded_series_idx.lock().unwrap() = initial_idx;
        *self.next_chapter_id.lock().unwrap() = 0;

        let total_series_count = series.dir_files.len() as i32;
        let title_name = archive.filename.clone();
        *self.series.lock().unwrap() = Some(series);

        self.append_archive(archive, initial_idx);

        self.has_comic = true;
        self.comic_title = QString::from(title_name.as_str());
        self.current_chapter_name = QString::from(title_name.as_str());
        self.current_chapter_idx = (initial_idx + 1) as i32;
        self.total_chapters_count = total_series_count;

        let total_pages = self.global_to_key.lock().unwrap().len() as i32;
        self.total_pages_count = total_pages;
        self.current_page_number = 1;

        self.has_comic_changed();
        self.comic_title_changed();
        self.current_chapter_name_changed();
        self.current_chapter_idx_changed();
        self.total_chapters_count_changed();
        self.total_pages_count_changed();
        self.current_page_number_changed();

        self.request_pages_around(0);

        true
    }

    pub fn toggle_reading_mode(&mut self) {
        self.reading_mode = if self.reading_mode == 0 { 1 } else { 0 };
        self.reading_mode_changed();
    }

    pub fn request_pages_around(&mut self, current_global_idx: i32) {
        let global_idx = current_global_idx.max(0) as usize;

        let global_keys = match self.global_to_key.lock() {
            Ok(k) => k.clone(),
            Err(_) => return,
        };

        if global_keys.is_empty() {
            return;
        }

        let total = global_keys.len();
        let start = global_idx.saturating_sub(3);
        let end = (global_idx + 8).min(total);

        let archives = self.archives.lock().unwrap().clone();
        let memory_manager = self.memory_manager.clone();
        let in_flight = self.in_flight.clone();

        for (idx, key) in global_keys.iter().enumerate().take(end).skip(start) {
            if let Ok(mem) = memory_manager.lock() {
                if mem.contains(key) {
                    continue;
                }
            }
            {
                let mut inflight = in_flight.lock().unwrap();
                if !inflight.insert(key.clone()) {
                    continue;
                }
            }

            if let Some(archive) = archives.get(&key.chapter_idx) {
                let archive_clone = archive.clone();
                let key_clone = key.clone();
                let memory_manager_clone = memory_manager.clone();
                let in_flight_clone = in_flight.clone();

                thread::spawn(move || {
                    if key_clone.page_idx < archive_clone.image_entries.len() {
                        let inner_name = &archive_clone.image_entries[key_clone.page_idx];
                        if let Ok(file) = std::fs::File::open(&archive_clone.path) {
                            let reader = std::io::BufReader::new(file);
                            if let Ok(mut zip) = zip::ZipArchive::new(reader) {
                                if let Ok(mut entry) = zip.by_name(inner_name) {
                                    let mut bytes = Vec::new();
                                    if std::io::Read::read_to_end(&mut entry, &mut bytes).is_ok() {
                                        if let Ok((w, h, rgba)) = CbzArchive::decode_page_bytes(&bytes) {
                                            if let Ok(page_data) = CbzArchive::create_page_data(w, h, rgba) {
                                                if let Ok(mut mem) = memory_manager_clone.lock() {
                                                    mem.insert(key_clone.clone(), page_data, idx, &|_| idx);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Ok(mut inflight) = in_flight_clone.lock() {
                        inflight.remove(&key_clone);
                    }
                });
            }
        }
    }

    pub fn get_chapter_count(&self) -> i32 {
        self.chapters.lock().unwrap().len() as i32
    }

    pub fn get_chapter_title(&self, chapter_idx: i32) -> QString {
        let chaps = self.chapters.lock().unwrap();
        if let Some(chap) = chaps.get(chapter_idx as usize) {
            QString::from(chap.filename.as_str())
        } else {
            QString::from("")
        }
    }

    pub fn get_chapter_page_count(&self, chapter_idx: i32) -> i32 {
        let chaps = self.chapters.lock().unwrap();
        if let Some(chap) = chaps.get(chapter_idx as usize) {
            chap.page_count as i32
        } else {
            0
        }
    }

    pub fn get_total_page_count(&self) -> i32 {
        self.global_to_key.lock().unwrap().len() as i32
    }

    pub fn next_chapter(&mut self) -> bool {
        let last_idx = *self.last_loaded_series_idx.lock().unwrap();
        let series_opt = self.series.lock().unwrap().clone();

        if let Some(series) = series_opt {
            if last_idx + 1 < series.dir_files.len() {
                let next_idx = last_idx + 1;
                let next_path = &series.dir_files[next_idx];
                if let Ok(archive) = CbzArchive::open(next_path) {
                    self.append_archive(Arc::new(archive), next_idx);
                    self.total_pages_count = self.global_to_key.lock().unwrap().len() as i32;
                    self.current_chapter_idx = (next_idx + 1) as i32;
                    self.total_pages_count_changed();
                    self.current_chapter_idx_changed();
                    return true;
                }
            }
        }
        false
    }

    fn append_archive(&mut self, archive: Arc<CbzArchive>, series_idx: usize) {
        let chapter_id = *self.next_chapter_id.lock().unwrap();
        *self.next_chapter_id.lock().unwrap() += 1;

        let total_pages = archive.page_count();

        let mut keys_lock = self.global_to_key.lock().unwrap();
        for page_idx in 0..total_pages {
            let key = PageKey {
                chapter_idx: chapter_id,
                page_idx,
            };
            keys_lock.push(key);
        }

        self.archives
            .lock()
            .unwrap()
            .insert(chapter_id, archive.clone());

        self.chapters.lock().unwrap().push(ChapterInfo {
            chapter_id,
            series_idx,
            filename: archive.filename.clone(),
            page_count: total_pages,
            archive_path: archive.path.clone(),
        });

        *self.last_loaded_series_idx.lock().unwrap() = series_idx;
    }
}
