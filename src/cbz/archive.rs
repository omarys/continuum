use gdk4::Texture;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

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
}
