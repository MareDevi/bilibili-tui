//! History page with watch history display in a grid layout with cover images

use super::{Component, Theme};
use crate::api::client::ApiClient;
use crate::api::history::{HistoryCursor, HistoryItem};
use crate::app::AppAction;
use crate::storage::Keybindings;
use image::DynamicImage;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind},
    prelude::*,
    widgets::*,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

/// History card with cached cover image
struct HistoryCard {
    item: HistoryItem,
    cover_protocol: Option<StatefulProtocol>,
}

/// Message for completed cover download
struct CoverResult {
    index: usize,
    protocol: StatefulProtocol,
}

pub struct HistoryPage {
    items: Vec<HistoryCard>,
    selected: usize,
    scroll_offset: usize,
    loading: bool,
    error: Option<String>,
    picker: Arc<Picker>,
    cursor: Option<HistoryCursor>,
    has_more: bool,

    pending_downloads: HashSet<usize>,
    cover_rx: mpsc::Receiver<CoverResult>,
    cover_tx: mpsc::Sender<CoverResult>,

    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
}

impl HistoryPage {
    pub fn new() -> Self {
        let picker = Arc::new(Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks()));
        let (tx, rx) = mpsc::channel(32);

        Self {
            items: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            loading: false,
            error: None,
            picker,
            cursor: None,
            has_more: true,
            pending_downloads: HashSet::new(),
            cover_rx: rx,
            cover_tx: tx,
            last_click_time: None,
            last_click_index: None,
        }
    }

    pub async fn load_history(&mut self, api_client: &ApiClient) {
        self.loading = true;
        self.error = None;

        match api_client.get_history(None, None, None).await {
            Ok(data) => {
                self.items = data
                    .list
                    .into_iter()
                    .map(|item| HistoryCard {
                        item,
                        cover_protocol: None,
                    })
                    .collect();
                self.cursor = Some(data.cursor);
                self.has_more = !self.items.is_empty();
                self.loading = false;
            }
            Err(e) => {
                self.error = Some(format!("加载历史记录失败: {}", e));
                self.loading = false;
            }
        }
    }

    pub async fn load_more(&mut self, api_client: &ApiClient) {
        if self.loading || !self.has_more {
            return;
        }

        let Some(cursor) = &self.cursor else {
            return;
        };

        self.loading = true;

        match api_client
            .get_history(
                Some(cursor.max),
                Some(cursor.view_at),
                Some(&cursor.business),
            )
            .await
        {
            Ok(data) => {
                let new_items: Vec<HistoryCard> = data
                    .list
                    .into_iter()
                    .map(|item| HistoryCard {
                        item,
                        cover_protocol: None,
                    })
                    .collect();

                if new_items.is_empty() {
                    self.has_more = false;
                } else {
                    self.cursor = Some(data.cursor);
                    self.items.extend(new_items);
                }
                self.loading = false;
            }
            Err(e) => {
                self.error = Some(format!("加载更多失败: {}", e));
                self.loading = false;
            }
        }
    }

    fn is_near_bottom(&self, visible_rows: usize) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let cols = 4;
        let total_rows = self.items.len().div_ceil(cols);
        let current_row = self.selected / cols;
        current_row + 2 >= self.scroll_offset + visible_rows.min(total_rows)
    }

    /// Start background downloads for visible covers (non-blocking)
    pub fn start_cover_downloads(&mut self) {
        if self.items.is_empty() {
            return;
        }

        // Calculate visible range
        let cols = 4;
        let visible_start = self.scroll_offset * cols;
        let visible_end = (visible_start + 5 * cols).min(self.items.len());

        for idx in visible_start..visible_end {
            if self.items[idx].cover_protocol.is_some() || self.pending_downloads.contains(&idx) {
                continue;
            }

            let Some(cover_url) = self.items[idx].item.get_cover() else {
                continue;
            };

            self.pending_downloads.insert(idx);
            let url = cover_url.to_string();
            let tx = self.cover_tx.clone();
            let picker = Arc::clone(&self.picker);

            tokio::spawn(async move {
                if let Some(img) = Self::download_image(&url).await {
                    let protocol = picker.new_resize_protocol(img);
                    let _ = tx
                        .send(CoverResult {
                            index: idx,
                            protocol,
                        })
                        .await;
                }
            });
        }
    }

    /// Poll for completed cover downloads (non-blocking)
    pub fn poll_cover_results(&mut self) {
        while let Ok(result) = self.cover_rx.try_recv() {
            self.pending_downloads.remove(&result.index);
            if result.index < self.items.len() {
                self.items[result.index].cover_protocol = Some(result.protocol);
            }
        }
    }

    async fn download_image(url: &str) -> Option<DynamicImage> {
        let response = reqwest::get(url).await.ok()?;
        let bytes = response.bytes().await.ok()?;
        image::load_from_memory(&bytes).ok()
    }

    fn visible_rows(&self, height: u16) -> usize {
        let card_height = 12u16;
        (height / card_height).max(1) as usize
    }

    fn selected_row(&self) -> usize {
        self.selected / 4
    }

    fn update_scroll(&mut self, visible_rows: usize) {
        let row = self.selected_row();
        if row < self.scroll_offset {
            self.scroll_offset = row;
        } else if row >= self.scroll_offset + visible_rows {
            self.scroll_offset = row - visible_rows + 1;
        }
    }
}

impl Default for HistoryPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for HistoryPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, _keys: &Keybindings) {
        // Main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " 📜 观看历史 ",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Left);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Loading state
        if self.loading && self.items.is_empty() {
            let loading = Paragraph::new("加载中...")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.fg_muted));
            frame.render_widget(loading, inner);
            return;
        }

        // Error state
        if let Some(ref err) = self.error {
            let error = Paragraph::new(err.as_str())
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.error));
            frame.render_widget(error, inner);
            return;
        }

        // Empty state
        if self.items.is_empty() {
            let empty = Paragraph::new("暂无历史记录")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.fg_muted));
            frame.render_widget(empty, inner);
            return;
        }

        // Render grid
        self.render_grid(frame, inner, theme);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        keys: &crate::storage::Keybindings,
    ) -> Option<AppAction> {
        let cols = 4;
        let total = self.items.len();

        if keys.matches_quit(key) || keys.matches_back(key) {
            return Some(AppAction::BackToList);
        }
        if keys.matches_left(key) {
            if self.selected > 0 {
                self.selected -= 1;
            }
            return None;
        }
        if keys.matches_right(key) {
            if self.selected + 1 < total {
                self.selected += 1;
            }
            return None;
        }
        if keys.matches_up(key) {
            if self.selected >= cols {
                self.selected -= cols;
            }
            return None;
        }
        if keys.matches_down(key) {
            if self.selected + cols < total {
                self.selected += cols;
            }
            // Check if we need to load more
            if self.is_near_bottom(4) {
                return Some(AppAction::LoadMoreHistory);
            }
            return None;
        }
        if keys.matches_confirm(key) {
            if let Some(card) = self.items.get(self.selected) {
                // Only open video detail for video types
                if card.item.is_video() {
                    if let Some(bvid) = card.item.get_bvid() {
                        let aid = card.item.history.oid;
                        return Some(AppAction::OpenVideoDetail(bvid.to_string(), aid));
                    }
                }
            }
            return None;
        }
        if keys.matches_nav_next(key) {
            return Some(AppAction::NavNext);
        }
        if keys.matches_nav_prev(key) {
            return Some(AppAction::NavPrev);
        }
        if keys.matches_next_theme(key) {
            return Some(AppAction::NextTheme);
        }
        None
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        let cols = 4;
        let total = self.items.len();

        match event.kind {
            MouseEventKind::ScrollDown => {
                if self.selected + cols < total {
                    self.selected += cols;
                    if self.is_near_bottom(4) {
                        return Some(AppAction::LoadMoreHistory);
                    }
                }
                None
            }
            MouseEventKind::ScrollUp => {
                if self.selected >= cols {
                    self.selected -= cols;
                }
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let inner = area.inner(Margin::new(1, 1));

                if !inner.contains(ratatui::layout::Position::new(event.column, event.row)) {
                    return None;
                }

                let card_height = 12u16;
                let card_width = inner.width / cols as u16;

                let relative_y = event.row - inner.y;
                let click_row = (relative_y / card_height) as usize;
                let actual_row = self.scroll_offset + click_row;

                let click_col = (event.column.saturating_sub(inner.x) / card_width) as usize;

                let click_idx = actual_row * cols + click_col;

                if click_idx < self.items.len() {
                    let now = Instant::now();
                    let is_double_click = self.last_click_index == Some(click_idx)
                        && self
                            .last_click_time
                            .is_some_and(|t| now.duration_since(t).as_millis() < 500);

                    if is_double_click {
                        self.last_click_time = None;
                        self.last_click_index = None;
                        if let Some(card) = self.items.get(click_idx) {
                            if card.item.is_video() {
                                if let Some(bvid) = card.item.get_bvid() {
                                    let aid = card.item.history.oid;
                                    return Some(AppAction::OpenVideoDetail(bvid.to_string(), aid));
                                }
                            }
                        }
                    } else {
                        self.selected = click_idx;
                        let visible_rows = self.visible_rows(area.height);
                        self.update_scroll(visible_rows);
                        self.last_click_time = Some(now);
                        self.last_click_index = Some(click_idx);
                    }
                }
                None
            }
            _ => None,
        }
    }
}

impl HistoryPage {
    fn render_grid(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let cols = 4;
        let visible_rows = self.visible_rows(area.height);
        self.update_scroll(visible_rows);

        let card_height = 12u16;
        let card_width = area.width / cols as u16;

        let start_idx = self.scroll_offset * cols;
        let end_idx = (start_idx + visible_rows * cols).min(self.items.len());

        for (i, idx) in (start_idx..end_idx).enumerate() {
            let row = i / cols;
            let col = i % cols;

            let x = area.x + (col as u16 * card_width);
            let y = area.y + (row as u16 * card_height);

            if y + card_height > area.y + area.height {
                break;
            }

            let card_area = Rect::new(x, y, card_width, card_height);
            let is_selected = idx == self.selected;

            self.render_history_card(frame, card_area, idx, is_selected, theme);
        }

        // Loading indicator at bottom
        if self.loading && !self.items.is_empty() {
            let loading_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
            let loading = Paragraph::new("加载更多...")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.fg_muted));
            frame.render_widget(loading, loading_area);
        }
    }

    fn render_history_card(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        idx: usize,
        is_selected: bool,
        theme: &Theme,
    ) {
        let card = &mut self.items[idx];

        // Card border
        let border_color = if is_selected {
            theme.bilibili_pink
        } else {
            theme.border_subtle
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(if is_selected {
                BorderType::Thick
            } else {
                BorderType::Rounded
            })
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 4 || inner.height < 4 {
            return;
        }

        // Split into cover area and info area
        let cover_height = 6u16.min(inner.height.saturating_sub(3));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(cover_height), Constraint::Min(3)])
            .split(inner);

        // Render cover
        if let Some(ref mut protocol) = card.cover_protocol {
            let image = StatefulImage::default();
            frame.render_stateful_widget(image, chunks[0], protocol);
        } else {
            // Placeholder with badge
            let badge = card.item.badge.as_deref().unwrap_or("");
            let placeholder = Paragraph::new(badge)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.fg_muted).bg(theme.bg_secondary));
            frame.render_widget(placeholder, chunks[0]);
        }

        // Info area
        let info_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Length(1), // Author + time
                Constraint::Min(0),    // Progress/duration
            ])
            .split(chunks[1]);

        // Title (2 lines)
        let title = &card.item.title;
        let title_style = if is_selected {
            Style::default()
                .fg(theme.fg_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg_primary)
        };
        let title_widget = Paragraph::new(title.as_str())
            .style(title_style)
            .wrap(Wrap { trim: true });
        frame.render_widget(title_widget, info_chunks[0]);

        // Author + view time
        let author = &card.item.author_name;
        let view_time = card.item.format_view_time();
        let info_text = format!("{} · {}", author, view_time);
        let info_widget = Paragraph::new(info_text)
            .style(Style::default().fg(theme.fg_muted))
            .wrap(Wrap { trim: true });
        frame.render_widget(info_widget, info_chunks[1]);

        // Progress / Duration
        if card.item.duration > 0 {
            let progress_text = format!(
                "{} / {}",
                card.item.format_progress(),
                card.item.format_duration()
            );
            let progress_widget =
                Paragraph::new(progress_text).style(Style::default().fg(theme.fg_secondary));
            frame.render_widget(progress_widget, info_chunks[2]);
        }
    }
}
