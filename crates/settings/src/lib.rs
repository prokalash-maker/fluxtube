use fluxtube_core::{PerformanceProfileType, PerformanceSettings};
use fluxtube_storage::{StorageError, StorageManager};
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum SettingsError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialization(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialization(#[from] toml::de::Error),
}

pub fn recommend_profile(cores: usize, total_ram_mb: u64) -> PerformanceProfileType {
    if total_ram_mb < 4096 || cores <= 2 {
        PerformanceProfileType::UltraLite
    } else if total_ram_mb <= 8192 || cores <= 4 {
        PerformanceProfileType::Balanced
    } else {
        PerformanceProfileType::HighQuality
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub language: String,        // "en", "uk", "ru", "auto"
    pub theme: String,           // "graphite_dark", "pure_black", "arctic_light", "system"
    pub accent_color: String,    // hex code "#FF0033"
    pub ui_scale: f32,           // 0.75 .. 1.5
    pub density: String,         // "compact", "standard", "comfortable"
    pub start_page: String,      // "home", "trending", "subscriptions", "library"
    pub close_to_tray: bool,
    pub performance: PerformanceSettings,
    pub save_history: bool,
    pub save_search_history: bool,
    pub h264ify_enabled: bool,
    pub autoplay_next: bool,
    pub default_volume: f32,     // 0.0 .. 1.0
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            theme: "graphite_dark".to_string(),
            accent_color: "#FF0033".to_string(),
            ui_scale: 1.0,
            density: "standard".to_string(),
            start_page: "home".to_string(),
            close_to_tray: false,
            performance: PerformanceSettings::balanced(),
            save_history: true,
            save_search_history: true,
            h264ify_enabled: true,
            autoplay_next: true,
            default_volume: 0.8,
        }
    }
}

pub struct SettingsManager {
    settings: AppSettings,
}

impl SettingsManager {
    pub fn new(settings: AppSettings) -> Self {
        Self { settings }
    }

    pub fn current(&self) -> &AppSettings {
        &self.settings
    }

    pub fn current_mut(&mut self) -> &mut AppSettings {
        &mut self.settings
    }

    pub fn set(&mut self, settings: AppSettings) {
        self.settings = settings;
    }

    pub fn load_from_storage(storage: &StorageManager) -> Result<Self, SettingsError> {
        let mut settings = AppSettings::default();

        if let Some(json_str) = storage.get_setting("app_settings")? {
            match serde_json::from_str::<AppSettings>(&json_str) {
                Ok(loaded) => settings = loaded,
                Err(e) => {
                    tracing::warn!("Failed to deserialize app_settings, using defaults: {}", e);
                }
            }
        } else {
            // Also check individual keys for backwards compatibility or fine-grained storage
            if let Some(lang) = storage.get_setting("app.language")? {
                settings.language = lang;
            }
            if let Some(theme) = storage.get_setting("app.theme")? {
                settings.theme = theme;
            }
        }

        Ok(Self { settings })
    }

    pub fn sync_to_storage(&self, storage: &StorageManager) -> Result<(), SettingsError> {
        let json_str = serde_json::to_string(&self.settings)?;
        storage.set_setting("app_settings", &json_str)?;

        // Also sync scalar keys
        storage.set_setting("app.language", &self.settings.language)?;
        storage.set_setting("app.theme", &self.settings.theme)?;

        let perf_json = serde_json::to_string(&self.settings.performance)?;
        storage.set_setting("performance_settings", &perf_json)?;

        Ok(())
    }

    pub fn export_toml(&self) -> Result<String, SettingsError> {
        let toml_str = toml::to_string_pretty(&self.settings)?;
        Ok(toml_str)
    }

    pub fn import_toml(&mut self, toml_str: &str) -> Result<(), SettingsError> {
        let parsed: AppSettings = toml::from_str(toml_str)?;
        self.settings = parsed;
        Ok(())
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new(AppSettings::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommend_profile_heuristic() {
        // <= 2 cores or < 4GB RAM -> UltraLite
        assert_eq!(
            recommend_profile(2, 3500),
            PerformanceProfileType::UltraLite
        );
        assert_eq!(
            recommend_profile(8, 3800),
            PerformanceProfileType::UltraLite
        );
        assert_eq!(
            recommend_profile(1, 16000),
            PerformanceProfileType::UltraLite
        );

        // <= 4 cores or <= 8GB RAM -> Balanced
        assert_eq!(
            recommend_profile(4, 8000),
            PerformanceProfileType::Balanced
        );
        assert_eq!(
            recommend_profile(8, 6000),
            PerformanceProfileType::Balanced
        );
        assert_eq!(
            recommend_profile(4, 16000),
            PerformanceProfileType::Balanced
        );

        // > 4 cores and > 8GB RAM -> HighQuality
        assert_eq!(
            recommend_profile(8, 16000),
            PerformanceProfileType::HighQuality
        );
        assert_eq!(
            recommend_profile(12, 32000),
            PerformanceProfileType::HighQuality
        );
    }

    #[test]
    fn test_settings_storage_sync() {
        let storage = StorageManager::open_in_memory().unwrap();
        let mut manager = SettingsManager::default();
        manager.current_mut().language = "uk".to_string();
        manager.current_mut().theme = "pure_black".to_string();
        manager.current_mut().performance = PerformanceSettings::high_quality();

        manager.sync_to_storage(&storage).unwrap();

        let loaded = SettingsManager::load_from_storage(&storage).unwrap();
        assert_eq!(loaded.current().language, "uk");
        assert_eq!(loaded.current().theme, "pure_black");
        assert_eq!(
            loaded.current().performance.profile,
            PerformanceProfileType::HighQuality
        );
    }

    #[test]
    fn test_toml_export_import() {
        let mut manager = SettingsManager::default();
        manager.current_mut().theme = "arctic_light".to_string();

        let exported = manager.export_toml().expect("export toml");
        assert!(exported.contains("theme = \"arctic_light\""));

        let mut manager2 = SettingsManager::default();
        manager2.import_toml(&exported).expect("import toml");
        assert_eq!(manager2.current().theme, "arctic_light");
    }
}
