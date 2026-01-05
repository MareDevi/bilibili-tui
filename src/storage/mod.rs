//! Credential storage and persistence

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// User credentials from Bilibili login
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub sessdata: String,
    pub bili_jct: String,
    pub dede_user_id: String,
    pub dede_user_id_ckmd5: Option<String>,
    pub refresh_token: Option<String>,
}

impl Credentials {
    pub fn from_cookies(
        cookies: &[(String, String)],
        refresh_token: Option<String>,
    ) -> Option<Self> {
        let mut sessdata = None;
        let mut bili_jct = None;
        let mut dede_user_id = None;
        let mut dede_user_id_ckmd5 = None;

        for (name, value) in cookies {
            match name.as_str() {
                "SESSDATA" => sessdata = Some(value.clone()),
                "bili_jct" => bili_jct = Some(value.clone()),
                "DedeUserID" => dede_user_id = Some(value.clone()),
                "DedeUserID__ckMd5" => dede_user_id_ckmd5 = Some(value.clone()),
                _ => {}
            }
        }

        Some(Credentials {
            sessdata: sessdata?,
            bili_jct: bili_jct?,
            dede_user_id: dede_user_id?,
            dede_user_id_ckmd5,
            refresh_token,
        })
    }
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings {
    pub quit: String,
    pub nav_up: String,
    pub nav_down: String,
    pub nav_left: String,
    pub nav_right: String,
    pub confirm: String,
    pub back: String,
    pub next_theme: String,
    pub play: String,
    pub refresh: String,
    pub open_settings: String,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            quit: "q".to_string(),
            nav_up: "k".to_string(),
            nav_down: "j".to_string(),
            nav_left: "h".to_string(),
            nav_right: "l".to_string(),
            confirm: "Enter".to_string(),
            back: "Esc".to_string(),
            next_theme: "t".to_string(),
            play: "p".to_string(),
            refresh: "r".to_string(),
            open_settings: "s".to_string(),
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme: String,
    pub keybindings: Keybindings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "CatppuccinMocha".to_string(),
            keybindings: Keybindings::default(),
        }
    }
}

/// Get the config directory path
fn get_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("bilibili-tui");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir)
}

/// Get the credentials file path
fn get_credentials_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("credentials.json"))
}

/// Get the config file path
fn get_config_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.json"))
}

/// Save credentials to disk
pub fn save_credentials(credentials: &Credentials) -> Result<()> {
    let path = get_credentials_path()?;
    let json = serde_json::to_string_pretty(credentials)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load credentials from disk
pub fn load_credentials() -> Result<Credentials> {
    let path = get_credentials_path()?;
    let json = fs::read_to_string(path)?;
    let credentials: Credentials = serde_json::from_str(&json)?;
    Ok(credentials)
}

/// Delete credentials (logout)
pub fn delete_credentials() -> Result<()> {
    let path = get_credentials_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Save app config to disk
pub fn save_config(config: &AppConfig) -> Result<()> {
    let path = get_config_path()?;
    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load app config from disk
pub fn load_config() -> Result<AppConfig> {
    let path = get_config_path()?;
    if path.exists() {
        let json = fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&json)?;
        Ok(config)
    } else {
        Ok(AppConfig::default())
    }
}

/// Export cookies in Netscape format for yt-dlp
pub fn export_cookies_for_ytdlp(credentials: &Credentials) -> Result<PathBuf> {
    let path = get_config_dir()?.join("cookies.txt");

    let content = format!(
        "# Netscape HTTP Cookie File\n\
        .bilibili.com\tTRUE\t/\tTRUE\t0\tSESSDATA\t{}\n\
        .bilibili.com\tTRUE\t/\tFALSE\t0\tbili_jct\t{}\n\
        .bilibili.com\tTRUE\t/\tFALSE\t0\tDedeUserID\t{}\n",
        credentials.sessdata, credentials.bili_jct, credentials.dede_user_id
    );

    fs::write(&path, content)?;
    Ok(path)
}
