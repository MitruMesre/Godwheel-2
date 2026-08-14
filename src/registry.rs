use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::card::CardDefinition;

#[derive(Debug, Clone, Deserialize)]
pub struct LocalizedCardText {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug)]
pub struct LoadError(String);

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for LoadError {}
impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        LoadError(format!("io error: {e}"))
    }
}
impl From<toml::de::Error> for LoadError {
    fn from(e: toml::de::Error) -> Self {
        LoadError(format!("toml parse error: {e}"))
    }
}

/// All card definitions + locale strings from every loaded mod, merged
/// into one flat lookup. Only the "base" mod is loaded for the MVP --
/// mod load order / conflict handling is deferred until there's a second
/// mod to conflict with.
pub struct CardRegistry {
    pub cards: HashMap<String, CardDefinition>,
    pub locale: HashMap<String, LocalizedCardText>,
}

impl CardRegistry {
    pub fn load_mod(mod_dir: &Path, locale: &str) -> Result<Self, LoadError> {
        let cards_path = mod_dir.join("cards.toml");
        let locale_path = mod_dir.join("locale").join(locale).join("cards.toml");

        let cards_text = fs::read_to_string(&cards_path)
            .map_err(|e| LoadError(format!("reading {}: {e}", cards_path.display())))?;
        let mut cards: HashMap<String, CardDefinition> = toml::from_str(&cards_text)?;
        for (id, def) in cards.iter_mut() {
            def.id = id.clone();
        }

        let locale_text = fs::read_to_string(&locale_path)
            .map_err(|e| LoadError(format!("reading {}: {e}", locale_path.display())))?;
        let locale: HashMap<String, LocalizedCardText> = toml::from_str(&locale_text)?;

        Ok(CardRegistry { cards, locale })
    }

    pub fn get(&self, id: &str) -> Option<&CardDefinition> {
        self.cards.get(id)
    }

    pub fn name_of(&self, id: &str) -> &str {
        self.locale
            .get(id)
            .map(|t| t.name.as_str())
            .unwrap_or(id)
    }

    pub fn description_of(&self, id: &str) -> &str {
        self.locale
            .get(id)
            .map(|t| t.description.as_str())
            .unwrap_or("")
    }
}
