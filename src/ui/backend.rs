#![allow(clippy::useless_transmute)]

use crate::cache::{MemoryManager, PageKey};
use crate::cbz::archive::{CbzArchive, DirectorySeries};
use qmetaobject::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ChapterInfo {
    pub chapter_id: usize,
    pub series_idx: usize,
    pub filename: String,
    pub page_count: usize,
    pub archive_path: PathBuf,
}

#[allow(clippy::useless_transmute)]
#[derive(Default, QObject)]
pub struct ContinuumEngine {
    base: qt_base_class!(trait QObject),

    // QML Properties with Change Notifications
    pub has_comic: qt_property!(bool; NOTIFY has_comic_changed),
    pub comic_title: qt_property!(QString; NOTIFY comic_title_changed),
    pub reading_mode: qt_property!(i32; NOTIFY reading_mode_changed), // 0: Vertical (Manhwa), 1: Horizontal (Manga)
    pub current_chapter_name: qt_property!(QString; NOTIFY current_chapter_name_changed),
    pub current_page_number: qt_property!(i32; NOTIFY current_page_number_changed),
    pub current_chapter_page_count: qt_property!(i32; NOTIFY current_chapter_page_count_changed),
    pub total_pages_count: qt_property!(i32; NOTIFY total_pages_count_changed),
    pub current_chapter_idx: qt_property!(i32; NOTIFY current_chapter_idx_changed),
    pub total_chapters_count: qt_property!(i32; NOTIFY total_chapters_count_changed),
    pub loaded_chapters_count: qt_property!(i32; NOTIFY loaded_chapters_count_changed),
    pub initial_page_idx: qt_property!(i32; NOTIFY initial_page_idx_changed),

    // Signals
    pub has_comic_changed: qt_signal!(),
    pub comic_title_changed: qt_signal!(),
    pub reading_mode_changed: qt_signal!(),
    pub current_chapter_name_changed: qt_signal!(),
    pub current_page_number_changed: qt_signal!(),
    pub current_chapter_page_count_changed: qt_signal!(),
    pub total_pages_count_changed: qt_signal!(),
    pub current_chapter_idx_changed: qt_signal!(),
    pub total_chapters_count_changed: qt_signal!(),
    pub loaded_chapters_count_changed: qt_signal!(),
    pub initial_page_idx_changed: qt_signal!(),
    pub jump_to_initial_page_requested: qt_signal!(page_idx: i32),
    pub page_loaded: qt_signal!(chapter_idx: i32, page_idx: i32),

    pub app_icon: qt_property!(QString; READ get_app_icon),

    // Invokable QML Methods
    pub open_file: qt_method!(fn(&mut self, file_path_qstr: QString) -> bool),
    pub toggle_reading_mode: qt_method!(fn(&mut self)),
    pub request_pages_around: qt_method!(fn(&mut self, current_global_idx: i32)),
    pub get_chapter_count: qt_method!(fn(&self) -> i32),
    pub get_chapter_title: qt_method!(fn(&self, chapter_idx: i32) -> QString),
    pub get_chapter_page_count: qt_method!(fn(&self, chapter_idx: i32) -> i32),
    pub get_total_page_count: qt_method!(fn(&self) -> i32),
    pub get_page_source: qt_method!(fn(&self, global_idx: i32) -> QString),
    pub is_first_page_of_chapter: qt_method!(fn(&self, global_idx: i32) -> bool),
    pub get_page_chapter_title: qt_method!(fn(&self, global_idx: i32) -> QString),
    pub get_page_chapter_number: qt_method!(fn(&self, global_idx: i32) -> i32),
    pub get_page_chapter_page_count: qt_method!(fn(&self, global_idx: i32) -> i32),
    pub get_page_chapter_page_idx: qt_method!(fn(&self, global_idx: i32) -> i32),
    pub update_current_page: qt_method!(fn(&mut self, global_idx: i32)),
    pub next_chapter: qt_method!(fn(&mut self) -> bool),
    pub prev_chapter: qt_method!(fn(&mut self) -> bool),
    pub jump_to_chapter: qt_method!(fn(&mut self, series_idx: i32) -> bool),

    // Rust Internal State
    pub tui_mode: bool,
    pub memory_manager: Arc<Mutex<MemoryManager>>,
    series: Arc<Mutex<Option<DirectorySeries>>>,
    chapters: Arc<Mutex<Vec<ChapterInfo>>>,
    pub archives: Arc<Mutex<HashMap<usize, Arc<CbzArchive>>>>,
    global_to_key: Arc<Mutex<Vec<PageKey>>>,
    key_to_global: Arc<Mutex<HashMap<PageKey, usize>>>,
    in_flight: Arc<Mutex<HashSet<PageKey>>>,
    first_loaded_series_idx: Arc<Mutex<usize>>,
    last_loaded_series_idx: Arc<Mutex<usize>>,
    next_chapter_id: Arc<Mutex<usize>>,
    completed_chapters: Arc<Mutex<HashSet<usize>>>,
}

fn percent_decode(s: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = s.bytes().peekable();
    while let Some(b) = chars.next() {
        if b == b'%' {
            if let (Some(h1), Some(h2)) = (chars.next(), chars.next()) {
                if let Ok(val) =
                    u8::from_str_radix(std::str::from_utf8(&[h1, h2]).unwrap_or(""), 16)
                {
                    bytes.push(val);
                    continue;
                }
                bytes.push(b'%');
                bytes.push(h1);
                bytes.push(h2);
            } else {
                bytes.push(b'%');
            }
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
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
            current_chapter_page_count: 0,
            total_pages_count: 0,
            current_chapter_idx: 0,
            total_chapters_count: 0,
            loaded_chapters_count: 0,
            initial_page_idx: 0,

            has_comic_changed: Default::default(),
            comic_title_changed: Default::default(),
            reading_mode_changed: Default::default(),
            current_chapter_name_changed: Default::default(),
            current_page_number_changed: Default::default(),
            current_chapter_page_count_changed: Default::default(),
            total_pages_count_changed: Default::default(),
            current_chapter_idx_changed: Default::default(),
            total_chapters_count_changed: Default::default(),
            loaded_chapters_count_changed: Default::default(),
            initial_page_idx_changed: Default::default(),
            jump_to_initial_page_requested: Default::default(),
            page_loaded: Default::default(),

            app_icon: Default::default(),

            open_file: Default::default(),
            toggle_reading_mode: Default::default(),
            request_pages_around: Default::default(),
            get_chapter_count: Default::default(),
            get_chapter_title: Default::default(),
            get_chapter_page_count: Default::default(),
            get_total_page_count: Default::default(),
            get_page_source: Default::default(),
            is_first_page_of_chapter: Default::default(),
            get_page_chapter_title: Default::default(),
            get_page_chapter_number: Default::default(),
            get_page_chapter_page_count: Default::default(),
            get_page_chapter_page_idx: Default::default(),
            update_current_page: Default::default(),
            next_chapter: Default::default(),
            prev_chapter: Default::default(),
            jump_to_chapter: Default::default(),

            tui_mode: false,
            memory_manager,
            series: Arc::new(Mutex::new(None)),
            chapters: Arc::new(Mutex::new(Vec::new())),
            archives: Arc::new(Mutex::new(HashMap::new())),
            global_to_key: Arc::new(Mutex::new(Vec::new())),
            key_to_global: Arc::new(Mutex::new(HashMap::new())),
            in_flight: Arc::new(Mutex::new(HashSet::new())),
            first_loaded_series_idx: Arc::new(Mutex::new(0)),
            last_loaded_series_idx: Arc::new(Mutex::new(0)),
            next_chapter_id: Arc::new(Mutex::new(0)),
            completed_chapters: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn set_tui_mode(&mut self, enabled: bool) {
        self.tui_mode = enabled;
    }

    pub fn open_file(&mut self, file_path_qstr: QString) -> bool {
        self.open_file_with_page(file_path_qstr, 1)
    }

    pub fn open_file_with_page(&mut self, file_path_qstr: QString, initial_page: i64) -> bool {
        let mut raw_path = file_path_qstr.to_string();
        if raw_path.starts_with("file://") {
            raw_path = raw_path[7..].to_string();
        }
        let decoded_path_str = percent_decode(&raw_path);

        let mut file_path = PathBuf::from(&decoded_path_str);
        if !file_path.exists() {
            file_path = PathBuf::from(&raw_path);
        }

        if !file_path.exists() {
            eprintln!("[Continuum] Error: File does not exist at {:?}", file_path);
            return false;
        }

        self.open_file_path_with_page(&file_path, initial_page)
    }

    fn open_file_path(&mut self, file_path: &Path) -> bool {
        self.open_file_path_with_page(file_path, 1)
    }

    pub fn open_file_path_with_page(&mut self, file_path: &Path, initial_page: i64) -> bool {
        let series = DirectorySeries::new(file_path);
        let archive = match CbzArchive::open(file_path) {
            Ok(a) => Arc::new(a),
            Err(e) => {
                eprintln!("[Continuum] Error opening CBZ archive: {}", e);
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
        if let Ok(mut keymap) = self.key_to_global.lock() {
            keymap.clear();
        }
        if let Ok(mut inflight) = self.in_flight.lock() {
            inflight.clear();
        }
        if let Ok(mut completed) = self.completed_chapters.lock() {
            completed.clear();
        }

        let initial_idx = series.current_index;
        *self.first_loaded_series_idx.lock().unwrap() = initial_idx;
        *self.last_loaded_series_idx.lock().unwrap() = initial_idx;
        *self.next_chapter_id.lock().unwrap() = 0;

        let total_series_count = series.dir_files.len() as i32;
        let title_name = archive.filename.clone();
        let page_count = archive.page_count();
        *self.series.lock().unwrap() = Some(series);

        self.append_archive(archive, initial_idx);

        let target_page_idx = if initial_page > 1 {
            ((initial_page - 1) as usize).min(page_count.saturating_sub(1)) as i32
        } else {
            0
        };
        self.initial_page_idx = target_page_idx;

        self.has_comic = true;
        self.comic_title = QString::from(title_name.as_str());
        self.current_chapter_name = QString::from(title_name.as_str());
        self.current_chapter_idx = (initial_idx + 1) as i32;
        self.total_chapters_count = total_series_count;
        self.loaded_chapters_count = self.chapters.lock().unwrap().len() as i32;

        let total_pages = self.global_to_key.lock().unwrap().len() as i32;
        self.total_pages_count = total_pages;
        self.current_page_number = target_page_idx + 1;
        self.current_chapter_page_count = page_count as i32;

        self.has_comic_changed();
        self.comic_title_changed();
        self.current_chapter_name_changed();
        self.current_chapter_idx_changed();
        self.total_chapters_count_changed();
        self.loaded_chapters_count_changed();
        self.total_pages_count_changed();
        self.initial_page_idx_changed();
        self.current_page_number_changed();
        self.current_chapter_page_count_changed();
        self.jump_to_initial_page_requested(target_page_idx);

        self.request_pages_around(target_page_idx);

        if page_count > 0 && target_page_idx as usize + 1 >= page_count {
            self.check_chapter_completion(0, target_page_idx as usize, page_count);
        }

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
        let start = global_idx.saturating_sub(2);
        let end = (global_idx + 6).min(total);

        // Auto-load next chapter when user is near bottom
        if global_idx + 4 >= total {
            self.next_chapter();
        }

        let archives = self.archives.lock().unwrap().clone();
        let memory_manager = self.memory_manager.clone();
        let in_flight = self.in_flight.clone();
        let key_to_global = self.key_to_global.clone();

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
                let key_to_global_clone = key_to_global.clone();

                rayon::spawn(move || {
                    if key_clone.page_idx < archive_clone.image_entries.len() {
                        let inner_name = &archive_clone.image_entries[key_clone.page_idx];
                        if let Ok(file) = std::fs::File::open(&archive_clone.path) {
                            let reader = std::io::BufReader::new(file);
                            if let Ok(mut zip) = zip::ZipArchive::new(reader) {
                                if let Ok(mut entry) = zip.by_name(inner_name) {
                                    let mut bytes = Vec::new();
                                    if std::io::Read::read_to_end(&mut entry, &mut bytes).is_ok() {
                                        if let Ok((w, h, rgba)) =
                                            CbzArchive::decode_page_bytes(&bytes)
                                        {
                                            if let Ok(page_data) =
                                                CbzArchive::create_page_data(w, h, rgba)
                                            {
                                                if let Ok(mut mem) = memory_manager_clone.lock() {
                                                    let map = key_to_global_clone.lock().unwrap();
                                                    mem.insert(
                                                        key_clone.clone(),
                                                        page_data,
                                                        idx,
                                                        &|k| map.get(k).copied().unwrap_or(0),
                                                    );
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

    pub fn is_first_page_of_chapter(&self, global_idx: i32) -> bool {
        if global_idx < 0 {
            return false;
        }
        let keys = self.global_to_key.lock().unwrap();
        if let Some(key) = keys.get(global_idx as usize) {
            key.page_idx == 0
        } else {
            false
        }
    }

    pub fn get_page_chapter_title(&self, global_idx: i32) -> QString {
        if global_idx < 0 {
            return QString::from("");
        }
        let keys = self.global_to_key.lock().unwrap();
        if let Some(key) = keys.get(global_idx as usize) {
            let chaps = self.chapters.lock().unwrap();
            if let Some(chap) = chaps.get(key.chapter_idx) {
                return QString::from(chap.filename.as_str());
            }
        }
        QString::from("")
    }

    pub fn get_page_chapter_number(&self, global_idx: i32) -> i32 {
        if global_idx < 0 {
            return 1;
        }
        let keys = self.global_to_key.lock().unwrap();
        if let Some(key) = keys.get(global_idx as usize) {
            let chaps = self.chapters.lock().unwrap();
            if let Some(chap) = chaps.get(key.chapter_idx) {
                return (chap.series_idx + 1) as i32;
            }
        }
        1
    }

    pub fn get_page_chapter_page_count(&self, global_idx: i32) -> i32 {
        if global_idx < 0 {
            return 0;
        }
        let keys = self.global_to_key.lock().unwrap();
        if let Some(key) = keys.get(global_idx as usize) {
            let chaps = self.chapters.lock().unwrap();
            if let Some(chap) = chaps.get(key.chapter_idx) {
                return chap.page_count as i32;
            }
        }
        0
    }

    pub fn get_page_chapter_page_idx(&self, global_idx: i32) -> i32 {
        if global_idx < 0 {
            return 1;
        }
        let keys = self.global_to_key.lock().unwrap();
        if let Some(key) = keys.get(global_idx as usize) {
            return (key.page_idx + 1) as i32;
        }
        1
    }

    pub fn update_current_page(&mut self, global_idx: i32) {
        if global_idx < 0 {
            return;
        }
        let g_idx = global_idx as usize;
        let info = {
            let keys = match self.global_to_key.lock() {
                Ok(k) => k,
                Err(_) => return,
            };
            let key = match keys.get(g_idx) {
                Some(k) => k.clone(),
                None => return,
            };
            let chaps = match self.chapters.lock() {
                Ok(c) => c,
                Err(_) => return,
            };
            let chap = match chaps.get(key.chapter_idx) {
                Some(c) => c.clone(),
                None => return,
            };
            (key, chap)
        };

        let (key, chap) = info;
        let new_chap_idx = (chap.series_idx + 1) as i32;
        let new_page_num = (key.page_idx + 1) as i32;
        let new_page_count = chap.page_count as i32;

        if self.current_chapter_idx != new_chap_idx {
            self.current_chapter_idx = new_chap_idx;
            self.current_chapter_idx_changed();
            self.current_chapter_name = QString::from(chap.filename.as_str());
            self.current_chapter_name_changed();
        }
        if self.current_page_number != new_page_num {
            self.current_page_number = new_page_num;
            self.current_page_number_changed();
        }
        if self.current_chapter_page_count != new_page_count {
            self.current_chapter_page_count = new_page_count;
            self.current_chapter_page_count_changed();
        }

        self.check_chapter_completion(key.chapter_idx, key.page_idx, chap.page_count);
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
                    self.loaded_chapters_count = self.chapters.lock().unwrap().len() as i32;
                    self.loaded_chapters_count_changed();
                    self.total_pages_count_changed();
                    return true;
                }
            }
        }
        false
    }

    pub fn prev_chapter(&mut self) -> bool {
        let current_chap = (self.current_chapter_idx - 1).max(0) as usize;
        if current_chap > 0 {
            self.jump_to_chapter((current_chap - 1) as i32)
        } else {
            false
        }
    }

    pub fn jump_to_chapter(&mut self, series_idx: i32) -> bool {
        if series_idx < 0 {
            return false;
        }
        let target_idx = series_idx as usize;
        let series_opt = self.series.lock().unwrap().clone();

        if let Some(series) = series_opt {
            if target_idx < series.dir_files.len() {
                let target_path = series.dir_files[target_idx].clone();
                return self.open_file_path(&target_path);
            }
        }
        false
    }

    pub fn get_app_icon(&self) -> QString {
        QString::from("qrc:/continuum.png")
    }

    pub fn get_page_source(&self, global_idx: i32) -> QString {
        let keys = self.global_to_key.lock().unwrap();
        if let Some(key) = keys.get(global_idx as usize) {
            QString::from(format!("image://cbz/{}/{}", key.chapter_idx, key.page_idx))
        } else {
            QString::from("")
        }
    }

    fn append_archive(&mut self, archive: Arc<CbzArchive>, series_idx: usize) {
        let chapter_id = *self.next_chapter_id.lock().unwrap();
        *self.next_chapter_id.lock().unwrap() += 1;

        let total_pages = archive.page_count();

        let mut keys_lock = self.global_to_key.lock().unwrap();
        let mut keymap_lock = self.key_to_global.lock().unwrap();

        for page_idx in 0..total_pages {
            let key = PageKey {
                chapter_idx: chapter_id,
                page_idx,
            };
            let global_idx = keys_lock.len();
            keymap_lock.insert(key.clone(), global_idx);
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

    fn check_chapter_completion(
        &mut self,
        current_internal_chap_idx: usize,
        page_idx: usize,
        total_pages: usize,
    ) {
        let chaps = match self.chapters.lock() {
            Ok(c) => c.clone(),
            Err(_) => return,
        };

        // 1. Any chapter before the current one in the continuous stream was completed
        for chap in chaps.iter() {
            if chap.chapter_id < current_internal_chap_idx {
                self.mark_chapter_completed(chap);
            }
        }

        // 2. If user is at or past the last page of the current chapter
        if total_pages > 0 && page_idx + 1 >= total_pages {
            if let Some(chap) = chaps
                .iter()
                .find(|c| c.chapter_id == current_internal_chap_idx)
            {
                self.mark_chapter_completed(chap);
            }
        }
    }

    fn mark_chapter_completed(&mut self, chap: &ChapterInfo) {
        let mut completed_set = self.completed_chapters.lock().unwrap();
        completed_set.insert(chap.series_idx);
    }

    pub fn is_current_chapter_completed(&self) -> bool {
        let chaps = self.chapters.lock().unwrap();
        if self.current_chapter_idx > 0 {
            let series_idx = (self.current_chapter_idx - 1) as usize;
            if self
                .completed_chapters
                .lock()
                .unwrap()
                .contains(&series_idx)
            {
                return true;
            }
            if let Some(chap) = chaps.iter().find(|c| c.series_idx == series_idx) {
                if chap.page_count > 0 && self.current_page_number as usize >= chap.page_count {
                    return true;
                }
            }
        }
        false
    }

    pub fn emit_exit_payload(&self) {
        let chaps = match self.chapters.lock() {
            Ok(c) => c.clone(),
            Err(_) => return,
        };
        if chaps.is_empty() && !self.has_comic {
            return;
        }

        let current_chap = if self.current_chapter_idx > 0 {
            let series_idx = (self.current_chapter_idx - 1) as usize;
            chaps.iter().find(|c| c.series_idx == series_idx).cloned()
        } else {
            chaps.first().cloned()
        };

        let is_completed = self.is_current_chapter_completed();
        let last_page = self.current_page_number as i64;
        let file_path = current_chap
            .as_ref()
            .map(|c| c.archive_path.to_string_lossy().to_string())
            .unwrap_or_default();
        let chapter_name = current_chap
            .as_ref()
            .map(|c| c.filename.clone())
            .unwrap_or_default();

        let completed_set = self.completed_chapters.lock().unwrap();

        let completed_chapters_paths: Vec<String> = chaps
            .iter()
            .filter(|c| {
                completed_set.contains(&c.series_idx)
                    || (is_completed
                        && current_chap.as_ref().map(|cc| cc.series_idx) == Some(c.series_idx))
            })
            .map(|c| c.archive_path.to_string_lossy().to_string())
            .collect();

        let completed_filenames: Vec<String> = chaps
            .iter()
            .filter(|c| {
                completed_set.contains(&c.series_idx)
                    || (is_completed
                        && current_chap.as_ref().map(|cc| cc.series_idx) == Some(c.series_idx))
            })
            .map(|c| c.filename.clone())
            .collect();

        let completed_indices: Vec<i64> = chaps
            .iter()
            .filter(|c| {
                completed_set.contains(&c.series_idx)
                    || (is_completed
                        && current_chap.as_ref().map(|cc| cc.series_idx) == Some(c.series_idx))
            })
            .map(|c| (c.series_idx + 1) as i64)
            .collect();

        let exit_payload = serde_json::json!({
            "last_page": last_page,
            "completed": is_completed,
            "completed_chapters": completed_chapters_paths,
            "completed_filenames": completed_filenames,
            "completed_chapter_indices": completed_indices,
            "chapter_idx": self.current_chapter_idx,
            "chapter_name": chapter_name,
            "file_path": file_path
        });

        println!("{}", exit_payload);
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
}
