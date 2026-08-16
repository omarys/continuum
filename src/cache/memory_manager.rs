use crate::cbz::archive::LoadedPageData;
use std::collections::HashMap;

pub const MIN_CACHE_BYTES: usize = 128 * 1024 * 1024; // 128 MB minimum loaded
pub const MAX_CACHE_BYTES: usize = 512 * 1024 * 1024; // 512 MB maximum allowed

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PageKey {
    pub chapter_idx: usize,
    pub page_idx: usize,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct PageCacheEntry {
    pub key: PageKey,
    pub data: LoadedPageData,
    pub last_accessed_idx: usize,
}

#[derive(Default)]
pub struct MemoryManager {
    entries: HashMap<PageKey, PageCacheEntry>,
    total_bytes: usize,
    access_counter: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    #[allow(dead_code)]
    pub fn loaded_count(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&mut self, key: &PageKey) -> Option<LoadedPageData> {
        if let Some(entry) = self.entries.get_mut(key) {
            self.access_counter += 1;
            entry.last_accessed_idx = self.access_counter;
            Some(entry.data.clone())
        } else {
            None
        }
    }

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
            self.total_bytes = self.total_bytes.saturating_sub(old.data.byte_size);
        }

        self.access_counter += 1;
        self.total_bytes += data.byte_size;
        self.entries.insert(
            key.clone(),
            PageCacheEntry {
                key,
                data,
                last_accessed_idx: self.access_counter,
            },
        );

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

        let mut candidates: Vec<(PageKey, usize, usize)> = self
            .entries
            .iter()
            .map(|(key, entry)| {
                let global_idx = page_to_global_map(key);
                let distance = global_idx.abs_diff(current_visible_global_idx);
                (key.clone(), distance, entry.data.byte_size)
            })
            .collect();

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
                self.total_bytes = self.total_bytes.saturating_sub(removed.data.byte_size);
                evicted_keys.push(key);
            }
        }

        evicted_keys
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, key: &PageKey) {
        if let Some(entry) = self.entries.remove(key) {
            self.total_bytes = self.total_bytes.saturating_sub(entry.data.byte_size);
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
    }
}
