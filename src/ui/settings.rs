//! Settings page with theme selection, keybinding display, and account management

use super::{Component, DEFAULT_THEME_ID, Theme, ThemeChoice};
use crate::application::AppAction;
use crate::storage::{DanmakuConfig, Keybindings, VideoQuality};
use ratatui::{crossterm::event::KeyCode, prelude::*, widgets::*};

/// Settings sections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Theme,
    Danmaku,
    Playback,
    Keybindings,
    Account,
}

impl SettingsSection {
    pub fn all() -> &'static [SettingsSection] {
        &[
            SettingsSection::Theme,
            SettingsSection::Danmaku,
            SettingsSection::Playback,
            SettingsSection::Keybindings,
            SettingsSection::Account,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SettingsSection::Theme => "🎨 主题",
            SettingsSection::Danmaku => "💬 弹幕",
            SettingsSection::Playback => "▶  播放",
            SettingsSection::Keybindings => "⌨️ 快捷键",
            SettingsSection::Account => "👤 账户",
        }
    }
}

pub struct SettingsPage {
    pub current_section: SettingsSection,
    pub selected_theme_index: usize,
    pub selected_danmaku_index: usize,
    pub selected_playback_index: usize,
    pub selected_keybind_index: usize,
    pub keybindings: Keybindings,
    pub current_theme_id: String,
    pub theme_choices: Vec<ThemeChoice>,
    pub is_logged_in: bool,
    pub danmaku: DanmakuConfig,
    pub auto_play: bool,
    pub video_quality: VideoQuality,
    section_index: usize,
    pub editing_keybind: bool,
    editing_danmaku: bool,
    danmaku_input: String,
    theme_scroll: usize,
    keybind_scroll: usize,
}

impl SettingsPage {
    pub fn new(
        keybindings: Keybindings,
        theme_id: String,
        is_logged_in: bool,
        danmaku: DanmakuConfig,
        auto_play: bool,
        video_quality: VideoQuality,
    ) -> Self {
        let theme_choices = Theme::available_theme_choices();
        let theme_index = theme_choices
            .iter()
            .position(|t| t.id == theme_id)
            .unwrap_or(0);

        Self {
            current_section: SettingsSection::Theme,
            selected_theme_index: theme_index,
            selected_danmaku_index: 0,
            selected_playback_index: 0,
            selected_keybind_index: 0,
            keybindings,
            current_theme_id: theme_id,
            theme_choices,
            is_logged_in,
            danmaku,
            auto_play,
            video_quality,
            section_index: 0,
            editing_keybind: false,
            editing_danmaku: false,
            danmaku_input: String::new(),
            theme_scroll: 0,
            keybind_scroll: 0,
        }
    }

    fn keybind_labels(&self) -> Vec<(&'static str, &str)> {
        vec![
            // Global actions
            ("退出", &self.keybindings.quit),
            ("确认", &self.keybindings.confirm),
            ("返回", &self.keybindings.back),
            ("刷新", &self.keybindings.refresh),
            // Navigation
            ("向上", &self.keybindings.nav_up),
            ("向下", &self.keybindings.nav_down),
            ("向左", &self.keybindings.nav_left),
            ("向右", &self.keybindings.nav_right),
            ("下一页面", &self.keybindings.nav_next_page),
            ("上一页面", &self.keybindings.nav_prev_page),
            ("内容下翻页", &self.keybindings.page_down),
            ("内容上翻页", &self.keybindings.page_up),
            // Section/Tab
            ("上一分区", &self.keybindings.section_prev),
            ("下一分区", &self.keybindings.section_next),
            ("标签1", &self.keybindings.tab_1),
            ("标签2", &self.keybindings.tab_2),
            ("标签3", &self.keybindings.tab_3),
            // Actions
            ("切换主题", &self.keybindings.next_theme),
            ("播放", &self.keybindings.play),
            ("设置", &self.keybindings.open_settings),
            ("搜索", &self.keybindings.search_focus),
            // Comments
            ("评论", &self.keybindings.comment),
            ("展开回复", &self.keybindings.toggle_replies),
            // Dynamic page
            ("上一UP", &self.keybindings.up_prev),
            ("下一UP", &self.keybindings.up_next),
        ]
    }
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new(
            Keybindings::default(),
            DEFAULT_THEME_ID.to_string(),
            false,
            DanmakuConfig::default(),
            true,
            VideoQuality::Best,
        )
    }
}

impl Component for SettingsPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        // Main layout: header + content
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Content
                Constraint::Length(2), // Help
            ])
            .split(area);

        // Header
        let header_line = Line::from(vec![
            Span::styled("⚙️ ", Style::default().fg(theme.bilibili_pink)),
            Span::styled(
                "设置",
                Style::default()
                    .fg(theme.fg_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        let header = Paragraph::new(header_line)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_type(BorderType::Plain)
                    .border_style(Style::default().fg(theme.border_subtle)),
            );
        frame.render_widget(header, main_chunks[0]);

        // Content: sidebar + section content
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(16), // Section list
                Constraint::Min(30),    // Section content
            ])
            .split(main_chunks[1]);

        // Section list (sidebar)
        self.draw_section_list(frame, content_chunks[0], theme);

        // Section content
        match self.current_section {
            SettingsSection::Theme => self.draw_theme_section(frame, content_chunks[1], theme),
            SettingsSection::Danmaku => self.draw_danmaku_section(frame, content_chunks[1], theme),
            SettingsSection::Playback => {
                self.draw_playback_section(frame, content_chunks[1], theme)
            }
            SettingsSection::Keybindings => {
                self.draw_keybindings_section(frame, content_chunks[1], theme)
            }
            SettingsSection::Account => self.draw_account_section(frame, content_chunks[1], theme),
        }

        // Help bar
        let help_line = if self.editing_keybind {
            Line::from("请按新的快捷键 · Esc 取消")
        } else if self.editing_danmaku {
            Line::from("输入新值 · Enter 保存 · Esc 取消")
        } else {
            Line::from(vec![
                Span::styled(" [", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    format!("{}{}", keys.section_prev, keys.section_next),
                    Style::default()
                        .fg(theme.fg_accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("] ", Style::default().fg(theme.fg_secondary)),
                Span::styled("切换分类", Style::default().fg(theme.fg_secondary)),
                Span::styled("  [", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    format!("{}{}", keys.nav_up, keys.nav_down),
                    Style::default()
                        .fg(theme.fg_accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("] ", Style::default().fg(theme.fg_secondary)),
                Span::styled("选择", Style::default().fg(theme.fg_secondary)),
                Span::styled("  [", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    &keys.confirm,
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("] ", Style::default().fg(theme.fg_secondary)),
                Span::styled("确认", Style::default().fg(theme.fg_secondary)),
                Span::styled("  [", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    &keys.nav_next_page,
                    Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                ),
                Span::styled("] ", Style::default().fg(theme.fg_secondary)),
                Span::styled("切页面", Style::default().fg(theme.fg_secondary)),
            ])
        };
        let help = Paragraph::new(help_line).alignment(Alignment::Center);
        frame.render_widget(help, main_chunks[2]);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        keys: &crate::storage::Keybindings,
    ) -> Option<AppAction> {
        // Handle keybind editing mode - any key pressed becomes the new binding
        if self.editing_keybind {
            if key == KeyCode::Esc {
                self.editing_keybind = false;
                return Some(AppAction::None);
            }
            let new_key = crate::storage::Keybindings::keycode_to_string(key);
            self.keybindings
                .update_by_index(self.selected_keybind_index, new_key);
            self.editing_keybind = false;
            // Save keybindings immediately after editing
            return Some(AppAction::SaveKeybindings(Box::new(
                self.keybindings.clone(),
            )));
        }

        if self.editing_danmaku {
            match key {
                KeyCode::Esc => {
                    self.editing_danmaku = false;
                    self.danmaku_input.clear();
                    return Some(AppAction::None);
                }
                KeyCode::Enter => {
                    if self.apply_danmaku_input() {
                        self.editing_danmaku = false;
                        self.danmaku_input.clear();
                        return Some(AppAction::SaveDanmakuConfig(Box::new(self.danmaku.clone())));
                    }
                    return Some(AppAction::None);
                }
                KeyCode::Backspace => {
                    self.danmaku_input.pop();
                    return Some(AppAction::None);
                }
                KeyCode::Char(character) => {
                    self.danmaku_input.push(character);
                    return Some(AppAction::None);
                }
                _ => return Some(AppAction::None),
            }
        }

        if keys.matches_back(key) {
            return Some(AppAction::BackToList);
        }
        if keys.matches_nav_next(key) {
            return Some(AppAction::NavNext);
        }
        if keys.matches_nav_prev(key) {
            return Some(AppAction::NavPrev);
        }
        if keys.matches_section_prev(key) {
            // Cycle through sections backwards
            let sections = SettingsSection::all();
            self.section_index = if self.section_index == 0 {
                sections.len() - 1
            } else {
                self.section_index - 1
            };
            self.current_section = sections[self.section_index];
            return Some(AppAction::None);
        }
        if keys.matches_section_next(key) {
            // Cycle through sections forwards
            let sections = SettingsSection::all();
            self.section_index = (self.section_index + 1) % sections.len();
            self.current_section = sections[self.section_index];
            return Some(AppAction::None);
        }
        if (keys.matches_left(key) || keys.matches_right(key))
            && self.current_section == SettingsSection::Playback
        {
            return Some(self.adjust_playback(if keys.matches_right(key) { 1 } else { -1 }));
        }
        if keys.matches_left(key) || keys.matches_right(key) {
            self.change_section(if keys.matches_right(key) { 1 } else { -1 });
            return Some(AppAction::None);
        }
        if keys.matches_up(key) {
            match self.current_section {
                SettingsSection::Theme => {
                    if self.selected_theme_index > 0 {
                        self.selected_theme_index -= 1;
                    }
                }
                SettingsSection::Keybindings => {
                    if self.selected_keybind_index > 0 {
                        self.selected_keybind_index -= 1;
                    }
                }
                SettingsSection::Danmaku => {
                    self.selected_danmaku_index = self.selected_danmaku_index.saturating_sub(1);
                }
                SettingsSection::Playback => {
                    self.selected_playback_index = self.selected_playback_index.saturating_sub(1);
                }
                SettingsSection::Account => {}
            }
            return Some(AppAction::None);
        }
        if keys.matches_down(key) {
            match self.current_section {
                SettingsSection::Theme => {
                    let max = self.theme_choices.len().saturating_sub(1);
                    if self.selected_theme_index < max {
                        self.selected_theme_index += 1;
                    }
                }
                SettingsSection::Keybindings => {
                    let max = self.keybindings.get_all_labels().len().saturating_sub(1);
                    if self.selected_keybind_index < max {
                        self.selected_keybind_index += 1;
                    }
                }
                SettingsSection::Danmaku => {
                    self.selected_danmaku_index =
                        (self.selected_danmaku_index + 1).min(Self::DANMAKU_ROWS - 1);
                }
                SettingsSection::Playback => {
                    self.selected_playback_index = (self.selected_playback_index + 1).min(1);
                }
                SettingsSection::Account => {}
            }
            return Some(AppAction::None);
        }
        if keys.matches_confirm(key) {
            match self.current_section {
                SettingsSection::Theme => {
                    if let Some(selected) = self.theme_choices.get(self.selected_theme_index) {
                        self.current_theme_id = selected.id.clone();
                        return Some(AppAction::SetTheme(selected.id.clone()));
                    }
                }
                SettingsSection::Account => {
                    return Some(if self.is_logged_in {
                        AppAction::Logout
                    } else {
                        AppAction::SwitchToLogin
                    });
                }
                SettingsSection::Danmaku => {
                    if matches!(self.selected_danmaku_index, 0 | 7) {
                        self.adjust_danmaku(1);
                        return Some(AppAction::SaveDanmakuConfig(Box::new(self.danmaku.clone())));
                    }
                    self.danmaku_input = self.current_danmaku_input();
                    self.editing_danmaku = true;
                }
                SettingsSection::Playback => {
                    return Some(self.adjust_playback(1));
                }
                SettingsSection::Keybindings => {
                    // Enter keybind editing mode
                    self.editing_keybind = true;
                }
            }
            return Some(AppAction::None);
        }
        if keys.matches_quit(key) {
            return Some(AppAction::Quit);
        }
        Some(AppAction::None)
    }
}

impl SettingsPage {
    const DANMAKU_ROWS: usize = 9;

    fn adjust_playback(&mut self, direction: i32) -> AppAction {
        match self.selected_playback_index {
            0 => {
                self.auto_play = !self.auto_play;
                AppAction::SaveAutoPlay(self.auto_play)
            }
            _ => {
                self.video_quality = self.video_quality.cycle(direction);
                AppAction::SaveVideoQuality(self.video_quality)
            }
        }
    }

    fn change_section(&mut self, direction: i32) {
        let sections = SettingsSection::all();
        self.section_index = if direction > 0 {
            (self.section_index + 1) % sections.len()
        } else {
            (self.section_index + sections.len() - 1) % sections.len()
        };
        self.current_section = sections[self.section_index];
    }

    fn current_danmaku_input(&self) -> String {
        match self.selected_danmaku_index {
            1 => format!("{:.0}", self.danmaku.display_area * 100.0),
            2 => format!("{:.0}", self.danmaku.opacity * 100.0),
            3 => format!("{:.0}", self.danmaku.font_scale * 100.0),
            4 => format!("{:.0}", self.danmaku.duration),
            5 => format!("{:.1}", self.danmaku.stroke_width),
            6 => format!("{:.1}", self.danmaku.line_height),
            8 => self.danmaku.font_family.clone(),
            _ => String::new(),
        }
    }

    fn apply_danmaku_input(&mut self) -> bool {
        let input = self.danmaku_input.trim();
        match self.selected_danmaku_index {
            1 => input
                .parse::<f64>()
                .map(|v| self.danmaku.display_area = (v / 100.0).clamp(0.1, 1.0))
                .is_ok(),
            2 => input
                .parse::<f64>()
                .map(|v| self.danmaku.opacity = (v / 100.0).clamp(0.0, 1.0))
                .is_ok(),
            3 => input
                .parse::<f64>()
                .map(|v| self.danmaku.font_scale = (v / 100.0).clamp(0.5, 2.5))
                .is_ok(),
            4 => input
                .parse::<f64>()
                .map(|v| self.danmaku.duration = v.clamp(3.0, 20.0))
                .is_ok(),
            5 => input
                .parse::<f64>()
                .map(|v| self.danmaku.stroke_width = v.clamp(0.0, 5.0))
                .is_ok(),
            6 => input
                .parse::<f64>()
                .map(|v| self.danmaku.line_height = v.clamp(1.0, 3.0))
                .is_ok(),
            8 if !input.is_empty() => {
                self.danmaku.font_family = input.to_string();
                true
            }
            _ => false,
        }
    }

    fn adjust_danmaku(&mut self, direction: i32) {
        let up = direction > 0;
        match self.selected_danmaku_index {
            0 => self.danmaku.enabled = !self.danmaku.enabled,
            1 => {
                self.danmaku.display_area =
                    (self.danmaku.display_area + if up { 0.1 } else { -0.1 }).clamp(0.1, 1.0)
            }
            2 => {
                self.danmaku.opacity =
                    (self.danmaku.opacity + if up { 0.1 } else { -0.1 }).clamp(0.0, 1.0)
            }
            3 => {
                self.danmaku.font_scale =
                    (self.danmaku.font_scale + if up { 0.1 } else { -0.1 }).clamp(0.5, 2.5)
            }
            4 => {
                self.danmaku.duration =
                    (self.danmaku.duration + if up { 1.0 } else { -1.0 }).clamp(3.0, 20.0)
            }
            5 => {
                self.danmaku.stroke_width =
                    (self.danmaku.stroke_width + if up { 0.5 } else { -0.5 }).clamp(0.0, 5.0)
            }
            6 => {
                self.danmaku.line_height =
                    (self.danmaku.line_height + if up { 0.1 } else { -0.1 }).clamp(1.0, 3.0)
            }
            7 => self.danmaku.massive_mode = !self.danmaku.massive_mode,
            8 => {
                const FONTS: [&str; 5] = [
                    "Yuanti SC",
                    "PingFang SC",
                    "Microsoft YaHei UI",
                    "Noto Sans CJK SC",
                    "sans-serif",
                ];
                let current = FONTS
                    .iter()
                    .position(|font| *font == self.danmaku.font_family)
                    .unwrap_or(0);
                let next = if up {
                    (current + 1) % FONTS.len()
                } else {
                    (current + FONTS.len() - 1) % FONTS.len()
                };
                self.danmaku.font_family = FONTS[next].to_string();
            }
            _ => {}
        }
    }

    fn draw_danmaku_section(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " 💬 弹幕设置 ",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rows = [
            format!(
                "显示：{}",
                if self.danmaku.enabled {
                    "开启"
                } else {
                    "关闭"
                }
            ),
            format!("显示区域：{:.0}%", self.danmaku.display_area * 100.0),
            format!("不透明度：{:.0}%", self.danmaku.opacity * 100.0),
            format!("字体大小：{:.0}%", self.danmaku.font_scale * 100.0),
            format!("滚动时长：{:.0} 秒", self.danmaku.duration),
            format!("描边粗细：{:.1}", self.danmaku.stroke_width),
            format!("弹幕行高：{:.1}", self.danmaku.line_height),
            format!(
                "海量模式：{}",
                if self.danmaku.massive_mode {
                    "开启"
                } else {
                    "关闭"
                }
            ),
            format!("字体：{}", self.danmaku.font_family),
        ];
        let items = rows.into_iter().enumerate().map(|(index, row)| {
            let selected = index == self.selected_danmaku_index;
            ListItem::new(format!("{}{}", if selected { "▶ " } else { "  " }, row)).style(
                if selected {
                    Style::default()
                        .fg(theme.fg_primary)
                        .bg(theme.selection_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg_secondary)
                },
            )
        });
        let chunks = Layout::vertical([
            Constraint::Min(9),
            Constraint::Length(if self.editing_danmaku { 3 } else { 0 }),
        ])
        .split(inner);
        frame.render_widget(List::new(items), chunks[0]);
        if self.editing_danmaku {
            let input = Paragraph::new(format!("{}▏", self.danmaku_input)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 输入新值 ")
                    .border_style(Style::default().fg(theme.fg_accent)),
            );
            frame.render_widget(input, chunks[1]);
        }
    }

    fn draw_playback_section(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " ▶  播放设置 ",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rows = [
            format!(
                "进入视频自动播放：{}",
                if self.auto_play { "开启" } else { "关闭" }
            ),
            format!("默认视频画质：{}", self.video_quality.label()),
        ];

        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

        let items = rows.into_iter().enumerate().map(|(index, row)| {
            let selected = index == self.selected_playback_index;
            ListItem::new(format!("{}{}", if selected { "▶ " } else { "  " }, row)).style(
                if selected {
                    Style::default()
                        .fg(theme.fg_primary)
                        .bg(theme.selection_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg_secondary)
                },
            )
        });
        frame.render_widget(List::new(items), chunks[0]);
        frame.render_widget(
            Paragraph::new("  按 ←/→ 或 Enter 更改并立即保存")
                .style(Style::default().fg(theme.fg_secondary)),
            chunks[1],
        );
    }

    fn draw_section_list(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::RIGHT)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " 分类 ",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let items: Vec<ListItem> = SettingsSection::all()
            .iter()
            .map(|section| {
                let is_selected = *section == self.current_section;
                let style = if is_selected {
                    Style::default()
                        .fg(theme.fg_accent)
                        .add_modifier(Modifier::BOLD)
                        .bg(theme.selection_bg)
                } else {
                    Style::default().fg(theme.fg_secondary)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                ListItem::new(format!("{}{}", prefix, section.label())).style(style)
            })
            .collect();

        frame.render_widget(List::new(items), inner);
    }

    fn draw_theme_section(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " 🎨 选择主题 ",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);
        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Keep selected visible with a scroll window
        let visible = inner.height as usize;
        if self.selected_theme_index < self.theme_scroll {
            self.theme_scroll = self.selected_theme_index;
        } else if self.selected_theme_index >= self.theme_scroll + visible {
            self.theme_scroll = self.selected_theme_index - visible + 1;
        }
        let max_scroll = self.theme_choices.len().saturating_sub(visible);
        self.theme_scroll = self.theme_scroll.min(max_scroll);

        let items: Vec<ListItem> = self
            .theme_choices
            .iter()
            .enumerate()
            .skip(self.theme_scroll)
            .take(visible)
            .map(|(idx, choice)| {
                let is_selected = idx == self.selected_theme_index;
                let is_current = choice.id == self.current_theme_id;

                let mut style = if is_selected {
                    Style::default()
                        .fg(theme.fg_primary)
                        .add_modifier(Modifier::BOLD)
                        .bg(theme.selection_bg)
                } else {
                    Style::default().fg(theme.fg_secondary)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                let suffix = if is_current { " ✓" } else { "" };

                if is_current && !is_selected {
                    style = style.fg(theme.success);
                }

                ListItem::new(format!("{}{}{}", prefix, choice.label.as_str(), suffix)).style(style)
            })
            .collect();

        frame.render_widget(List::new(items), inner);

        // Scroll hint when there are more items
        if self.theme_choices.len() > visible {
            let hint = format!(" {}/{} ", self.selected_theme_index + 1, self.theme_choices.len());
            let hint_area = Rect {
                x: inner.x + inner.width.saturating_sub(hint.len() as u16 + 1),
                y: inner.y + inner.height.saturating_sub(1),
                width: (hint.len() as u16 + 1).min(inner.width),
                height: 1,
            };
            frame.render_widget(
                Paragraph::new(hint).style(Style::default().fg(theme.fg_muted)),
                hint_area,
            );
        }
    }

    fn draw_keybindings_section(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " ⌨️ 快捷键 ",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::vertical([
            Constraint::Min(5),
            Constraint::Length(if self.editing_keybind { 3 } else { 0 }),
        ])
        .split(inner);

        let visible = chunks[0].height as usize;
        // Compute scroll without holding `labels` borrow
        let label_count = self.keybind_labels().len();
        if visible > 0 {
            if self.selected_keybind_index < self.keybind_scroll {
                self.keybind_scroll = self.selected_keybind_index;
            } else if self.selected_keybind_index >= self.keybind_scroll + visible {
                self.keybind_scroll = self.selected_keybind_index - visible + 1;
            }
            let max_scroll = label_count.saturating_sub(visible);
            self.keybind_scroll = self.keybind_scroll.min(max_scroll);
        }

        let labels = self.keybind_labels();
        let items: Vec<ListItem> = labels
            .iter()
            .enumerate()
            .skip(self.keybind_scroll)
            .take(chunks[0].height as usize)
            .map(|(idx, (label, key))| {
                let is_selected = idx == self.selected_keybind_index;
                let style = if is_selected {
                    Style::default()
                        .fg(theme.fg_primary)
                        .add_modifier(Modifier::BOLD)
                        .bg(theme.selection_bg)
                } else {
                    Style::default().fg(theme.fg_secondary)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(format!("{:<12}", label), style),
                    Span::styled(
                        format!("[{}]", key),
                        Style::default()
                            .fg(theme.fg_accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
            })
            .collect();

        frame.render_widget(List::new(items), chunks[0]);
        if self.editing_keybind {
            let label = labels[self.selected_keybind_index].0;
            frame.render_widget(
                Paragraph::new(format!("正在设置「{label}」：请按新的快捷键"))
                    .block(Block::default().borders(Borders::ALL).title(" 快捷键输入 ")),
                chunks[1],
            );
        }
    }

    fn draw_account_section(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " 👤 账户 ",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout for account info + action button
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Info
                Constraint::Length(3), // Action button
                Constraint::Min(0),    // Spacer
            ])
            .split(inner);

        let info = Paragraph::new(if self.is_logged_in {
            "已登录"
        } else {
            "未登录"
        })
        .style(Style::default().fg(if self.is_logged_in {
            theme.success
        } else {
            theme.warning
        }))
        .alignment(Alignment::Left);
        frame.render_widget(info, chunks[0]);

        let action_label = if self.is_logged_in {
            "▶ 退出登录"
        } else {
            "▶ 去登录"
        };
        let action_color = if self.is_logged_in {
            theme.error
        } else {
            theme.info
        };
        let action_btn = Paragraph::new(action_label)
            .style(
                Style::default()
                    .fg(action_color)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(action_color)),
            )
            .alignment(Alignment::Center);
        frame.render_widget(action_btn, chunks[1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn danmaku_settings_use_explicit_text_editor() {
        let keys = Keybindings::default();
        let mut page = SettingsPage {
            current_section: SettingsSection::Danmaku,
            selected_danmaku_index: 1,
            ..SettingsPage::default()
        };

        page.handle_input(KeyCode::Enter, &keys);
        assert!(page.editing_danmaku);
        page.danmaku_input = "75".to_string();
        let action = page
            .handle_input(KeyCode::Enter, &keys)
            .expect("settings action");
        assert!((page.danmaku.display_area - 0.75).abs() < f64::EPSILON);
        assert!(matches!(action, AppAction::SaveDanmakuConfig(_)));
    }

    #[test]
    fn escape_cancels_keybinding_edit() {
        let keys = Keybindings::default();
        let mut page = SettingsPage {
            current_section: SettingsSection::Keybindings,
            editing_keybind: true,
            ..SettingsPage::default()
        };
        let original = page.keybindings.quit.clone();
        page.handle_input(KeyCode::Esc, &keys);
        assert!(!page.editing_keybind);
        assert_eq!(page.keybindings.quit, original);
    }

    #[test]
    fn playback_rows_save_auto_play_and_quality_immediately() {
        let keys = Keybindings::default();
        let mut page = SettingsPage {
            current_section: SettingsSection::Playback,
            section_index: 2,
            ..SettingsPage::default()
        };

        let action = page.handle_input(KeyCode::Enter, &keys);
        assert!(matches!(action, Some(AppAction::SaveAutoPlay(false))));

        page.handle_input(KeyCode::Char('j'), &keys);
        let action = page.handle_input(KeyCode::Char('l'), &keys);
        assert_eq!(page.video_quality, VideoQuality::Q4k);
        assert!(matches!(
            action,
            Some(AppAction::SaveVideoQuality(VideoQuality::Q4k))
        ));
    }
}
