use crate::cbz::archive::LoadedPageData;
use gdk4::Texture;
use std::collections::HashMap;

pub const MIN_CACHE_BYTES: usize = 192 * 1024 * 1024; // 192 MB minimum loaded
pub const MAX_CACHE_BYTES: usize = 512 * 1024 * 1024; // 512 MB maximum allowed

/// Share of currently-available RAM we are willing to hold as decoded textures.
/// Sampled when a comic is opened, i.e. before this cache has filled, so the
/// number reflects the machine rather than our own footprint. On a 7.5 GB box
/// this keeps resident textures near ~475 MB instead of the old fixed 1 GB.
const AVAILABLE_RAM_SHARE: usize = 8;
const ASSUMED_AVAILABLE_RAM: usize = MAX_CACHE_BYTES * AVAILABLE_RAM_SHARE;

/// `MemAvailable` from /proc/meminfo: the kernel's estimate of what a new
/// allocation can claim without swapping. Linux-only; elsewhere we fall back to
/// the ceiling.
fn available_ram_bytes() -> Option<usize> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kb = meminfo
        .lines()
        .find_map(|line| line.strip_prefix("MemAvailable:"))?
        .split_whitespace()
        .next()?
        .parse::<usize>()
        .ok()?;
    Some(kb.saturating_mul(1024))
}

fn detect_cache_budget() -> usize {
    let available = available_ram_bytes().unwrap_or(ASSUMED_AVAILABLE_RAM);
    (available / AVAILABLE_RAM_SHARE).clamp(MIN_CACHE_BYTES, MAX_CACHE_BYTES)
}

/// Loaded page keys ordered furthest-from-the-viewport first, so eviction drops
/// the pages a reader is least likely to scroll back to.
fn eviction_order<'a>(
    loaded: impl Iterator<Item = &'a PageKey>,
    global_to_key: &[PageKey],
    current_visible_global_idx: usize,
) -> Vec<PageKey> {
    // Reverse index built once: probing the page list per entry made eviction
    // quadratic, and it runs on the GTK main thread mid-scroll.
    let positions: HashMap<&PageKey, usize> = global_to_key
        .iter()
        .enumerate()
        .map(|(idx, key)| (key, idx))
        .collect();

    let mut candidates: Vec<(PageKey, usize)> = loaded
        .map(|key| {
            let global_idx = positions
                .get(key)
                .copied()
                .unwrap_or(current_visible_global_idx);
            (key.clone(), global_idx.abs_diff(current_visible_global_idx))
        })
        .collect();

    candidates.sort_by_key(|(_, distance)| std::cmp::Reverse(*distance));
    candidates.into_iter().map(|(key, _)| key).collect()
}

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
    budget_bytes: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            total_bytes: 0,
            access_counter: 0,
            budget_bytes: detect_cache_budget(),
        }
    }
    pub fn insert(
        &mut self,
        key: PageKey,
        data: LoadedPageData,
        current_visible_global_idx: usize,
        global_to_key: &[PageKey],
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

        // Perform memory eviction if over the resident-texture budget
        self.evict_if_needed(current_visible_global_idx, global_to_key)
    }

    pub fn evict_if_needed(
        &mut self,
        current_visible_global_idx: usize,
        global_to_key: &[PageKey],
    ) -> Vec<PageKey> {
        let mut evicted_keys = Vec::new();

        if self.total_bytes <= self.budget_bytes {
            return evicted_keys;
        }

        for key in eviction_order(
            self.entries.keys(),
            global_to_key,
            current_visible_global_idx,
        ) {
            if self.total_bytes <= self.budget_bytes {
                break;
            }

            if let Some(removed) = self.entries.remove(&key) {
                self.total_bytes = self.total_bytes.saturating_sub(removed.byte_size);
                evicted_keys.push(key);
            }
        }

        evicted_keys
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_initial_state() {
        let mgr = MemoryManager::new();
        assert_eq!(mgr.total_bytes, 0);
        assert!(mgr.entries.is_empty());
    }

    #[test]
    fn test_memory_manager_clear() {
        let mut mgr = MemoryManager::new();
        mgr.total_bytes = 500;
        mgr.clear();
        assert_eq!(mgr.total_bytes, 0);
        assert!(mgr.entries.is_empty());
    }

    #[test]
    fn test_eviction_under_limit_does_nothing() {
        let mut mgr = MemoryManager::new();
        mgr.total_bytes = 100 * 1024 * 1024; // 100MB (under the budget)
        let evicted = mgr.evict_if_needed(0, &[]);
        assert!(evicted.is_empty());
    }

    #[test]
    fn test_cache_budget_stays_within_bounds() {
        let budget = detect_cache_budget();
        assert!(
            (MIN_CACHE_BYTES..=MAX_CACHE_BYTES).contains(&budget),
            "budget {budget} outside [{MIN_CACHE_BYTES}, {MAX_CACHE_BYTES}]"
        );
    }

    #[test]
    fn test_eviction_order_drops_furthest_pages_first() {
        let globals: Vec<PageKey> = (0..10)
            .map(|page_idx| PageKey {
                chapter_idx: 0,
                page_idx,
            })
            .collect();
        let loaded: Vec<PageKey> = [1usize, 3, 5, 8]
            .iter()
            .map(|&page_idx| PageKey {
                chapter_idx: 0,
                page_idx,
            })
            .collect();

        let order: Vec<usize> = eviction_order(loaded.iter(), &globals, 4)
            .iter()
            .map(|key| key.page_idx)
            .collect();

        // 8 is furthest from page 4, then 1; 3 and 5 tie and keep their order.
        assert_eq!(order, vec![8, 1, 3, 5]);
    }

    #[test]
    fn test_eviction_order_handles_pages_missing_from_global_list() {
        let globals = vec![PageKey {
            chapter_idx: 0,
            page_idx: 0,
        }];
        let orphan = PageKey {
            chapter_idx: 7,
            page_idx: 7,
        };
        let order = eviction_order([&orphan].into_iter(), &globals, 3);
        assert_eq!(order, vec![orphan]);
    }
}
