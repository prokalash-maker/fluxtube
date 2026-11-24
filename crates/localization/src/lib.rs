use std::collections::HashMap;

const EN_JSON: &str = include_str!("../locales/en.json");
const UK_JSON: &str = include_str!("../locales/uk.json");
const RU_JSON: &str = include_str!("../locales/ru.json");

#[derive(thiserror::Error, Debug)]
pub enum LocalizationError {
    #[error("Unsupported locale: {0}")]
    UnsupportedLocale(String),

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
}

pub struct LocalizationManager {
    current_locale: String,
    catalogs: HashMap<String, HashMap<String, String>>,
}

impl LocalizationManager {
    pub fn new(initial_locale: &str) -> Self {
        let mut catalogs = HashMap::new();
        catalogs.insert("en".to_string(), Self::parse_catalog(EN_JSON));
        catalogs.insert("uk".to_string(), Self::parse_catalog(UK_JSON));
        catalogs.insert("ru".to_string(), Self::parse_catalog(RU_JSON));

        let locale = if catalogs.contains_key(initial_locale) {
            initial_locale.to_string()
        } else {
            "en".to_string()
        };

        Self {
            current_locale: locale,
            catalogs,
        }
    }

    fn parse_catalog(json_str: &str) -> HashMap<String, String> {
        let val: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
        let mut map = HashMap::new();
        Self::flatten_value("", &val, &mut map);
        map
    }

    fn flatten_value(prefix: &str, val: &serde_json::Value, map: &mut HashMap<String, String>) {
        match val {
            serde_json::Value::Object(obj) => {
                for (k, v) in obj {
                    let new_prefix = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    Self::flatten_value(&new_prefix, v, map);
                }
            }
            serde_json::Value::String(s) => {
                map.insert(prefix.to_string(), s.clone());
            }
            _ => {}
        }
    }

    pub fn set_locale(&mut self, locale: &str) -> Result<(), LocalizationError> {
        if self.catalogs.contains_key(locale) {
            self.current_locale = locale.to_string();
            Ok(())
        } else {
            tracing::warn!(
                "Unsupported locale: {}, keeping {}",
                locale,
                self.current_locale
            );
            Err(LocalizationError::UnsupportedLocale(locale.to_string()))
        }
    }

    pub fn current_locale(&self) -> &str {
        &self.current_locale
    }

    pub fn t(&self, key: &str) -> String {
        // 1. Try current locale
        if let Some(cat) = self.catalogs.get(&self.current_locale) {
            if let Some(text) = cat.get(key) {
                return text.clone();
            }
        }
        // 2. Fallback to "en"
        if let Some(en_cat) = self.catalogs.get("en") {
            if let Some(text) = en_cat.get(key) {
                return text.clone();
            }
        }
        // 3. Fallback to raw key
        tracing::warn!("Missing localization key: {}", key);
        key.to_string()
    }

    pub fn t_with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut template = self.t(key);
        for (k, v) in args {
            let placeholder = format!("{{{}}}", k);
            template = template.replace(&placeholder, v);
        }
        template
    }

    pub fn available_locales() -> Vec<&'static str> {
        vec!["en", "uk", "ru"]
    }
}

impl Default for LocalizationManager {
    fn default() -> Self {
        Self::new("en")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_locales_load() {
        let locales = LocalizationManager::available_locales();
        assert_eq!(locales, vec!["en", "uk", "ru"]);

        let en_mgr = LocalizationManager::new("en");
        assert_eq!(en_mgr.t("app.name"), "FluxTube");
        assert_eq!(en_mgr.t("nav.home"), "Home");

        let uk_mgr = LocalizationManager::new("uk");
        assert_eq!(uk_mgr.t("nav.home"), "Головна");
        assert_eq!(uk_mgr.t("settings.profile.ultralite"), "Ультра Легкий");

        let ru_mgr = LocalizationManager::new("ru");
        assert_eq!(ru_mgr.t("nav.home"), "Главная");
        assert_eq!(ru_mgr.t("settings.profile.ultralite"), "Ультра Легкий");
    }

    #[test]
    fn test_fallback_chain() {
        let mut mgr = LocalizationManager::new("uk");
        // Unknown key falls back to key itself
        assert_eq!(mgr.t("non.existent.key"), "non.existent.key");

        // Setting invalid locale fails gracefully
        assert!(mgr.set_locale("es").is_err());
        assert_eq!(mgr.current_locale(), "uk");
    }

    #[test]
    fn test_argument_interpolation() {
        let mgr = LocalizationManager::new("en");
        let result = mgr.t_with_args("player.quality", &[("quality", "1080p60")]);
        assert_eq!(result, "Quality: 1080p60");

        let result2 = mgr.t_with_args(
            "diagnostics.memory",
            &[("used", "120"), ("total", "8192")],
        );
        assert_eq!(result2, "RAM: 120 MB / 8192 MB");

        let uk_mgr = LocalizationManager::new("uk");
        let uk_res = uk_mgr.t_with_args("player.seek_forward", &[("seconds", "10")]);
        assert_eq!(uk_res, "Вперед на 10с");
    }
}
