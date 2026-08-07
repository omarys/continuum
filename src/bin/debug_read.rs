use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use zip::ZipArchive;

fn main() {
    let path = PathBuf::from("/home/omary/Documents/Books/TGED/Official_Chapter 70_95fbe1.cbz");
    println!("Opening archive: {:?}", path);
    let file = File::open(&path).expect("Failed to open file");
    let reader = BufReader::new(file);
    let mut zip = ZipArchive::new(reader).expect("Failed to read ZIP");
    println!("Zip entry count: {}", zip.len());

    let mut entries = Vec::new();
    for i in 0..zip.len() {
        if let Ok(entry) = zip.by_index(i) {
            let name = entry.name().to_string();
            let lower = name.to_lowercase();
            if !entry.is_dir()
                && (lower.ends_with(".png")
                    || lower.ends_with(".jpg")
                    || lower.ends_with(".jpeg")
                    || lower.ends_with(".webp"))
            {
                entries.push(name);
            }
        }
    }
    entries.sort_by(|a, b| natord::compare(a, b));
    println!("Found {} image entries", entries.len());
    if let Some(first) = entries.first() {
        println!("First entry name: '{}'", first);
        let mut entry = zip.by_name(first).expect("Failed to get by_name");
        let mut buf = Vec::with_capacity(entry.size() as usize);
        std::io::Read::read_to_end(&mut entry, &mut buf).expect("Failed to read_to_end");
        println!("Read {} bytes from entry {}", buf.len(), first);

        match image::load_from_memory(&buf) {
            Ok(img) => {
                let rgba = img.into_rgba8();
                println!(
                    "Decoded RGBA dimensions: {}x{}, raw bytes: {}",
                    rgba.width(),
                    rgba.height(),
                    rgba.as_raw().len()
                );
            }
            Err(e) => {
                println!("Image decode error: {}", e);
            }
        }
    }
}
