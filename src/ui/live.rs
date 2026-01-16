//! Live streaming recommendations page with grid layout

use super::{Component, Theme};
use crate::api::client::ApiClient;
use crate::api::live::LiveRoom;
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

/// Live card with cached cover image
struct LiveCard {
    pub room: LiveRoom,
    pub cover_image: Option<StatefulProtocol>,
}

/// Message for completed cover download
struct CoverResult {
    room_id: i64,
    protocol: Option<StatefulProtocol>,
}

pub struct LivePage {
    rooms: Vec<LiveCard>,
    selected_index: usize,
    columns: usize,
    scroll_offset: usize,
    loading: bool,
    error: Option<String>,
    last_area_height: u16,

    // Image loading state
    picker: Arc<Picker>,
    cover_tx: mpsc::Sender<CoverResult>,
    cover_rx: mpsc::Receiver<CoverResult>,
    pending_downloads: HashSet<i64>,
    last_load_time: Option<Instant>,
    loading_more: bool,
}

impl LivePage {
    /// 默认列数
    const DEFAULT_COLUMNS: usize = 3;
    /// 卡片高度
    const CARD_HEIGHT: u16 = 10;
    /// 预加载行数
    const PREFETCH_ROWS: usize = 4;
    /// 默认可见行数
    const DEFAULT_VISIBLE_ROWS: usize = 3;

    pub fn new() -> Self {
        let picker = Arc::new(Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks()));
        let (tx, rx) = mpsc::channel(50);
        Self {
            rooms: Vec::new(),
            selected_index: 0,
            columns: Self::DEFAULT_COLUMNS,
            scroll_offset: 0,
            loading: false,
            error: None,
            last_area_height: 0,
            picker,
            cover_tx: tx,
            cover_rx: rx,
            pending_downloads: HashSet::new(),
            last_load_time: None,
            loading_more: false,
        }
    }

    pub async fn load_recommendations(&mut self, api_client: &ApiClient) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;

        match api_client.get_live_recommendations().await {
            Ok(rooms) => {
                self.rooms = rooms
                    .into_iter()
                    .map(|room| LiveCard {
                        room,
                        cover_image: None,
                    })
                    .collect();
                self.loading = false;
                self.selected_index = 0;
                self.scroll_offset = 0;
                self.last_load_time = Some(Instant::now());
            }
            Err(e) => {
                self.error = Some(format!("加载直播推荐失败: {}", e));
                self.loading = false;
            }
        }
    }

    pub async fn refresh(&mut self, api_client: &ApiClient) {
        self.rooms.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.pending_downloads.clear();
        self.load_recommendations(api_client).await;
    }

    pub async fn load_more(&mut self, api_client: &ApiClient) {
        if self.loading_more {
            return;
        }

        self.loading_more = true;

        match api_client.get_live_recommendations().await {
            Ok(rooms) => {
                for room in rooms {
                    self.rooms.push(LiveCard {
                        room,
                        cover_image: None,
                    });
                }
                self.loading_more = false;
            }
            Err(_) => {
                self.loading_more = false;
            }
        }
    }

    pub fn is_near_bottom(&self, visible_rows: usize) -> bool {
        if self.rooms.is_empty() {
            return false;
        }
        let current_row = self.selected_row();
        let total = self.total_rows();
        current_row + 2 >= total.saturating_sub(1) && total > visible_rows
    }

    /// Start background downloads for visible covers
    fn start_cover_downloads(&mut self) {
        let visible_rows = self.visible_rows(self.last_area_height);
        let prefetch_rows = visible_rows + Self::PREFETCH_ROWS;
        let start_idx = self.scroll_offset * self.columns;
        let end_idx = std::cmp::min(start_idx + prefetch_rows * self.columns, self.rooms.len());

        for idx in start_idx..end_idx {
            let room = &self.rooms[idx];
            if room.cover_image.is_some() || self.pending_downloads.contains(&room.room.roomid) {
                continue;
            }

            let cover_url = if !room.room.cover.is_empty() {
                room.room.cover.clone()
            } else if !room.room.keyframe.is_empty() {
                room.room.keyframe.clone()
            } else {
                continue;
            };

            self.pending_downloads.insert(room.room.roomid);
            let room_id = room.room.roomid;
            let tx = self.cover_tx.clone();
            let picker = Arc::clone(&self.picker);

            tokio::spawn(async move {
                let protocol = Self::download_image(&cover_url)
                    .await
                    .map(|img| picker.new_resize_protocol(img));
                let _ = tx.send(CoverResult { room_id, protocol }).await;
            });
        }
    }

    /// Poll for completed cover downloads
    fn poll_cover_results(&mut self) {
        while let Ok(result) = self.cover_rx.try_recv() {
            // Handle cover download result
            self.pending_downloads.remove(&result.room_id);
            if let Some(protocol) = result.protocol {
                if let Some(card) = self
                    .rooms
                    .iter_mut()
                    .find(|c| c.room.roomid == result.room_id)
                {
                    card.cover_image = Some(protocol);
                }
            }
        }
    }

    async fn download_image(url: &str) -> Option<DynamicImage> {
        let response = reqwest::get(url).await.ok()?;
        let bytes = response.bytes().await.ok()?;
        image::load_from_memory(&bytes).ok()
    }

    fn visible_rows(&self, height: u16) -> usize {
        (height as usize / Self::CARD_HEIGHT as usize).max(1)
    }

    fn selected_row(&self) -> usize {
        self.selected_index / self.columns
    }

    fn update_scroll(&mut self, visible_rows: usize) {
        let selected_row = self.selected_row();
        if selected_row < self.scroll_offset {
            self.scroll_offset = selected_row;
        } else if selected_row >= self.scroll_offset + visible_rows {
            self.scroll_offset = selected_row.saturating_sub(visible_rows - 1);
        }
    }

    fn total_rows(&self) -> usize {
        self.rooms.len().div_ceil(self.columns)
    }
}

impl Default for LivePage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for LivePage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        self.last_area_height = area.height;
        self.poll_cover_results();
        self.start_cover_downloads();

        // Split area into header/content/footer like home page
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(2),
            ])
            .split(area);

        // Header
        let header = Paragraph::new("📺 直播推荐")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_subtle))
                    .title(Span::styled(
                        " 直播 ",
                        Style::default()
                            .fg(theme.fg_accent)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .style(
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        frame.render_widget(header, chunks[0]);

        // Content area
        if self.loading && self.rooms.is_empty() {
            let loading = Paragraph::new("⏳ 加载中...")
                .style(
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::ITALIC),
                )
                .alignment(Alignment::Center);
            frame.render_widget(loading, chunks[1]);
            return;
        }

        if let Some(ref err) = self.error {
            let error = Paragraph::new(format!("❌ 加载失败: {}", err))
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center);
            frame.render_widget(error, chunks[1]);
            return;
        }

        if self.rooms.is_empty() {
            let empty = Paragraph::new("📭 暂无直播推荐")
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center);
            frame.render_widget(empty, chunks[1]);
            return;
        }

        // Render grid
        self.render_grid(frame, chunks[1], theme);

        // Footer with hints
        let nav_keys = keys.get_nav_keys_display();
        let hints = Paragraph::new(Line::from(vec![
            Span::styled(" [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                format!("{}/↑↓←→", nav_keys),
                Style::default()
                    .fg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("导航", Style::default().fg(theme.fg_secondary)),
            Span::styled("  [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                &keys.confirm,
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("/", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                &keys.play,
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("进入", Style::default().fg(theme.fg_secondary)),
            Span::styled("  [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                &keys.refresh,
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("刷新", Style::default().fg(theme.fg_secondary)),
            Span::styled("  [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                &keys.next_theme,
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("切换主题", Style::default().fg(theme.fg_secondary)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_subtle)),
        )
        .alignment(Alignment::Center);
        frame.render_widget(hints, chunks[2]);
    }

    fn handle_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        // Handle global keybindings first
        if keys.matches_quit(key) {
            return Some(AppAction::Quit);
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
        if keys.matches_open_settings(key) {
            return Some(AppAction::SwitchToSettings);
        }
        if keys.matches_refresh(key) {
            return Some(AppAction::RefreshLive);
        }

        if self.rooms.is_empty() {
            return Some(AppAction::None);
        }

        if keys.matches_up(key) {
            if self.selected_index >= self.columns {
                self.selected_index -= self.columns;
                self.update_scroll(Self::DEFAULT_VISIBLE_ROWS);
            }
            return Some(AppAction::None);
        }
        if keys.matches_down(key) {
            let new_idx = self.selected_index + self.columns;
            if new_idx < self.rooms.len() {
                self.selected_index = new_idx;
                self.update_scroll(Self::DEFAULT_VISIBLE_ROWS);
            }
            // Check for pagination
            if self.is_near_bottom(Self::DEFAULT_VISIBLE_ROWS) && !self.loading_more {
                return Some(AppAction::LoadMoreLive);
            }
            return Some(AppAction::None);
        }
        if keys.matches_left(key) {
            if self.selected_index > 0 {
                self.selected_index -= 1;
                self.update_scroll(Self::DEFAULT_VISIBLE_ROWS);
            }
            return Some(AppAction::None);
        }
        if keys.matches_right(key) {
            if self.selected_index + 1 < self.rooms.len() {
                self.selected_index += 1;
                self.update_scroll(Self::DEFAULT_VISIBLE_ROWS);
            }
            return Some(AppAction::None);
        }
        if keys.matches_confirm(key) || keys.matches_play(key) {
            if let Some(card) = self.rooms.get(self.selected_index) {
                return Some(AppAction::OpenLiveDetail(card.room.roomid));
            }
        }
        Some(AppAction::None)
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Calculate which card was clicked
                let x = event.column.saturating_sub(area.x + 1);
                let y = event.row.saturating_sub(area.y + 1);

                let card_width = area.width.saturating_sub(2) / self.columns as u16;
                let col = (x / card_width) as usize;
                let row = (y / Self::CARD_HEIGHT) as usize + self.scroll_offset;

                let idx = row * self.columns + col;
                if idx < self.rooms.len() && col < self.columns {
                    self.selected_index = idx;
                }
                None
            }
            MouseEventKind::ScrollDown => {
                let new_idx = self.selected_index + self.columns;
                if new_idx < self.rooms.len() {
                    self.selected_index = new_idx;
                    // Check for pagination
                    if self.is_near_bottom(Self::DEFAULT_VISIBLE_ROWS) && !self.loading_more {
                        return Some(AppAction::LoadMoreLive);
                    }
                }
                None
            }
            MouseEventKind::ScrollUp => {
                if self.selected_index >= self.columns {
                    self.selected_index -= self.columns;
                }
                None
            }
            _ => None,
        }
    }
}

impl LivePage {
    fn render_grid(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let visible_rows = self.visible_rows(area.height);
        self.update_scroll(visible_rows);

        let row_constraints: Vec<Constraint> = (0..visible_rows)
            .map(|_| Constraint::Min(Self::CARD_HEIGHT))
            .collect();

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(area);

        // Collect all card areas first
        let mut card_areas: Vec<(usize, Rect)> = Vec::new();

        for (row_offset, row_area) in rows.iter().enumerate() {
            let actual_row = self.scroll_offset + row_offset;
            let start_idx = actual_row * self.columns;

            if start_idx >= self.rooms.len() {
                break;
            }

            let col_constraints: Vec<Constraint> = (0..self.columns)
                .map(|_| Constraint::Ratio(1, self.columns as u32))
                .collect();

            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(col_constraints)
                .split(*row_area);

            for (col_idx, col_area) in cols.iter().enumerate() {
                let room_idx = start_idx + col_idx;
                if room_idx >= self.rooms.len() {
                    break;
                }
                card_areas.push((room_idx, *col_area));
            }
        }

        // Now render each card with mutable access
        for (room_idx, col_area) in card_areas {
            let is_selected = room_idx == self.selected_index;
            self.render_live_card(frame, col_area, room_idx, is_selected, theme);
        }
    }

    fn render_live_card(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        idx: usize,
        is_selected: bool,
        theme: &Theme,
    ) {
        let card = &mut self.rooms[idx];
        let room = &card.room;

        // Enhanced border styling (consistent with home page)
        let (border_style, border_type) = if is_selected {
            (
                Style::default()
                    .fg(theme.border_focused)
                    .add_modifier(Modifier::BOLD),
                BorderType::Rounded,
            )
        } else {
            (
                Style::default().fg(theme.border_unfocused),
                BorderType::Rounded,
            )
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into cover and info
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Cover
                Constraint::Min(2),    // Info
            ])
            .split(inner);

        // Render cover image or placeholder
        if let Some(ref mut protocol) = card.cover_image {
            let image = StatefulImage::new();
            frame.render_stateful_widget(image, chunks[0], protocol);
        } else {
            let placeholder = Paragraph::new("🎬")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.fg_muted));
            frame.render_widget(placeholder, chunks[0]);
        }

        // Info section
        let title = room.title.chars().take(20).collect::<String>();
        let title_style = if is_selected {
            Style::default()
                .fg(theme.bilibili_pink)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg_primary)
        };

        // Format online count
        let online_text = if room.online >= 10000 {
            format!("👁 {:.1}万", room.online as f64 / 10000.0)
        } else {
            format!("👁 {}", room.online)
        };

        let info_lines = vec![
            Line::from(Span::styled(title, title_style)),
            Line::from(vec![Span::styled(
                &room.uname,
                Style::default().fg(theme.fg_secondary),
            )]),
            Line::from(vec![
                Span::styled(
                    format!("{} | ", room.area_v2_name),
                    Style::default().fg(theme.fg_muted),
                ),
                Span::styled(online_text, Style::default().fg(theme.fg_accent)),
            ]),
        ];

        let info = Paragraph::new(info_lines).wrap(Wrap { trim: true });
        frame.render_widget(info, chunks[1]);
    }
}
