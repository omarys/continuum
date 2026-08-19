use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub const MAX_ARCHIVE_ENTRIES: usize = 2_000;
pub const MAX_PAGE_FILE_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB max compressed/raw entry size
pub const MAX_IMAGE_DIMENSION: u32 = 16_384; // 16,384px width / height max
pub const MAX_IMAGE_ALLOC_BYTES: u64 = 256 * 1024 * 1024; // 256 MiB max pixel allocation per image

pub fn is_valid_zip_file<P: AsRef<Path>>(path: P) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut header = [0u8; 4];
    if std::io::Read::read_exact(&mut file, &mut header).is_err() {
        return false;
    }
    header == [0x50, 0x4B, 0x03, 0x04]
        || header == [0x50, 0x4B, 0x05, 0x06]
        || header == [0x50, 0x4B, 0x07, 0x08]
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
        if !is_valid_zip_file(&path_buf) {
            return Err(
                "File is not a valid ZIP/CBZ archive (missing PK magic header)".to_string(),
            );
        }

        let file = File::open(&path_buf).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);
        let mut zip =
            ZipArchive::new(reader).map_err(|e| format!("Failed to read ZIP archive: {}", e))?;

        if zip.len() > MAX_ARCHIVE_ENTRIES {
            return Err(format!(
                "Archive contains too many entries ({} > maximum {})",
                zip.len(),
                MAX_ARCHIVE_ENTRIES
            ));
        }

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
        if bytes.len() as u64 > MAX_PAGE_FILE_SIZE {
            return Err(format!(
                "Image byte payload exceeds maximum allowed size ({} > {} bytes)",
                bytes.len(),
                MAX_PAGE_FILE_SIZE
            ));
        }

        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
        limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
        limits.max_alloc = Some(MAX_IMAGE_ALLOC_BYTES);

        let mut reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| format!("Failed to recognize image format: {}", e))?;
        reader.limits(limits);

        let img = reader
            .decode()
            .map_err(|e| format!("Failed to decode image: {}", e))?;
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
}

#[derive(Clone, Debug)]
pub struct DirectorySeries {
    pub dir_files: Vec<PathBuf>,
    pub current_index: usize,
}

impl DirectorySeries {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        let current_path = file_path.as_ref().to_path_buf();
        let mut dir_files: Vec<PathBuf> = current_path
            .parent()
            .and_then(|p| std::fs::read_dir(p).ok())
            .into_iter()
            .flat_map(|entries| entries.flatten())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension().is_some_and(|ext| {
                        let e = ext.to_string_lossy().to_lowercase();
                        e == "cbz" || e == "zip"
                    })
                    && is_valid_zip_file(p)
            })
            .collect();

        dir_files.sort_by(|a, b| {
            natord::compare(
                &a.file_name().unwrap_or_default().to_string_lossy(),
                &b.file_name().unwrap_or_default().to_string_lossy(),
            )
        });

        let current_index = dir_files
            .iter()
            .position(|p| p == &current_path || p.file_name() == current_path.file_name())
            .unwrap_or(0);

        Self {
            dir_files,
            current_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_sample_comic() {
        let sample = PathBuf::from("sample_comics/Solo_Leveling_Ch01.cbz");
        if sample.exists() {
            let archive = CbzArchive::open(&sample).expect("Should open sample comic");
            assert!(archive.page_count() > 0);
            assert_eq!(archive.filename, "Solo_Leveling_Ch01.cbz");
        }
    }

    #[test]
    fn test_is_valid_zip_file() {
        let sample = PathBuf::from("sample_comics/Solo_Leveling_Ch01.cbz");
        if sample.exists() {
            assert!(is_valid_zip_file(&sample));
        }
        let bad = PathBuf::from("Cargo.toml");
        assert!(!is_valid_zip_file(&bad));
    }

    #[test]
    fn test_directory_series_sorting() {
        let sample1 = PathBuf::from("sample_comics/Solo_Leveling_Ch01.cbz");
        let sample2 = PathBuf::from("sample_comics/Solo_Leveling_Ch02.cbz");
        if sample1.exists() && sample2.exists() {
            let series1 = DirectorySeries::new(&sample1);
            let series2 = DirectorySeries::new(&sample2);
            assert_eq!(series1.dir_files, series2.dir_files);
            assert_eq!(series1.current_index, 0);
            assert_eq!(series2.current_index, 1);
        }
    }

    #[test]
    fn test_decode_page_bytes_invalid() {
        let bad_bytes = vec![0u8; 16];
        assert!(CbzArchive::decode_page_bytes(&bad_bytes).is_err());
    }

    #[test]
    fn test_decode_page_bytes_payload_too_large() {
        let fake_large_payload = vec![0u8; (MAX_PAGE_FILE_SIZE as usize) + 1];
        let res = CbzArchive::decode_page_bytes(&fake_large_payload);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("exceeds maximum allowed size"));
    }
}
