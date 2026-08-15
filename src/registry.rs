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

/// All card definitions + locale strings from one loaded mod. Only the
/// "base" mod is loaded for the MVP -- mod load order / conflict
/// handling across multiple mods is deferred until there's a second mod
/// to conflict with.
pub struct CardRegistry {
    pub cards: HashMap<String, CardDefinition>,
    pub locale: HashMap<String, LocalizedCardText>,
}

impl CardRegistry {
    /// Parses already-loaded TOML text. Shared by the filesystem loader
    /// (native) and the embedded loader (wasm, or any target where
    /// reading arbitrary files off disk isn't available/desired).
    pub fn from_toml_str(cards_toml: &str, locale_toml: &str) -> Result<Self, LoadError> {
        let mut cards: HashMap<String, CardDefinition> = toml::from_str(cards_toml)?;
        for (id, def) in cards.iter_mut() {
            def.id = id.clone();
        }
        let locale: HashMap<String, LocalizedCardText> = toml::from_str(locale_toml)?;
        Ok(CardRegistry { cards, locale })
    }

    /// Reads `<mod_dir>/cards.toml` and `<mod_dir>/locale/<locale>/cards.toml`
    /// off disk. Native only -- there's no filesystem in the browser.
    /// This is the path real third-party mods will eventually load
    /// through; the MVP itself boots from `load_embedded_base` instead.
    pub fn load_mod(mod_dir: &Path, locale: &str) -> Result<Self, LoadError> {
        let cards_path = mod_dir.join("cards.toml");
        let locale_path = mod_dir.join("locale").join(locale).join("cards.toml");

        let cards_text = fs::read_to_string(&cards_path)
            .map_err(|e| LoadError(format!("reading {}: {e}", cards_path.display())))?;
        let locale_text = fs::read_to_string(&locale_path)
            .map_err(|e| LoadError(format!("reading {}: {e}", locale_path.display())))?;

        Self::from_toml_str(&cards_text, &locale_text)
    }

    /// The base mod, baked into the binary at compile time via
    /// `include_str!`. Used by both the CLI and the wasm build, so
    /// neither depends on a working directory or a browser fetch.
    pub fn load_embedded_base() -> CardRegistry {
        const CARDS_TOML: &str = include_str!("../mods/base/cards.toml");
        const LOCALE_TOML: &str = include_str!("../mods/base/locale/en/cards.toml");
        Self::from_toml_str(CARDS_TOML, LOCALE_TOML)
            .expect("embedded base mod TOML failed to parse -- this is a build-time bug")
    }

    pub fn get(&self, id: &str) -> Option<&CardDefinition> {
        self.cards.get(id)
    }

    /// Both input lifetimes are unified to `'a` on purpose: the
    /// fallback branch returns `id` itself (borrowed for as long as the
    /// caller's `id` reference lives), while the found branch returns
    /// text borrowed from `self`. Tying both parameters to the same
    /// lifetime is what lets the compiler accept either return path.
    pub fn name_of<'a>(&'a self, id: &'a str) -> &'a str {
        self.locale.get(id).map(|t| t.name.as_str()).unwrap_or(id)
    }

    pub fn description_of(&self, id: &str) -> &str {
        self.locale
            .get(id)
            .map(|t| t.description.as_str())
            .unwrap_or("")
    }
}
