use crate::cache::PageKey;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CbzPageInfo {
    pub archive_path: PathBuf,
    pub archive_name: String,
    pub inner_filename: String,
    pub page_index: usize,
    pub total_pages: usize,
}

#[derive(Clone, Debug)]
pub struct LoadedPageData {
    pub width: u32,
    pub height: u32,
    pub rgba_bytes: Vec<u8>,
    pub byte_size: usize,
}

#[allow(dead_code)]
pub struct DecodedImagePayload {
    pub key: PageKey,
    pub current_global_idx: usize,
    pub width: u32,
    pub height: u32,
    pub rgba_bytes: Vec<u8>,
}

pub struct CbzArchive {
    pub path: PathBuf,
    pub filename: String,
    pub image_entries: Vec<String>,
}

impl CbzArchive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);
        let mut zip =
            ZipArchive::new(reader).map_err(|e| format!("Failed to read ZIP archive: {}", e))?;

        let mut image_entries = Vec::new();
        for i in 0..zip.len() {
            if let Ok(entry) = zip.by_index(i) {
                let name = entry.name().to_string();
                let lower = name.to_lowercase();
                if !entry.is_dir()
                    && (lower.ends_with(".jpg")
                        || lower.ends_with(".jpeg")
                        || lower.ends_with(".png")
                        || lower.ends_with(".webp")
                        || lower.ends_with(".gif")
                        || lower.ends_with(".avif")
                        || lower.ends_with(".bmp"))
                {
                    image_entries.push(name);
                }
            }
        }

        image_entries.sort_by(|a, b| natord::compare(a, b));

        let filename = path_buf
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown.cbz".to_string());

        Ok(Self {
            path: path_buf,
            filename,
            image_entries,
        })
    }

    pub fn page_count(&self) -> usize {
        self.image_entries.len()
    }

    #[allow(dead_code)]
    pub fn get_dimensions(&self, _index: usize) -> (u32, u32) {
        (800, 1400)
    }

    pub fn decode_page_bytes(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
        let img =
            image::load_from_memory(bytes).map_err(|e| format!("Failed to decode image: {}", e))?;
        let rgba = img.into_rgba8();
        let (width, height) = rgba.dimensions();
        let raw_rgba_bytes = rgba.into_raw();
        Ok((width, height, raw_rgba_bytes))
    }

    pub fn create_page_data(
        width: u32,
        height: u32,
        rgba_bytes: Vec<u8>,
    ) -> Result<LoadedPageData, String> {
        let byte_size = (width * height * 4) as usize;
        if rgba_bytes.len() != byte_size {
            return Err(format!(
                "RGBA byte count mismatch: expected {} but got {}",
                byte_size,
                rgba_bytes.len()
            ));
        }
        Ok(LoadedPageData {
            width,
            height,
            rgba_bytes,
            byte_size,
        })
    }

    #[allow(dead_code)]
    pub fn page_info(&self, index: usize) -> Option<CbzPageInfo> {
        if index >= self.image_entries.len() {
            return None;
        }
        Some(CbzPageInfo {
            archive_path: self.path.clone(),
            archive_name: self.filename.clone(),
            inner_filename: self.image_entries[index].clone(),
            page_index: index,
            total_pages: self.image_entries.len(),
        })
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct DirectorySeries {
    pub current_path: PathBuf,
    pub dir_files: Vec<PathBuf>,
    pub current_index: usize,
}

impl DirectorySeries {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        let current_path = file_path.as_ref().to_path_buf();
        let mut dir_files = Vec::new();

        if let Some(parent) = current_path.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy().to_lowercase();
                            if ext_str == "cbz" || ext_str == "zip" {
                                dir_files.push(path);
                            }
                        }
                    }
                }
            }
        }

        dir_files.sort_by(|a, b| {
            let name_a = a
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default();
            let name_b = b
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default();
            natord::compare(&name_a, &name_b)
        });

        let current_index = dir_files
            .iter()
            .position(|p| p == &current_path || p.file_name() == current_path.file_name())
            .unwrap_or(0);

        Self {
            current_path,
            dir_files,
            current_index,
        }
    }

    #[allow(dead_code)]
    pub fn next_path(&self, offset_from_current: usize) -> Option<PathBuf> {
        let idx = self.current_index + offset_from_current;
        if idx < self.dir_files.len() {
            Some(self.dir_files[idx].clone())
        } else {
            None
        }
    }
}
