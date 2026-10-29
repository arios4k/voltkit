use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// User configuration, stored at `~/.config/voltkit/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub hotkeys: Hotkeys,
    pub ocr: OcrConfig,
    pub compress: CompressConfig,
    pub expander: ExpanderConfig,
    pub snippets: Vec<Snippet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkeys {
    pub ocr: String,
    pub color_picker: String,
    pub snippets: String,
    pub qr: String,
    pub ruler: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OcrConfig {
    pub lang: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CompressConfig {
    pub quality: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExpanderConfig {
    /// Expand triggers typed in any app. Needs `voltkit expander setup`.
    pub typed_triggers: bool,
    /// Keys sent to paste an expansion: "ctrl+v", "ctrl+shift+v" or "shift+insert".
    pub paste_shortcut: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    /// Typed abbreviation that triggers expansion, e.g. ";date".
    pub trigger: String,
    pub template: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkeys: Hotkeys::default(),
            ocr: OcrConfig::default(),
            compress: CompressConfig::default(),
            expander: ExpanderConfig::default(),
            snippets: vec![
                Snippet { trigger: ";date".into(), template: "{date:%Y-%m-%d}".into() },
                Snippet { trigger: ";now".into(), template: "{date:%Y-%m-%d} {time}".into() },
                Snippet {
                    trigger: ";quote".into(),
                    template: "> {clipboard}\n\n— {arg:author}".into(),
                },
            ],
        }
    }
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            ocr: "<Super><Shift>t".into(),
            color_picker: "<Super><Shift>c".into(),
            snippets: "<Super><Shift>e".into(),
            qr: "<Super><Shift>q".into(),
            ruler: "<Super><Shift>r".into(),
        }
    }
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self { lang: "eng".into() }
    }
}

impl Default for ExpanderConfig {
    fn default() -> Self {
        Self { typed_triggers: false, paste_shortcut: "ctrl+v".into() }
    }
}

impl Default for CompressConfig {
    fn default() -> Self {
        Self { quality: 80 }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("voltkit").join("config.toml"))
    }

    /// Loads the config file, falling back to defaults when it does not exist.
    pub fn load() -> Result<Self> {
        let Some(path) = Self::path().filter(|p| p.exists()) else {
            return Ok(Self::default());
        };
        let text = std::fs::read_to_string(&path)?;
        toml::from_str(&text).with_context(|| format!("invalid config at {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path().context("no config directory")?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_config_fills_defaults() {
        let cfg: Config = toml::from_str("[ocr]\nlang = \"deu\"\n").unwrap();
        assert_eq!(cfg.ocr.lang, "deu");
        assert_eq!(cfg.compress.quality, 80);
        assert_eq!(cfg.hotkeys.ocr, "<Super><Shift>t");
    }

    #[test]
    fn example_config_parses() {
        let cfg: Config = toml::from_str(include_str!("../data/config.example.toml")).unwrap();
        assert_eq!(cfg.snippets[1].trigger, ";sig");
    }

    #[test]
    fn default_round_trips() {
        let text = toml::to_string_pretty(&Config::default()).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.snippets.len(), 3);
    }
}
