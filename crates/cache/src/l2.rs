use crate::error::CacheError;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

pub struct DiskEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub last_accessed: SystemTime,
}

pub struct DiskL2Cache {
    dir: PathBuf,
    capacity_bytes: u64,
    current_bytes: u64,
    entries: HashMap<String, DiskEntry>, // hash -> entry
    nonce: u64,
}

impl DiskL2Cache {
    pub fn open(dir: PathBuf, capacity_bytes: u64) -> Result<Self, CacheError> {
        fs::create_dir_all(&dir)?;
        let mut entries = HashMap::new();
        let mut current_bytes = 0u64;

        if let Ok(read_dir) = fs::read_dir(&dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("bin") {
                    if let Ok(meta) = entry.metadata() {
                        let size = meta.len();
                        let accessed = meta
                            .accessed()
                            .unwrap_or(meta.modified().unwrap_or_else(|_| SystemTime::now()));
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            entries.insert(
                                stem.to_string(),
                                DiskEntry {
                                    path: path.clone(),
                                    size_bytes: size,
                                    last_accessed: accessed,
                                },
                            );
                            current_bytes += size;
                        }
                    }
                }
            }
        }

        let mut cache = Self {
            dir,
            capacity_bytes,
            current_bytes,
            entries,
            nonce: 0,
        };

        if cache.current_bytes > cache.capacity_bytes {
            cache.evict_if_needed()?;
        }

        Ok(cache)
    }

    pub fn hash_url(url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn get_path(&self, url: &str) -> PathBuf {
        let hash = Self::hash_url(url);
        self.dir.join(format!("{}.bin", hash))
    }

    pub fn get(&mut self, url: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let hash = Self::hash_url(url);
        if let Some(entry) = self.entries.get_mut(&hash) {
            match fs::read(&entry.path) {
                Ok(bytes) => {
                    entry.last_accessed = SystemTime::now();
                    Ok(Some(bytes))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    self.current_bytes = self.current_bytes.saturating_sub(entry.size_bytes);
                    self.entries.remove(&hash);
                    Ok(None)
                }
                Err(e) => Err(CacheError::Io(e.to_string())),
            }
        } else {
            Ok(None)
        }
    }

    pub fn put(&mut self, url: &str, bytes: &[u8]) -> Result<PathBuf, CacheError> {
        let hash = Self::hash_url(url);
        let final_path = self.dir.join(format!("{}.bin", hash));
        self.nonce = self.nonce.wrapping_add(1);
        let temp_path = self.dir.join(format!("{}.tmp.{}_{}", hash, std::process::id(), self.nonce));

        fs::write(&temp_path, bytes)?;
        fs::rename(&temp_path, &final_path)?;

        let size = bytes.len() as u64;
        if let Some(old) = self.entries.remove(&hash) {
            self.current_bytes = self.current_bytes.saturating_sub(old.size_bytes);
        }

        self.current_bytes += size;
        self.entries.insert(
            hash,
            DiskEntry {
                path: final_path.clone(),
                size_bytes: size,
                last_accessed: SystemTime::now(),
            },
        );

        self.evict_if_needed()?;
        Ok(final_path)
    }

    fn evict_if_needed(&mut self) -> Result<(), CacheError> {
        if self.current_bytes <= self.capacity_bytes {
            return Ok(());
        }

        let target_bytes = (self.capacity_bytes as f64 * 0.85) as u64; // 15% headroom
        let mut sorted_keys: Vec<(String, SystemTime)> = self
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.last_accessed))
            .collect();
        sorted_keys.sort_by_key(|(_, time)| *time);

        for (k, _) in sorted_keys {
            if self.current_bytes <= target_bytes {
                break;
            }
            if let Some(entry) = self.entries.remove(&k) {
                let _ = fs::remove_file(&entry.path);
                self.current_bytes = self.current_bytes.saturating_sub(entry.size_bytes);
            }
        }

        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), CacheError> {
        for (_, entry) in self.entries.drain() {
            let _ = fs::remove_file(&entry.path);
        }
        self.current_bytes = 0;
        Ok(())
    }

    pub fn stats(&self) -> (usize, u64, u64) {
        (self.entries.len(), self.current_bytes, self.capacity_bytes)
    }
}
