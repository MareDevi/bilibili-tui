//! Left sidebar navigation component

use super::Theme;
use ratatui::{prelude::*, widgets::*};

/// Navigation menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Home,
    Search,
    Dynamic,
    History,
    Live,
    Settings,
}

impl NavItem {
    pub fn label(&self) -> &'static str {
        match self {
            NavItem::Home => "🏠 首页",
            NavItem::Search => "🔍 搜索",
            NavItem::Dynamic => "📺 动态",
            NavItem::History => "📜 历史",
            NavItem::Live => "📡 直播",
            NavItem::Settings => "⚙️ 设置",
        }
    }

    pub fn all() -> &'static [NavItem] {
        &[
            NavItem::Home,
            NavItem::Search,
            NavItem::Dynamic,
            NavItem::History,
            NavItem::Live,
            NavItem::Settings,
        ]
    }
}

pub struct Sidebar {
    pub selected: NavItem,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            selected: NavItem::Home,
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Main block with subtle right border
        let block = Block::default()
            .borders(Borders::RIGHT)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.border_subtle));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into header and nav items
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Header with branding
                Constraint::Length(1), // Separator
                Constraint::Min(5),    // Nav items
                Constraint::Length(1), // Footer separator
            ])
            .split(inner);

        // Bilibili branding header with modern styling
        let brand_lines = vec![
            Line::raw(""),
            Line::from(vec![
                Span::styled(
                    "  ▌",
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "B",
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "ilibili",
                    Style::default()
                        .fg(theme.fg_primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![Span::styled(
                "   TUI Client",
                Style::default()
                    .fg(theme.fg_muted)
                    .add_modifier(Modifier::ITALIC),
            )]),
        ];
        let brand = Paragraph::new(brand_lines);
        frame.render_widget(brand, chunks[0]);

        // Separator line with gradient effect
        let separator =
            Paragraph::new("  ────────────").style(Style::default().fg(theme.border_subtle));
        frame.render_widget(separator, chunks[1]);

        // Nav items with modern block selection indicator
        let items: Vec<ListItem> = NavItem::all()
            .iter()
            .map(|item| {
                let is_selected = *item == self.selected;
                let style = if is_selected {
                    Style::default()
                        .fg(theme.bilibili_pink)
                        .add_modifier(Modifier::BOLD)
                        .bg(theme.bg_highlight)
                } else {
                    Style::default().fg(theme.fg_secondary)
                };

                // Use block indicator for selection instead of arrow
                let prefix = if is_selected { " ▌" } else { "  " };
                let suffix = if is_selected { " " } else { "" };
                ListItem::new(format!("{}{}{}", prefix, item.label(), suffix)).style(style)
            })
            .collect();

        let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(list, chunks[2]);
    }

    pub fn next(&mut self) {
        let items = NavItem::all();
        let current_idx = items.iter().position(|i| *i == self.selected).unwrap_or(0);
        let next_idx = (current_idx + 1) % items.len();
        self.selected = items[next_idx];
    }

    pub fn prev(&mut self) {
        let items = NavItem::all();
        let current_idx = items.iter().position(|i| *i == self.selected).unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            items.len() - 1
        } else {
            current_idx - 1
        };
        self.selected = items[prev_idx];
    }

    pub fn select(&mut self, item: NavItem) {
        self.selected = item;
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}
