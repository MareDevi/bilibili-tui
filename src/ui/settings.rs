//! Settings page with theme selection, keybinding display, and account management

use super::{Component, Theme, ThemeVariant};
use crate::app::AppAction;
use crate::storage::Keybindings;
use ratatui::{crossterm::event::KeyCode, prelude::*, widgets::*};

/// Settings sections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Theme,
    Keybindings,
    Account,
}

impl SettingsSection {
    pub fn all() -> &'static [SettingsSection] {
        &[
            SettingsSection::Theme,
            SettingsSection::Keybindings,
            SettingsSection::Account,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SettingsSection::Theme => "🎨 主题",
            SettingsSection::Keybindings => "⌨️ 快捷键",
            SettingsSection::Account => "👤 账户",
        }
    }
}

pub struct SettingsPage {
    pub current_section: SettingsSection,
    pub selected_theme_index: usize,
    pub selected_keybind_index: usize,
    pub keybindings: Keybindings,
    pub current_theme_variant: ThemeVariant,
    section_index: usize,
}

impl SettingsPage {
    pub fn new(keybindings: Keybindings, theme_variant: ThemeVariant) -> Self {
        let theme_index = ThemeVariant::all()
            .iter()
            .position(|v| *v == theme_variant)
            .unwrap_or(0);

        Self {
            current_section: SettingsSection::Theme,
            selected_theme_index: theme_index,
            selected_keybind_index: 0,
            keybindings,
            current_theme_variant: theme_variant,
            section_index: 0,
        }
    }

    fn keybind_labels(&self) -> Vec<(&'static str, &str)> {
        vec![
            ("退出", &self.keybindings.quit),
            ("向上", &self.keybindings.nav_up),
            ("向下", &self.keybindings.nav_down),
            ("向左", &self.keybindings.nav_left),
            ("向右", &self.keybindings.nav_right),
            ("确认", &self.keybindings.confirm),
            ("返回", &self.keybindings.back),
            ("切换主题", &self.keybindings.next_theme),
            ("播放", &self.keybindings.play),
            ("刷新", &self.keybindings.refresh),
            ("设置", &self.keybindings.open_settings),
        ]
    }
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new(Keybindings::default(), ThemeVariant::CatppuccinMocha)
    }
}

impl Component for SettingsPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
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
            Span::styled("⚙️ ", Style::default().fg(theme.fg_accent)),
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
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_unfocused)),
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
            SettingsSection::Keybindings => {
                self.draw_keybindings_section(frame, content_chunks[1], theme)
            }
            SettingsSection::Account => self.draw_account_section(frame, content_chunks[1], theme),
        }

        // Help bar
        let help_line = Line::from(vec![
            Span::styled(" [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                "Tab",
                Style::default()
                    .fg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("切换分类", Style::default().fg(theme.fg_secondary)),
            Span::styled("  [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                "↑↓",
                Style::default()
                    .fg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("选择", Style::default().fg(theme.fg_secondary)),
            Span::styled("  [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("确认", Style::default().fg(theme.fg_secondary)),
            Span::styled("  [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("返回", Style::default().fg(theme.fg_secondary)),
        ]);
        let help = Paragraph::new(help_line).alignment(Alignment::Center);
        frame.render_widget(help, main_chunks[2]);
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction> {
        match key {
            KeyCode::Esc => Some(AppAction::BackToList),
            KeyCode::Tab => {
                // Cycle through sections
                let sections = SettingsSection::all();
                self.section_index = (self.section_index + 1) % sections.len();
                self.current_section = sections[self.section_index];
                Some(AppAction::None)
            }
            KeyCode::Up | KeyCode::Char('k') => {
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
                    SettingsSection::Account => {}
                }
                Some(AppAction::None)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match self.current_section {
                    SettingsSection::Theme => {
                        let max = ThemeVariant::all().len().saturating_sub(1);
                        if self.selected_theme_index < max {
                            self.selected_theme_index += 1;
                        }
                    }
                    SettingsSection::Keybindings => {
                        let max = self.keybind_labels().len().saturating_sub(1);
                        if self.selected_keybind_index < max {
                            self.selected_keybind_index += 1;
                        }
                    }
                    SettingsSection::Account => {}
                }
                Some(AppAction::None)
            }
            KeyCode::Enter => {
                match self.current_section {
                    SettingsSection::Theme => {
                        let themes = ThemeVariant::all();
                        if self.selected_theme_index < themes.len() {
                            let selected = themes[self.selected_theme_index];
                            self.current_theme_variant = selected;
                            return Some(AppAction::SetTheme(selected));
                        }
                    }
                    SettingsSection::Account => {
                        // Logout
                        return Some(AppAction::Logout);
                    }
                    SettingsSection::Keybindings => {
                        // TODO: Could add keybind editing modal
                    }
                }
                Some(AppAction::None)
            }
            KeyCode::Char('q') => Some(AppAction::Quit),
            _ => Some(AppAction::None),
        }
    }
}

impl SettingsPage {
    fn draw_section_list(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::RIGHT)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_unfocused))
            .title(Span::styled(
                " 分类 ",
                Style::default()
                    .fg(theme.fg_accent)
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

        let list = List::new(items);
        frame.render_widget(list, inner);
    }

    fn draw_theme_section(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_unfocused))
            .title(Span::styled(
                " 🎨 选择主题 ",
                Style::default()
                    .fg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let items: Vec<ListItem> = ThemeVariant::all()
            .iter()
            .enumerate()
            .map(|(idx, variant)| {
                let is_selected = idx == self.selected_theme_index;
                let is_current = *variant == self.current_theme_variant;

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

                ListItem::new(format!("{}{}{}", prefix, variant.label(), suffix)).style(style)
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, inner);
    }

    fn draw_keybindings_section(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_unfocused))
            .title(Span::styled(
                " ⌨️ 快捷键 ",
                Style::default()
                    .fg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let labels = self.keybind_labels();
        let items: Vec<ListItem> = labels
            .iter()
            .enumerate()
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

        let list = List::new(items);
        frame.render_widget(list, inner);
    }

    fn draw_account_section(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_unfocused))
            .title(Span::styled(
                " 👤 账户 ",
                Style::default()
                    .fg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout for account info + logout button
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Info
                Constraint::Length(3), // Logout button
                Constraint::Min(0),    // Spacer
            ])
            .split(inner);

        let info = Paragraph::new("已登录")
            .style(Style::default().fg(theme.success))
            .alignment(Alignment::Left);
        frame.render_widget(info, chunks[0]);

        let logout_btn = Paragraph::new("▶ 退出登录")
            .style(
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.error)),
            )
            .alignment(Alignment::Center);
        frame.render_widget(logout_btn, chunks[1]);
    }
}
