//! Homepage with video recommendations in a grid layout with cover images

use super::{Component, SearchPage, Theme, shortcut_footer};
use crate::api::client::ApiClient;
use crate::api::recommend::HomeFeed;
use crate::api::recommend::VideoItem;
use crate::application::AppAction;
use crate::storage::Keybindings;
use image::DynamicImage;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind},
    prelude::*,
    widgets::*,
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

/// Video card with cached cover image
pub struct VideoCard {
    pub video: VideoItem,
    pub cover: Option<StatefulProtocol>,
}

/// Message for completed cover download
pub struct CoverResult {
    pub index: usize,
    pub protocol: StatefulProtocol,
}

pub struct HomePage {
    videos: Vec<VideoCard>,
    selected_index: usize,
    loading: bool,
    error_message: Option<String>,
    scroll_row: usize,
    picker: Arc<Picker>,
    columns: usize,
    card_height: u16,
    // Async cover loading
    cover_tx: mpsc::Sender<CoverResult>,
    cover_rx: mpsc::Receiver<CoverResult>,
    pending_downloads: HashSet<usize>,
    fresh_idx: i32,
    loading_more: bool,
    cached_visible_rows: usize,
    footer_notice: Option<String>,
    // Double-click detection
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
    feed: HomeFeed,
    pub search: SearchPage,
    pub focus_sources: bool,
    pub selected_source: usize,
}

impl HomePage {
    fn draw_sources(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = (0..self.source_count())
            .map(|index| {
                ListItem::new(self.source_label(index)).style(if index == self.selected_source {
                    Style::default()
                        .fg(if self.focus_sources {
                            theme.bilibili_pink
                        } else {
                            theme.bilibili_cyan
                        })
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg_secondary)
                })
            })
            .collect::<Vec<_>>();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_subtle))
                    .title(" 首页 "),
            )
            .highlight_symbol("▶ ");
        let mut state = ListState::default().with_selected(Some(self.selected_source));
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn draw_feed(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(2),
            ])
            .split(area);
        let header = Paragraph::new(Line::from(vec![
            Span::styled(" 首页  ", Style::default().fg(theme.fg_accent)),
            Span::styled(
                self.feed.label(),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ),
            if self.loading_more {
                Span::styled("  加载中…", Style::default().fg(theme.warning))
            } else {
                Span::raw("")
            },
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
        frame.render_widget(header, chunks[0]);

        if self.loading {
            frame.render_widget(
                Paragraph::new("⏳ 加载中…")
                    .style(Style::default().fg(theme.warning))
                    .alignment(Alignment::Center),
                chunks[1],
            );
        } else if let Some(error) = &self.error_message {
            frame.render_widget(
                Paragraph::new(format!("❌ {error}"))
                    .style(Style::default().fg(theme.error))
                    .alignment(Alignment::Center),
                chunks[1],
            );
        } else if self.videos.is_empty() {
            frame.render_widget(
                Paragraph::new("📭 暂无推荐视频")
                    .style(Style::default().fg(theme.fg_secondary))
                    .alignment(Alignment::Center),
                chunks[1],
            );
        } else {
            self.render_grid(frame, chunks[1], theme);
        }

        let notice = self.footer_notice.take();
        let mut help = shortcut_footer(
            theme,
            [
                ("↑/↓".into(), "选择视频".into(), theme.fg_accent),
                (
                    format!("{} / {}", keys.page_up, keys.page_down),
                    "翻页".into(),
                    theme.fg_accent,
                ),
                ("←/→".into(), "切换面板".into(), theme.fg_accent),
                (keys.confirm.clone(), "播放".into(), theme.success),
                (keys.search_focus.clone(), "搜索".into(), theme.info),
                (keys.refresh.clone(), "刷新".into(), theme.info),
            ],
        );
        if let Some(notice) = notice {
            help.spans.push(Span::styled(
                format!("  {notice}"),
                Style::default().fg(theme.fg_secondary),
            ));
        }
        frame.render_widget(Paragraph::new(help).alignment(Alignment::Center), chunks[2]);
    }
}

impl HomePage {
    /// 默认列数
    const DEFAULT_COLUMNS: usize = 1;
    /// 卡片高度
    const CARD_HEIGHT: u16 = 8;
    /// 预取缓冲行数（可见区域之外额外下载）
    const PREFETCH_BUFFER_ROWS: usize = 2;
    /// 初始可见行数回退值（首次渲染前使用）
    const INITIAL_VISIBLE_ROWS: usize = 3;

    pub fn new() -> Self {
        // Try to detect terminal graphics protocol (Kitty/Sixel/iTerm2)
        // Fall back to halfblocks if detection fails
        let picker = Arc::new(Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks()));

        // Create channel for background image downloads
        let (cover_tx, cover_rx) = mpsc::channel(32);

        Self {
            videos: Vec::new(),
            selected_index: 0,
            loading: true,
            error_message: None,
            scroll_row: 0,
            picker,
            columns: Self::DEFAULT_COLUMNS,
            card_height: Self::CARD_HEIGHT,
            cover_tx,
            cover_rx,
            pending_downloads: HashSet::new(),
            fresh_idx: 1,
            loading_more: false,
            cached_visible_rows: Self::INITIAL_VISIBLE_ROWS,
            footer_notice: None,
            last_click_time: None,
            last_click_index: None,
            feed: HomeFeed::Recommended,
            search: SearchPage::new(),
            focus_sources: true,
            selected_source: 1,
        }
    }

    pub async fn load_recommendations(&mut self, api_client: &ApiClient) {
        self.loading = true;
        self.error_message = None;
        self.pending_downloads.clear();
        self.fresh_idx = 1;

        match api_client.get_recommendations().await {
            Ok(videos) => {
                self.videos = videos
                    .into_iter()
                    .map(|video| VideoCard { video, cover: None })
                    .collect();
                self.loading = false;
                self.selected_index = 0;
                self.scroll_row = 0;
            }
            Err(e) => {
                self.error_message = Some(format!("加载推荐视频失败: {}", e));
                self.loading = false;
            }
        }
    }

    pub fn begin_loading(&mut self) {
        self.loading = true;
        self.error_message = None;
        self.pending_downloads.clear();
        self.fresh_idx = 1;
    }

    pub fn apply_recommendations(&mut self, feed: HomeFeed, videos: Vec<VideoItem>) {
        if self.feed != feed {
            return;
        }
        self.videos = videos
            .into_iter()
            .map(|video| VideoCard { video, cover: None })
            .collect();
        self.loading = false;
        self.selected_index = 0;
        self.scroll_row = 0;
        self.error_message = None;
    }

    pub fn apply_recommendations_error(&mut self, msg: String) {
        self.error_message = Some(msg);
        self.loading = false;
    }

    pub fn begin_load_more(&mut self) -> Option<i32> {
        if self.loading_more || !matches!(self.feed, HomeFeed::Recommended | HomeFeed::Popular) {
            return None;
        }
        self.loading_more = true;
        self.fresh_idx += 1;
        Some(self.fresh_idx)
    }

    pub fn apply_load_more(&mut self, feed: HomeFeed, videos: Vec<VideoItem>) {
        if self.feed != feed {
            return;
        }
        for video in videos {
            self.videos.push(VideoCard { video, cover: None });
        }
        self.loading_more = false;
    }

    pub fn apply_load_more_error(&mut self) {
        self.fresh_idx -= 1;
        self.loading_more = false;
    }

    pub async fn load_more(&mut self, api_client: &ApiClient) {
        if self.loading_more {
            return;
        }

        self.loading_more = true;
        self.fresh_idx += 1;

        match api_client.get_recommendations_paged(self.fresh_idx).await {
            Ok(videos) => {
                for video in videos {
                    self.videos.push(VideoCard { video, cover: None });
                }
                self.loading_more = false;
            }
            Err(_) => {
                self.fresh_idx -= 1;
                self.loading_more = false;
            }
        }
    }

    pub fn is_near_bottom(&self, visible_rows: usize) -> bool {
        if self.videos.is_empty() {
            return false;
        }
        let current_row = self.selected_row();
        let total_rows = self.total_rows();
        let last_row = total_rows.saturating_sub(1);

        if total_rows <= visible_rows {
            // When all currently loaded rows fit in viewport, trigger load-more at the real bottom.
            current_row >= last_row
        } else {
            // Keep preloading behavior when content is taller than viewport.
            current_row + 2 >= last_row
        }
    }

    /// Start background downloads for visible covers (non-blocking)
    pub fn start_cover_downloads(&mut self) {
        if !self.videos.is_empty() {
            // Calculate visible range using current viewport rows + small buffer
            let start = self.scroll_row * self.columns;
            let prefetch_rows = self.cached_visible_rows + Self::PREFETCH_BUFFER_ROWS;
            let end = (start + self.columns * prefetch_rows).min(self.videos.len());

            for idx in start..end {
                // Skip if already has cover or is pending
                if self.videos[idx].cover.is_some() || self.pending_downloads.contains(&idx) {
                    continue;
                }

                if let Some(pic_url) = self.videos[idx].video.pic.clone() {
                    self.pending_downloads.insert(idx);
                    let tx = self.cover_tx.clone();
                    let picker = Arc::clone(&self.picker);

                    // Spawn background task
                    tokio::spawn(async move {
                        if let Some(img) = Self::download_image(&pic_url).await {
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
        }
        self.search.start_cover_downloads();
    }

    /// Poll for completed cover downloads (non-blocking)
    pub fn poll_cover_results(&mut self) {
        // Try to receive all available results without blocking
        while let Ok(result) = self.cover_rx.try_recv() {
            if result.index < self.videos.len() {
                self.videos[result.index].cover = Some(result.protocol);
                self.pending_downloads.remove(&result.index);
            }
        }
        self.search.poll_cover_results();
    }

    async fn download_image(url: &str) -> Option<DynamicImage> {
        let response = reqwest::get(url).await.ok()?;
        let bytes = response.bytes().await.ok()?;
        image::load_from_memory(&bytes).ok()
    }

    fn visible_rows(&self, height: u16) -> usize {
        let available_height = height.saturating_sub(1);
        (available_height / self.card_height).max(1) as usize
    }

    fn selected_row(&self) -> usize {
        self.selected_index / self.columns
    }

    fn update_scroll(&mut self, visible_rows: usize) {
        let current_row = self.selected_row();
        if current_row < self.scroll_row {
            self.scroll_row = current_row;
        } else if current_row >= self.scroll_row + visible_rows {
            self.scroll_row = current_row - visible_rows + 1;
        }
    }

    fn move_page(&mut self, down: bool) -> bool {
        let Some(last_index) = self.videos.len().checked_sub(1) else {
            return false;
        };
        let page_size = self
            .cached_visible_rows
            .max(1)
            .saturating_mul(self.columns.max(1));
        let old_index = self.selected_index;
        self.selected_index = if down {
            old_index.saturating_add(page_size).min(last_index)
        } else {
            old_index.saturating_sub(page_size)
        };
        if self.selected_index == old_index {
            return false;
        }
        self.update_scroll(self.cached_visible_rows.max(1));
        true
    }

    fn total_rows(&self) -> usize {
        self.videos.len().div_ceil(self.columns)
    }

    pub fn set_footer_notice(&mut self, notice: String) {
        self.footer_notice = Some(notice);
    }

    pub fn feed(&self) -> HomeFeed {
        self.feed
    }

    pub fn search_mut(&mut self) -> &mut SearchPage {
        &mut self.search
    }

    pub fn search_ref(&self) -> &SearchPage {
        &self.search
    }

    pub fn begin_search(&mut self) {
        self.selected_source = 0;
        self.focus_sources = false;
        self.search.input_mode = true;
        self.search.show_hot_list = true;
    }

    pub fn select_source(&mut self, source: usize) {
        self.selected_source = source.min(HomeFeed::ALL.len());
        if self.selected_source == 0 {
            self.begin_search();
        }
    }

    pub fn source_count(&self) -> usize {
        HomeFeed::ALL.len() + 1
    }

    fn source_label(&self, index: usize) -> String {
        if index == 0 {
            "🔍 搜索".to_string()
        } else {
            HomeFeed::ALL[index - 1].label().to_string()
        }
    }

    fn source_feed(&self, index: usize) -> Option<HomeFeed> {
        (index > 0).then(|| HomeFeed::ALL[index - 1])
    }

    pub fn begin_feed_load(&mut self, feed: HomeFeed) {
        self.feed = feed;
        self.selected_source = HomeFeed::ALL
            .iter()
            .position(|candidate| *candidate == feed)
            .map(|index| index + 1)
            .unwrap_or(1);
        self.begin_loading();
        self.videos.clear();
        self.selected_index = 0;
        self.scroll_row = 0;
        self.loading_more = false;
    }
}

impl Default for HomePage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for HomePage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(30)])
            .split(area);
        self.draw_sources(frame, panes[0], theme);

        if self.selected_source == 0 {
            self.search.draw(frame, panes[1], theme, keys);
        } else {
            self.draw_feed(frame, panes[1], theme, keys);
        }
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        keys: &crate::storage::Keybindings,
    ) -> Option<AppAction> {
        // Search input has priority: when the search box is focused, every
        // character (including `i` / `/` which are also `search_focus` shortcuts)
        // must be inserted as text, not as a shortcut.
        if self.selected_source == 0 && self.search.input_mode {
            return self.search.handle_input(key, keys);
        }
        if keys.matches_quit(key) {
            return Some(AppAction::Quit);
        }
        if keys.matches_search_focus(key) {
            self.begin_search();
            return Some(AppAction::None);
        }
        // Tab navigation always wins, including while either pane is loading.
        if keys.matches_nav_next(key) {
            return Some(AppAction::NavNext);
        }
        if keys.matches_nav_prev(key) {
            return Some(AppAction::NavPrev);
        }
        if self.focus_sources {
            if keys.matches_down(key) {
                self.selected_source = (self.selected_source + 1) % self.source_count();
            } else if keys.matches_up(key) {
                self.selected_source = if self.selected_source == 0 {
                    self.source_count() - 1
                } else {
                    self.selected_source - 1
                };
            } else if keys.matches_right(key) || keys.matches_confirm(key) {
                if self.selected_source == 0 {
                    self.begin_search();
                } else if let Some(feed) = self.source_feed(self.selected_source)
                    && feed != self.feed
                {
                    self.focus_sources = false;
                    return Some(AppAction::SwitchHomeFeed(feed));
                } else {
                    self.focus_sources = false;
                }
            }
            return Some(AppAction::None);
        }

        if keys.matches_left(key) {
            self.focus_sources = true;
            if self.selected_source == 0 || self.loading || self.loading_more {
                self.loading = false;
                self.loading_more = false;
                return Some(AppAction::CancelPendingLoads);
            }
            return Some(AppAction::None);
        }
        if self.selected_source == 0 {
            return self.search.handle_input(key, keys);
        }
        if self.loading {
            return Some(AppAction::None);
        }
        if keys.matches_page_down(key) {
            self.move_page(true);
            if self.is_near_bottom(self.cached_visible_rows) && !self.loading_more {
                return Some(AppAction::LoadMoreRecommendations);
            }
            return Some(AppAction::None);
        }
        if keys.matches_page_up(key) {
            self.move_page(false);
            return Some(AppAction::None);
        }
        if keys.matches_down(key) {
            if !self.videos.is_empty() {
                let new_idx = self.selected_index + self.columns;
                if new_idx < self.videos.len() {
                    self.selected_index = new_idx;
                }
                self.update_scroll(self.cached_visible_rows);
                if self.is_near_bottom(self.cached_visible_rows) && !self.loading_more {
                    return Some(AppAction::LoadMoreRecommendations);
                }
            }
            return Some(AppAction::None);
        }
        if keys.matches_up(key) {
            if !self.videos.is_empty() && self.selected_index >= self.columns {
                self.selected_index -= self.columns;
                self.update_scroll(self.cached_visible_rows);
            }
            return Some(AppAction::None);
        }
        if key == KeyCode::Char('u')
            && let Some(mid) = self
                .videos
                .get(self.selected_index)
                .and_then(|card| card.video.owner.as_ref())
                .map(|owner| owner.mid)
        {
            return Some(AppAction::OpenUpPage(mid));
        }
        if keys.matches_confirm(key) || keys.matches_play(key) {
            if let Some(card) = self.videos.get(self.selected_index)
                && let Some(bvid) = &card.video.bvid
            {
                let aid = card.video.id;
                return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
            }
            return Some(AppAction::None);
        }
        if keys.matches_refresh(key) {
            self.loading = true;
            self.videos.clear();
            self.pending_downloads.clear();
            return Some(AppAction::RefreshHome);
        }
        if keys.matches_next_theme(key) {
            return Some(AppAction::NextTheme);
        }
        if keys.matches_open_settings(key) {
            return Some(AppAction::SwitchToSettings);
        }
        Some(AppAction::None)
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(30)])
            .split(area);
        let position = ratatui::layout::Position::new(event.column, event.row);

        if panes[0].contains(position) {
            self.focus_sources = true;
            match event.kind {
                MouseEventKind::ScrollDown => {
                    self.selected_source = (self.selected_source + 1) % self.source_count();
                }
                MouseEventKind::ScrollUp => {
                    self.selected_source = if self.selected_source == 0 {
                        self.source_count() - 1
                    } else {
                        self.selected_source - 1
                    };
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    let row = event.row.saturating_sub(panes[0].y + 1) as usize;
                    if row < self.source_count() {
                        self.selected_source = row;
                        if row == 0 {
                            self.begin_search();
                        } else if let Some(feed) = self.source_feed(row)
                            && feed != self.feed
                        {
                            return Some(AppAction::SwitchHomeFeed(feed));
                        }
                    }
                }
                _ => {}
            }
            return Some(AppAction::None);
        }

        if !panes[1].contains(position) {
            return None;
        }
        self.focus_sources = false;
        if self.selected_source == 0 {
            return self.search.handle_mouse(event, panes[1]);
        }

        match event.kind {
            MouseEventKind::ScrollDown => {
                // Scroll down by one row
                if !self.videos.is_empty() {
                    let new_idx = self.selected_index + self.columns;
                    if new_idx < self.videos.len() {
                        self.selected_index = new_idx;
                        self.update_scroll(self.cached_visible_rows);
                        // Check for pagination only when actually moved
                        if self.is_near_bottom(self.cached_visible_rows) && !self.loading_more {
                            return Some(AppAction::LoadMoreRecommendations);
                        }
                    }
                }
                None
            }
            MouseEventKind::ScrollUp => {
                // Scroll up by one row
                if !self.videos.is_empty() && self.selected_index >= self.columns {
                    self.selected_index -= self.columns;
                    self.update_scroll(self.cached_visible_rows);
                }
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let content_top = panes[1].y + 3;
                let content_bottom = panes[1].bottom().saturating_sub(2);

                if event.row >= content_top && event.row < content_bottom {
                    // Calculate which card was clicked
                    let relative_y = event.row - content_top;
                    let click_row = (relative_y / self.card_height) as usize;
                    let actual_row = self.scroll_row + click_row;

                    let card_width = panes[1].width / self.columns as u16;
                    let click_col = (event.column.saturating_sub(panes[1].x) / card_width) as usize;

                    let click_idx = actual_row * self.columns + click_col.min(self.columns - 1);

                    if click_idx < self.videos.len() {
                        // Check for double-click (same card within 500ms)
                        let now = Instant::now();
                        let is_double_click = self.last_click_index == Some(click_idx)
                            && self
                                .last_click_time
                                .is_some_and(|t| now.duration_since(t).as_millis() < 500);

                        if is_double_click {
                            // Double-click: open video detail
                            self.last_click_time = None;
                            self.last_click_index = None;
                            if let Some(card) = self.videos.get(click_idx)
                                && let Some(bvid) = &card.video.bvid
                            {
                                let aid = card.video.id;
                                return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
                            }
                        } else {
                            // Single click: select card and record for potential double-click
                            self.selected_index = click_idx;
                            self.update_scroll(self.cached_visible_rows);
                            self.last_click_time = Some(now);
                            self.last_click_index = Some(click_idx);
                        }
                    }
                }
                None
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                // Middle click opens video detail
                if let Some(card) = self.videos.get(self.selected_index)
                    && let Some(bvid) = &card.video.bvid
                {
                    let aid = card.video.id;
                    return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
                }
                None
            }
            _ => None,
        }
    }
}

impl HomePage {
    fn render_grid(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let visible_rows = self.visible_rows(area.height);
        self.cached_visible_rows = visible_rows;

        let row_constraints: Vec<Constraint> = (0..visible_rows)
            .map(|_| Constraint::Min(self.card_height))
            .collect();

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(area);

        // Collect all card areas first
        let mut card_areas: Vec<(usize, Rect)> = Vec::new();

        for (row_offset, row_area) in rows.iter().enumerate() {
            let actual_row = self.scroll_row + row_offset;
            let start_idx = actual_row * self.columns;

            if start_idx >= self.videos.len() {
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
                let video_idx = start_idx + col_idx;
                if video_idx >= self.videos.len() {
                    break;
                }
                card_areas.push((video_idx, *col_area));
            }
        }

        // Now render each card with mutable access
        for (video_idx, col_area) in card_areas {
            let is_selected = video_idx == self.selected_index;
            self.render_video_card(frame, col_area, video_idx, is_selected, theme);
        }
    }

    fn render_video_card(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        video_idx: usize,
        is_selected: bool,
        theme: &Theme,
    ) {
        // Enhanced border styling
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

        let title_span = if is_selected {
            Span::styled(
                " ▶ ",
                Style::default()
                    .fg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("")
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
            .title(title_span);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let card_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(30)])
            .split(inner);

        // Cover area - render with StatefulImage
        let cover_area = card_chunks[0];
        if let Some(cover) = &mut self.videos[video_idx].cover {
            // Render actual image using StatefulImage
            let image_widget = StatefulImage::new();
            frame.render_stateful_widget(image_widget, cover_area, cover);
        } else {
            // Loading placeholder with spinner animation hint
            let is_pending = self.pending_downloads.contains(&video_idx);
            let placeholder_text = if is_pending {
                "📺 加载中..."
            } else {
                "📺"
            };
            let placeholder = Paragraph::new(placeholder_text)
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center);
            frame.render_widget(placeholder, cover_area);
        }

        // Video info with enhanced styling
        let info_area = card_chunks[1];
        let card = &self.videos[video_idx];

        let title = card.video.title.as_deref().unwrap_or("无标题");
        let author = card.video.author_name();
        let views = card.video.format_views();
        let duration = card.video.format_duration();

        let max_title_len = (info_area.width as usize).saturating_sub(2);
        let display_title: String = if title.chars().count() > max_title_len {
            title
                .chars()
                .take(max_title_len.saturating_sub(3))
                .collect::<String>()
                + "..."
        } else {
            title.to_string()
        };

        // Multi-styled info text
        let title_style = if is_selected {
            Style::default()
                .fg(theme.fg_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg_secondary)
        };

        let meta_style = Style::default().fg(theme.fg_secondary);

        let follower = card
            .video
            .owner
            .as_ref()
            .and_then(|owner| owner.follower)
            .map(format_count)
            .unwrap_or_else(|| "-".to_string());
        let danmaku = card
            .video
            .stat
            .as_ref()
            .and_then(|stat| stat.danmaku)
            .map(format_count)
            .unwrap_or_else(|| "-".to_string());
        let replies = card
            .video
            .stat
            .as_ref()
            .and_then(|stat| stat.reply)
            .map(format_count)
            .unwrap_or_else(|| "-".to_string());
        let info_text = Text::from(vec![
            Line::from(Span::styled(&display_title, title_style)),
            Line::from(vec![
                Span::styled("UP  ", meta_style),
                Span::styled(author, Style::default().fg(theme.bilibili_cyan)),
                Span::styled(format!("  ·  {follower} 关注"), meta_style),
            ]),
            Line::from(vec![
                Span::styled(format!("▶ {views}"), meta_style),
                Span::styled(format!("   弹幕 {danmaku}"), meta_style),
                Span::styled(format!("   评论 {replies}"), meta_style),
                Span::styled(format!("   {duration}"), Style::default().fg(theme.success)),
            ]),
        ]);

        let info = Paragraph::new(info_text).wrap(Wrap { trim: true });
        frame.render_widget(info, info_area);
    }
}

fn format_count(value: i64) -> String {
    if value >= 10_000 {
        format!("{:.1}万", value as f64 / 10_000.0)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_pane_switch_preempts_loading() {
        let mut page = HomePage::new();
        page.focus_sources = false;
        page.loading = true;
        let keys = Keybindings::default();
        assert!(matches!(
            page.handle_input(KeyCode::Left, &keys),
            Some(AppAction::CancelPendingLoads)
        ));
        assert!(page.focus_sources);
    }

    #[test]
    fn home_is_a_single_column_list() {
        let page = HomePage::new();
        assert_eq!(page.columns, 1);
    }

    #[test]
    fn home_contains_search_source() {
        let mut page = HomePage::new();
        assert_eq!(page.source_count(), HomeFeed::ALL.len() + 1);
        assert_eq!(page.feed(), HomeFeed::Recommended);
        page.begin_search();
        assert_eq!(page.selected_source, 0);
        assert!(page.search.input_mode);
    }
}
