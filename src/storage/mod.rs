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
        // ✅ 使用闭包和迭代器查找 cookie
        let get_cookie = |name: &str| -> Option<String> {
            cookies
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
        };

        Some(Credentials {
            sessdata: get_cookie("SESSDATA")?,
            bili_jct: get_cookie("bili_jct")?,
            dede_user_id: get_cookie("DedeUserID")?,
            dede_user_id_ckmd5: get_cookie("DedeUserID__ckMd5"),
            refresh_token,
        })
    }
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Keybindings {
    // Global actions
    pub quit: String,
    pub confirm: String,
    pub back: String,
    pub refresh: String,

    // Navigation
    pub nav_up: String,
    pub nav_down: String,
    pub nav_left: String,
    pub nav_right: String,
    pub nav_next_page: String,
    pub nav_prev_page: String,

    // Section/Tab navigation
    pub section_prev: String,
    pub section_next: String,
    pub tab_1: String,
    pub tab_2: String,
    pub tab_3: String,

    // Actions
    pub next_theme: String,
    pub play: String,
    pub open_settings: String,
    pub search_focus: String,

    // Comments
    pub comment: String,
    pub toggle_replies: String,

    // Dynamic page specific
    pub up_prev: String,
    pub up_next: String,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            // Global
            quit: "q".to_string(),
            confirm: "Enter".to_string(),
            back: "Esc".to_string(),
            refresh: "r".to_string(),

            // Navigation
            nav_up: "k".to_string(),
            nav_down: "j".to_string(),
            nav_left: "h".to_string(),
            nav_right: "l".to_string(),
            nav_next_page: "Tab".to_string(),
            nav_prev_page: "BackTab".to_string(),

            // Section/Tab
            section_prev: "[".to_string(),
            section_next: "]".to_string(),
            tab_1: "1".to_string(),
            tab_2: "2".to_string(),
            tab_3: "3".to_string(),

            // Actions
            next_theme: "t".to_string(),
            play: "p".to_string(),
            open_settings: "s".to_string(),
            search_focus: "/".to_string(),

            // Comments
            comment: "c".to_string(),
            toggle_replies: "r".to_string(),

            // Dynamic page
            up_prev: "h".to_string(),
            up_next: "l".to_string(),
        }
    }
}

use ratatui::crossterm::event::KeyCode;

impl Keybindings {
    /// Parse a string representation into a KeyCode
    pub fn parse_keycode(s: &str) -> Option<KeyCode> {
        let s = s.trim();
        match s.to_lowercase().as_str() {
            "enter" | "return" => Some(KeyCode::Enter),
            "esc" | "escape" => Some(KeyCode::Esc),
            "tab" => Some(KeyCode::Tab),
            "backtab" | "shift+tab" => Some(KeyCode::BackTab),
            "backspace" => Some(KeyCode::Backspace),
            "delete" | "del" => Some(KeyCode::Delete),
            "insert" | "ins" => Some(KeyCode::Insert),
            "home" => Some(KeyCode::Home),
            "end" => Some(KeyCode::End),
            "pageup" | "pgup" => Some(KeyCode::PageUp),
            "pagedown" | "pgdn" => Some(KeyCode::PageDown),
            "up" | "↑" => Some(KeyCode::Up),
            "down" | "↓" => Some(KeyCode::Down),
            "left" | "←" => Some(KeyCode::Left),
            "right" | "→" => Some(KeyCode::Right),
            "space" | " " => Some(KeyCode::Char(' ')),
            "f1" => Some(KeyCode::F(1)),
            "f2" => Some(KeyCode::F(2)),
            "f3" => Some(KeyCode::F(3)),
            "f4" => Some(KeyCode::F(4)),
            "f5" => Some(KeyCode::F(5)),
            "f6" => Some(KeyCode::F(6)),
            "f7" => Some(KeyCode::F(7)),
            "f8" => Some(KeyCode::F(8)),
            "f9" => Some(KeyCode::F(9)),
            "f10" => Some(KeyCode::F(10)),
            "f11" => Some(KeyCode::F(11)),
            "f12" => Some(KeyCode::F(12)),
            _ => {
                // Single character
                let chars: Vec<char> = s.chars().collect();
                if chars.len() == 1 {
                    Some(KeyCode::Char(chars[0]))
                } else {
                    None
                }
            }
        }
    }

    /// Convert a KeyCode to its string representation
    pub fn keycode_to_string(key: KeyCode) -> String {
        match key {
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::BackTab => "BackTab".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Delete => "Delete".to_string(),
            KeyCode::Insert => "Insert".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PageUp".to_string(),
            KeyCode::PageDown => "PageDown".to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            KeyCode::F(n) => format!("F{}", n),
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char(c) => c.to_string(),
            _ => "Unknown".to_string(),
        }
    }

    /// Check if a key matches the configured keybinding (including arrow key alternatives)
    pub fn matches(&self, binding: &str, key: KeyCode) -> bool {
        if let Some(configured_key) = Self::parse_keycode(binding) {
            if key == configured_key {
                return true;
            }
        }
        false
    }

    // Convenience methods for common keybindings
    pub fn matches_quit(&self, key: KeyCode) -> bool {
        self.matches(&self.quit, key)
    }

    pub fn matches_confirm(&self, key: KeyCode) -> bool {
        self.matches(&self.confirm, key)
    }

    pub fn matches_back(&self, key: KeyCode) -> bool {
        self.matches(&self.back, key)
    }

    pub fn matches_refresh(&self, key: KeyCode) -> bool {
        self.matches(&self.refresh, key)
    }

    pub fn matches_up(&self, key: KeyCode) -> bool {
        self.matches(&self.nav_up, key) || key == KeyCode::Up
    }

    pub fn matches_down(&self, key: KeyCode) -> bool {
        self.matches(&self.nav_down, key) || key == KeyCode::Down
    }

    pub fn matches_left(&self, key: KeyCode) -> bool {
        self.matches(&self.nav_left, key) || key == KeyCode::Left
    }

    pub fn matches_right(&self, key: KeyCode) -> bool {
        self.matches(&self.nav_right, key) || key == KeyCode::Right
    }

    pub fn matches_nav_next(&self, key: KeyCode) -> bool {
        self.matches(&self.nav_next_page, key)
    }

    pub fn matches_nav_prev(&self, key: KeyCode) -> bool {
        self.matches(&self.nav_prev_page, key)
    }

    pub fn matches_next_theme(&self, key: KeyCode) -> bool {
        self.matches(&self.next_theme, key)
    }

    pub fn matches_play(&self, key: KeyCode) -> bool {
        self.matches(&self.play, key)
    }

    pub fn matches_open_settings(&self, key: KeyCode) -> bool {
        self.matches(&self.open_settings, key)
    }

    pub fn matches_search_focus(&self, key: KeyCode) -> bool {
        self.matches(&self.search_focus, key) || key == KeyCode::Char('i')
    }

    pub fn matches_section_prev(&self, key: KeyCode) -> bool {
        self.matches(&self.section_prev, key)
    }

    pub fn matches_section_next(&self, key: KeyCode) -> bool {
        self.matches(&self.section_next, key)
    }

    pub fn matches_comment(&self, key: KeyCode) -> bool {
        self.matches(&self.comment, key)
    }

    pub fn matches_toggle_replies(&self, key: KeyCode) -> bool {
        self.matches(&self.toggle_replies, key)
    }

    pub fn matches_tab_1(&self, key: KeyCode) -> bool {
        self.matches(&self.tab_1, key)
    }

    pub fn matches_tab_2(&self, key: KeyCode) -> bool {
        self.matches(&self.tab_2, key)
    }

    pub fn matches_tab_3(&self, key: KeyCode) -> bool {
        self.matches(&self.tab_3, key)
    }

    pub fn matches_up_prev(&self, key: KeyCode) -> bool {
        self.matches(&self.up_prev, key)
    }

    pub fn matches_up_next(&self, key: KeyCode) -> bool {
        self.matches(&self.up_next, key)
    }

    pub fn get_nav_keys_display(&self) -> String {
        format!(
            "{}{}{}{}",
            self.nav_left, self.nav_up, self.nav_down, self.nav_right
        )
    }

    pub fn get_arrow_keys_display(&self) -> String {
        "←↑↓→".to_string()
    }

    /// Get all keybinding labels for display in settings
    pub fn get_all_labels(&self) -> Vec<(&'static str, &str)> {
        vec![
            // Global actions
            ("退出", &self.quit),
            ("确认", &self.confirm),
            ("返回", &self.back),
            ("刷新", &self.refresh),
            // Navigation
            ("向上", &self.nav_up),
            ("向下", &self.nav_down),
            ("向左", &self.nav_left),
            ("向右", &self.nav_right),
            ("下一页面", &self.nav_next_page),
            ("上一页面", &self.nav_prev_page),
            // Section/Tab
            ("上一分区", &self.section_prev),
            ("下一分区", &self.section_next),
            ("标签1", &self.tab_1),
            ("标签2", &self.tab_2),
            ("标签3", &self.tab_3),
            // Actions
            ("切换主题", &self.next_theme),
            ("播放", &self.play),
            ("设置", &self.open_settings),
            ("搜索", &self.search_focus),
            // Comments
            ("评论", &self.comment),
            ("展开回复", &self.toggle_replies),
            // Dynamic page
            ("上一UP", &self.up_prev),
            ("下一UP", &self.up_next),
        ]
    }

    /// Update a keybinding by index (for settings page)
    pub fn update_by_index(&mut self, index: usize, new_key: String) {
        match index {
            // Global actions
            0 => self.quit = new_key,
            1 => self.confirm = new_key,
            2 => self.back = new_key,
            3 => self.refresh = new_key,
            // Navigation
            4 => self.nav_up = new_key,
            5 => self.nav_down = new_key,
            6 => self.nav_left = new_key,
            7 => self.nav_right = new_key,
            8 => self.nav_next_page = new_key,
            9 => self.nav_prev_page = new_key,
            // Section/Tab
            10 => self.section_prev = new_key,
            11 => self.section_next = new_key,
            12 => self.tab_1 = new_key,
            13 => self.tab_2 = new_key,
            14 => self.tab_3 = new_key,
            // Actions
            15 => self.next_theme = new_key,
            16 => self.play = new_key,
            17 => self.open_settings = new_key,
            18 => self.search_focus = new_key,
            // Comments
            19 => self.comment = new_key,
            20 => self.toggle_replies = new_key,
            // Dynamic page
            21 => self.up_prev = new_key,
            22 => self.up_next = new_key,
            _ => {}
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
