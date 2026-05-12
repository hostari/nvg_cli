//! Config file handling: `~/.config/nvg/config.toml`.
//!
//! Read + write of api_url + token for any profile. Used by login/logout in
//! `commands::auth`.

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const DEFAULT_API_URL: &str = "https://app.navegante.app";
pub const DEFAULT_PROFILE_NAME: &str = "default";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub api_url: Option<String>,
    pub token: Option<String>,
}

/// Resolved values used by every command.
#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub name: String,
    pub api_url: String,
    pub token: Option<String>,
}

impl Config {
    /// Path to the config file. Honors `NVG_CONFIG` for tests.
    pub fn path() -> Result<PathBuf> {
        if let Ok(p) = std::env::var("NVG_CONFIG") {
            return Ok(PathBuf::from(p));
        }
        let dirs = ProjectDirs::from("io", "Navegante", "nvg")
            .context("could not determine config directory")?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    /// Load the config file, returning a default empty Config if it does not exist.
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config at {}", path.display()))?;
        let cfg: Config = toml::from_str(&raw)
            .with_context(|| format!("parsing config at {}", path.display()))?;
        Ok(cfg)
    }

    /// Resolve the active profile, honoring an explicit `--profile` override.
    /// Falls back to defaults when the file or profile is missing.
    pub fn resolve(&self, profile_override: Option<&str>) -> ResolvedProfile {
        let name = profile_override
            .map(str::to_string)
            .or_else(|| self.default_profile.clone())
            .unwrap_or_else(|| DEFAULT_PROFILE_NAME.to_string());

        let profile = self.profiles.get(&name).cloned().unwrap_or_default();

        ResolvedProfile {
            name,
            api_url: profile
                .api_url
                .unwrap_or_else(|| DEFAULT_API_URL.to_string()),
            token: profile.token,
        }
    }

    /// Persist this Config back to disk, creating parent directories and the
    /// file with restrictive permissions.
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating config dir {}", parent.display()))?;
        }
        let toml = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&path, toml)
            .with_context(|| format!("writing config to {}", path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    /// Set the api_url and token for the named profile, also marking it as the
    /// default if no default is currently set.
    pub fn set_profile(&mut self, name: &str, api_url: &str, token: Option<String>) {
        let entry = self.profiles.entry(name.to_string()).or_default();
        entry.api_url = Some(api_url.to_string());
        entry.token = token;
        if self.default_profile.is_none() {
            self.default_profile = Some(name.to_string());
        }
    }

    /// Clear the token for the named profile, leaving the api_url intact.
    pub fn clear_token(&mut self, name: &str) {
        if let Some(profile) = self.profiles.get_mut(name) {
            profile.token = None;
        }
    }
}
