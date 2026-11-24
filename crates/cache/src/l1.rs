use crate::config::{DecodedImage, ImageDimensions};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct L1Key {
    pub url: String,
    pub dims: Option<ImageDimensions>,
}

struct L1Entry {
    image: Arc<DecodedImage>,
    size_bytes: usize,
    last_accessed: Instant,
}

pub struct MemoryLruCache {
    capacity_bytes: usize,
    current_bytes: usize,
    entries: HashMap<L1Key, L1Entry>,
}

impl MemoryLruCache {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            capacity_bytes,
            current_bytes: 0,
            entries: HashMap::new(),
        }
    }

    pub fn get(&mut self, key: &L1Key) -> Option<Arc<DecodedImage>> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed = Instant::now();
            Some(entry.image.clone())
        } else {
            None
        }
    }

    pub fn put(&mut self, key: L1Key, image: Arc<DecodedImage>) {
        let size = image.byte_size() + std::mem::size_of::<L1Key>() + key.url.len();
        if size > self.capacity_bytes {
            // Cannot cache item exceeding total capacity
            return;
        }

        // If replacing existing key, deduct previous size
        if let Some(old) = self.entries.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(old.size_bytes);
        }

        // Evict LRU entries until size fits
        while self.current_bytes + size > self.capacity_bytes && !self.entries.is_empty() {
            if let Some(lru_key) = self.find_lru_key() {
                if let Some(removed) = self.entries.remove(&lru_key) {
                    self.current_bytes = self.current_bytes.saturating_sub(removed.size_bytes);
                }
            } else {
                break;
            }
        }

        self.current_bytes += size;
        self.entries.insert(
            key,
            L1Entry {
                image,
                size_bytes: size,
                last_accessed: Instant::now(),
            },
        );
    }

    fn find_lru_key(&self) -> Option<L1Key> {
        self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_bytes = 0;
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        (self.entries.len(), self.current_bytes, self.capacity_bytes)
    }
}
