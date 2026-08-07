use crate::cbz::archive::LoadedPageData;
use gdk4::Texture;
use std::collections::HashMap;

pub const MIN_CACHE_BYTES: usize = 256 * 1024 * 1024; // 256 MB minimum loaded
pub const MAX_CACHE_BYTES: usize = 1024 * 1024 * 1024; // 1024 MB maximum allowed

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PageKey {
    pub chapter_idx: usize,
    pub page_idx: usize,
}

#[allow(dead_code)]
pub struct PageCacheEntry {
    pub key: PageKey,
    pub texture: Texture,
    pub width: u32,
    pub height: u32,
    pub byte_size: usize,
    pub last_accessed_idx: usize,
}

pub struct MemoryManager {
    entries: HashMap<PageKey, PageCacheEntry>,
    total_bytes: usize,
    access_counter: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            total_bytes: 0,
            access_counter: 0,
        }
    }

    #[allow(dead_code)]
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    #[allow(dead_code)]
    pub fn loaded_count(&self) -> usize {
        self.entries.len()
    }

    #[allow(dead_code)]
    pub fn get(&mut self, key: &PageKey) -> Option<Texture> {
        if let Some(entry) = self.entries.get_mut(key) {
            self.access_counter += 1;
            entry.last_accessed_idx = self.access_counter;
            Some(entry.texture.clone())
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn contains(&self, key: &PageKey) -> bool {
        self.entries.contains_key(key)
    }

    pub fn insert(
        &mut self,
        key: PageKey,
        data: LoadedPageData,
        current_visible_global_idx: usize,
        page_to_global_map: &dyn Fn(&PageKey) -> usize,
    ) -> Vec<PageKey> {
        if let Some(old) = self.entries.remove(&key) {
            self.total_bytes = self.total_bytes.saturating_sub(old.byte_size);
        }

        self.access_counter += 1;
        self.total_bytes += data.byte_size;
        self.entries.insert(
            key.clone(),
            PageCacheEntry {
                key,
                texture: data.texture,
                width: data.width,
                height: data.height,
                byte_size: data.byte_size,
                last_accessed_idx: self.access_counter,
            },
        );

        // Perform memory eviction if over MAX_CACHE_BYTES (1024MB)
        self.evict_if_needed(current_visible_global_idx, page_to_global_map)
    }

    pub fn evict_if_needed(
        &mut self,
        current_visible_global_idx: usize,
        page_to_global_map: &dyn Fn(&PageKey) -> usize,
    ) -> Vec<PageKey> {
        let mut evicted_keys = Vec::new();

        if self.total_bytes <= MAX_CACHE_BYTES {
            return evicted_keys;
        }

        // Sort loaded pages by distance to currently visible global index (furthest first)
        let mut candidates: Vec<(PageKey, usize, usize)> = self
            .entries
            .iter()
            .map(|(key, entry)| {
                let global_idx = page_to_global_map(key);
                let distance = global_idx.abs_diff(current_visible_global_idx);
                (key.clone(), distance, entry.byte_size)
            })
            .collect();

        // Sort descending by distance (furthest pages are evicted first)
        candidates.sort_by_key(|c| std::cmp::Reverse(c.1));

        for (key, _dist, size) in candidates {
            if self.total_bytes <= MAX_CACHE_BYTES {
                break;
            }
            if self.total_bytes.saturating_sub(size) < MIN_CACHE_BYTES
                && self.total_bytes <= MAX_CACHE_BYTES
            {
                break;
            }

            if let Some(removed) = self.entries.remove(&key) {
                self.total_bytes = self.total_bytes.saturating_sub(removed.byte_size);
                evicted_keys.push(key);
            }
        }

        evicted_keys
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, key: &PageKey) {
        if let Some(entry) = self.entries.remove(key) {
            self.total_bytes = self.total_bytes.saturating_sub(entry.byte_size);
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
    }
}
