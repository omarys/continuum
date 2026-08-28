use gdk4::Texture;
use std::fs::File;
use std::io::Cursor;
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

#[derive(Clone)]
pub struct LoadedPageData {
    pub texture: Texture,
    pub width: u32,
    pub height: u32,
    pub byte_size: usize,
    pub _bytes: glib::Bytes,
}

pub struct DecodedImagePayload {
    pub width: u32,
    pub height: u32,
    pub rgba_bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct CbzArchive {
    pub path: PathBuf,
    pub filename: String,
    pub image_entries: Vec<String>,
    pub data: std::sync::Arc<Vec<u8>>,
}

impl CbzArchive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_buf = path.as_ref().to_path_buf();
        if !is_valid_zip_file(&path_buf) {
            return Err(
                "File is not a valid ZIP/CBZ archive (missing PK magic header)".to_string(),
            );
        }

        let raw_bytes =
            std::fs::read(&path_buf).map_err(|e| format!("Failed to read file: {}", e))?;
        let data = std::sync::Arc::new(raw_bytes);
        let reader = Cursor::new(data.as_slice());
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
            data,
        })
    }

    pub fn extract_entry_bytes(data: &[u8], entry_name: &str) -> Result<Vec<u8>, String> {
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).map_err(|e| e.to_string())?;
        let mut entry = zip.by_name(entry_name).map_err(|e| e.to_string())?;

        // Reject spoofed header sizes
        if entry.size() > MAX_PAGE_FILE_SIZE {
            return Err(format!(
                "Entry '{}' declares size exceeding limit ({} > {} bytes)",
                entry_name,
                entry.size(),
                MAX_PAGE_FILE_SIZE
            ));
        }

        // Bound initial capacity allocation
        let initial_cap = (entry.size() as usize).min(MAX_PAGE_FILE_SIZE as usize);
        let mut buf = Vec::with_capacity(initial_cap);

        // Bound decompression stream to prevent zip-bomb expansion
        let mut limited_reader = std::io::Read::take(&mut entry, MAX_PAGE_FILE_SIZE + 1);
        std::io::Read::read_to_end(&mut limited_reader, &mut buf).map_err(|e| e.to_string())?;

        if buf.len() as u64 > MAX_PAGE_FILE_SIZE {
            return Err(format!(
                "Entry '{}' decompressed beyond maximum limit ({} bytes)",
                entry_name, MAX_PAGE_FILE_SIZE
            ));
        }

        Ok(buf)
    }

    pub fn page_count(&self) -> usize {
        self.image_entries.len()
    }

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

    pub fn create_texture(
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
        let bytes = glib::Bytes::from_owned(rgba_bytes);
        let pixbuf = gdk_pixbuf::Pixbuf::from_bytes(
            &bytes,
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            width as i32,
            height as i32,
            width as i32 * 4,
        );
        let texture = Texture::for_pixbuf(&pixbuf);
        Ok(LoadedPageData {
            texture,
            width,
            height,
            byte_size,
            _bytes: bytes,
        })
    }
}

/// Attempts to extract the canonical chapter/episode number from a comic filename.
/// Handles formats like:
/// - "[0047]_Chapter_0_Sep_7_2024.cbz" -> 0.0
/// - "[0000]_Chapter_40.6_Apr_4.cbz" -> 40.6
/// - "[0208]_Episode_1_Sep_7_2024.cbz" -> 1.0
/// - "Official_Chapter 70_95fbe1.cbz" -> 70.0
/// - "Solo_Leveling_Ch01.cbz" -> 1.0
pub fn extract_chapter_number(filename: &str) -> Option<f64> {
    let stem = Path::new(filename)
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| filename.into());
    let lower = stem.to_lowercase();

    // Look for explicit keywords: chapter, episode, chap, ep, ch, vol, v, c
    let keywords = ["chapter", "episode", "chap", "ep", "ch", "vol", "v", "c"];
    for kw in &keywords {
        if let Some(pos) = lower.find(kw) {
            let after = &lower[pos + kw.len()..];
            // Skip separators like ' ', '_', '-', '.', ':'
            let trimmed = after.trim_start_matches(|c: char| {
                c.is_whitespace() || c == '_' || c == '-' || c == '.' || c == ':'
            });
            if let Some(num) = parse_leading_number(trimmed) {
                return Some(num);
            }
        }
    }

    None
}

fn parse_leading_number(s: &str) -> Option<f64> {
    let mut end = 0;
    let mut has_dot = false;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() {
            end = i + 1;
        } else if c == '.' && !has_dot {
            if s[i + 1..]
                .chars()
                .next()
                .is_some_and(|next_c| next_c.is_ascii_digit())
            {
                has_dot = true;
                end = i + 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if end > 0 {
        s[..end].parse::<f64>().ok()
    } else {
        None
    }
}

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
            let name_a = a.file_name().unwrap_or_default().to_string_lossy();
            let name_b = b.file_name().unwrap_or_default().to_string_lossy();
            let num_a = extract_chapter_number(&name_a);
            let num_b = extract_chapter_number(&name_b);

            match (num_a, num_b) {
                (Some(na), Some(nb)) => na
                    .partial_cmp(&nb)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| natord::compare(&name_a, &name_b)),
                _ => natord::compare(&name_a, &name_b),
            }
        });

        let current_index = dir_files
            .iter()
            .position(|p| p == &current_path)
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
    fn test_extract_chapter_number() {
        assert_eq!(
            extract_chapter_number("[0047]_Chapter_0_Sep_7_2024.cbz"),
            Some(0.0)
        );
        assert_eq!(
            extract_chapter_number("[0046]_Chapter_1_Sep_7_2024.cbz"),
            Some(1.0)
        );
        assert_eq!(
            extract_chapter_number("[0000]_Chapter_40.6_Apr_4.cbz"),
            Some(40.6)
        );
        assert_eq!(
            extract_chapter_number("[0208]_Episode_1_Sep_7_2024.cbz"),
            Some(1.0)
        );
        assert_eq!(
            extract_chapter_number("Official_Chapter 70_95fbe1.cbz"),
            Some(70.0)
        );
        assert_eq!(extract_chapter_number("Solo_Leveling_Ch01.cbz"), Some(1.0));
        assert_eq!(extract_chapter_number("Random_Book.cbz"), None);
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
