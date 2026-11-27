//! Search page with video card grid display

use super::video_card::{VideoCard, VideoCardGrid};
use super::{Component, Theme, shortcut_footer};
use crate::api::client::ApiClient;
use crate::api::search::{HotwordItem, MediaSearchItem, SearchKind, SearchVideoItem};
use crate::application::AppAction;
use crate::storage::Keybindings;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind},
    prelude::*,
    widgets::*,
};
use std::time::Instant;

/// What a search result card opens when confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchResultTarget {
    Video { bvid: String, aid: i64 },
    Bangumi { season_id: i64 },
}

impl SearchResultTarget {
    fn to_action(&self) -> AppAction {
        match self {
            Self::Video { bvid, aid } if !bvid.is_empty() && *aid > 0 => {
                AppAction::OpenVideoDetail(bvid.clone(), *aid)
            }
            Self::Bangumi { season_id } if *season_id > 0 => {
                AppAction::OpenBangumiDetail(*season_id)
            }
            _ => AppAction::None,
        }
    }
}

pub struct SearchPage {
    pub query: String,
    pub search_kind: SearchKind,
    pub result_targets: Vec<SearchResultTarget>,
    pub grid: VideoCardGrid,
    pub loading: bool,
    pub error_message: Option<String>,
    pub input_mode: bool,
    pub hotwords: Vec<HotwordItem>,
    pub hotword_error: Option<String>,
    pub hotword_loading: bool,
    pub show_hot_list: bool,
    hotwords_enabled: bool,
    hot_selected: Option<u32>,
    pub page: i32,
    pub total_results: i32,
    pub loading_more: bool,
    last_click: Option<Instant>,
}

impl SearchPage {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            search_kind: SearchKind::Video,
            result_targets: Vec::new(),
            grid: VideoCardGrid::new_list(),
            loading: false,
            error_message: None,
            input_mode: true,
            hotwords: Vec::new(),
            hotword_error: None,
            hotword_loading: false,
            show_hot_list: true,
            hotwords_enabled: true,
            hot_selected: None,
            page: 1,
            total_results: 0,
            loading_more: false,
            last_click: None,
        }
    }

    pub fn disable_hotwords(&mut self) {
        self.hotwords_enabled = false;
        self.show_hot_list = false;
        self.hotword_loading = false;
        self.hotwords.clear();
        self.hot_selected = None;
        self.hotword_error = None;
    }

    pub fn set_results(&mut self, results: Vec<SearchVideoItem>, total: i32) {
        self.clear_results();
        self.append_results(results);
        self.total_results = total;
        self.loading = false;
        self.input_mode = false;
        self.show_hot_list = false;
        self.error_message = None;
    }

    pub fn append_results(&mut self, results: Vec<SearchVideoItem>) {
        for item in results {
            let card = VideoCard::new(
                item.bvid.clone(),
                item.aid,
                item.display_title(),
                item.author_name().to_string(),
                item.format_play(),
                item.duration.clone().unwrap_or_default(),
                item.cover_url(),
            )
            .with_uploader_mid(item.mid);
            self.grid.add_card(card);
            self.result_targets.push(SearchResultTarget::Video {
                bvid: item.bvid.unwrap_or_default(),
                aid: item.aid.unwrap_or(0),
            });
        }
        self.loading_more = false;
    }

    /// Replace the grid with bangumi / movie-TV results for `kind`.
    pub fn set_media_results(
        &mut self,
        kind: SearchKind,
        results: Vec<MediaSearchItem>,
        total: i32,
    ) {
        self.search_kind = kind;
        self.clear_results();
        self.append_media_results(results);
        self.total_results = total;
        self.loading = false;
        self.input_mode = false;
        self.show_hot_list = false;
        self.error_message = None;
    }

    pub fn append_media_results(&mut self, results: Vec<MediaSearchItem>) {
        for item in results {
            let Some(season_id) = item.resolved_season_id() else {
                continue;
            };
            let card = VideoCard::new(
                None,
                None,
                item.display_title(),
                item.display_subtitle(),
                item.score_text(),
                item.meta_text(),
                item.cover_url(),
            );
            self.grid.add_card(card);
            self.result_targets
                .push(SearchResultTarget::Bangumi { season_id });
        }
        self.loading_more = false;
    }

    /// Drop the current result grid and its targets.
    pub fn clear_results(&mut self) {
        self.grid.clear();
        self.result_targets.clear();
    }

    pub fn set_search_kind(&mut self, kind: SearchKind) {
        self.search_kind = kind;
    }

    /// Apply a kind switch; returns the keyword when a fresh search must run.
    pub fn begin_kind_switch(&mut self, kind: SearchKind) -> Option<String> {
        self.set_search_kind(kind);
        self.page = 1;
        self.error_message = None;
        if self.query.trim().is_empty() || self.show_hot_list {
            return None;
        }
        self.loading = true;
        self.clear_results();
        Some(self.query.clone())
    }

    /// Enter on the selected card opens the matching detail page.
    fn selected_target_action(&self) -> AppAction {
        match self.result_targets.get(self.grid.selected_index) {
            Some(target) => target.to_action(),
            None => AppAction::None,
        }
    }

    fn open_target_at(&self, index: usize) -> Option<AppAction> {
        self.result_targets
            .get(index)
            .map(SearchResultTarget::to_action)
            .filter(|action| !matches!(action, AppAction::None))
    }

    fn empty_message(&self) -> &'static str {
        if self.query.is_empty() {
            return "输入关键词开始搜索";
        }
        match self.search_kind {
            SearchKind::Video => "没有找到相关视频",
            SearchKind::MediaBangumi => "没有找到相关番剧",
            SearchKind::MediaFt => "没有找到相关影视",
        }
    }

    /// Tab / section shortcuts switch the search kind outside text input mode.
    fn kind_shortcut(&self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        let kind = if keys.matches_tab_1(key) {
            SearchKind::Video
        } else if keys.matches_tab_2(key) {
            SearchKind::MediaBangumi
        } else if keys.matches_tab_3(key) {
            SearchKind::MediaFt
        } else if keys.matches_section_prev(key) {
            self.search_kind.cycle(false)
        } else if keys.matches_section_next(key) {
            self.search_kind.cycle(true)
        } else {
            return None;
        };
        (kind != self.search_kind).then_some(AppAction::SwitchSearchKind(kind))
    }

    fn kind_bar_labels() -> Vec<(SearchKind, String)> {
        SearchKind::ALL
            .iter()
            .enumerate()
            .map(|(index, kind)| (*kind, format!("[{}] {}", index + 1, kind.label())))
            .collect()
    }

    fn kind_bar_ranges() -> Vec<(SearchKind, u16, u16)> {
        let mut ranges = Vec::new();
        let mut cursor = 1u16;
        for (index, (kind, text)) in Self::kind_bar_labels().into_iter().enumerate() {
            if index > 0 {
                cursor += 3;
            }
            let start = cursor;
            cursor += Span::raw(text).width() as u16;
            ranges.push((kind, start, cursor));
        }
        ranges
    }

    fn render_kind_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut spans = vec![Span::raw(" ")];
        for (index, (kind, text)) in Self::kind_bar_labels().into_iter().enumerate() {
            if index > 0 {
                spans.push(Span::raw("   "));
            }
            let style = if kind == self.search_kind {
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg_secondary)
            };
            spans.push(Span::styled(text, style));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
        self.loading = false;
        self.loading_more = false;
        self.show_hot_list = false;
    }

    pub fn start_hotword_loading(&mut self) {
        self.hotword_loading = true;
        self.hotword_error = None;
        self.hot_selected = None;
    }

    pub fn set_hotwords(&mut self, hotwords: Vec<HotwordItem>) {
        self.hotwords = hotwords;
        self.hotword_loading = false;
        self.hotword_error = None;
        self.hot_selected = if self.hotwords.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    pub fn set_hotword_error(&mut self, msg: String) {
        self.hotword_error = Some(msg);
        self.hotword_loading = false;
    }

    pub async fn load_more(&mut self, api_client: &ApiClient) {
        if self.loading_more || self.query.is_empty() || self.show_hot_list {
            return;
        }

        // Check if we have more results
        if self.grid.cards.len() >= self.total_results as usize {
            return;
        }

        self.loading_more = true;
        self.page += 1;

        match api_client.search_videos(&self.query, self.page).await {
            Ok(data) => {
                let results = data.result.unwrap_or_default();
                if results.is_empty() {
                    self.page -= 1;
                }
                self.append_results(results);
            }
            Err(_) => {
                self.page -= 1;
                self.loading_more = false;
            }
        }
    }

    pub fn poll_cover_results(&mut self) {
        self.grid.poll_cover_results();
    }

    pub fn start_cover_downloads(&mut self) {
        self.grid.start_cover_downloads();
    }

    fn hot_selected_index(&self) -> usize {
        self.hot_selected.unwrap_or(0) as usize
    }

    fn select_hotword(&mut self, idx: usize) {
        if idx < self.hotwords.len() {
            self.hot_selected = Some(idx as u32);
        }
    }

    fn search_selected_hotword(&mut self) -> Option<AppAction> {
        if let Some(idx) = self.hot_selected
            && let Some(item) = self.hotwords.get(idx as usize)
            && let Some(keyword) = item.keyword_text()
        {
            self.query = keyword.clone();
            self.loading = true;
            self.page = 1;
            self.show_hot_list = false;
            return Some(AppAction::Search(keyword));
        }
        None
    }

    fn draw_hot_list(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " 热搜榜 ",
                Style::default().fg(theme.bilibili_pink),
            ));

        if self.hotword_loading {
            let loading = Paragraph::new("⏳ 正在获取热搜...")
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center)
                .block(block);
            frame.render_widget(loading, area);
            return;
        }

        if let Some(err) = &self.hotword_error {
            let error_widget = Paragraph::new(format!("❌ {}", err))
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center)
                .block(block);
            frame.render_widget(error_widget, area);
            return;
        }

        if self.hotwords.is_empty() {
            let empty = Paragraph::new("暂无热搜数据")
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center)
                .block(block);
            frame.render_widget(empty, area);
            return;
        }

        let items: Vec<ListItem> = self
            .hotwords
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let mut spans = vec![
                    Span::styled(
                        format!("{:>2}. ", idx + 1),
                        Style::default().fg(theme.fg_muted),
                    ),
                    Span::styled(item.display_text(), Style::default().fg(theme.fg_primary)),
                ];

                if let Some(badge) = item.badge() {
                    spans.push(Span::styled(
                        format!(" [{}]", badge),
                        Style::default().fg(theme.bilibili_pink),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().fg(theme.bilibili_pink))
            .highlight_symbol("▶ ");

        let mut state =
            ListState::default().with_selected(self.hot_selected.map(|idx| idx as usize));
        frame.render_stateful_widget(list, area, &mut state);
    }
}

impl Default for SearchPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for SearchPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search input
                Constraint::Length(1), // Search kind switcher
                Constraint::Min(10),   // Results grid
                Constraint::Length(2), // Help
            ])
            .split(area);

        // Search input
        let input_style = if self.input_mode {
            Style::default().fg(theme.warning)
        } else {
            Style::default().fg(theme.fg_primary)
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.input_mode {
                Style::default().fg(theme.bilibili_pink)
            } else {
                Style::default().fg(theme.border_subtle)
            })
            .title(Span::styled(
                format!(" 🔍 搜索{} ", self.search_kind.label()),
                Style::default().fg(theme.bilibili_pink),
            ));

        let cursor_char = if self.input_mode { "▌" } else { "" };
        let input = Paragraph::new(format!("{}{}", self.query, cursor_char))
            .style(input_style)
            .block(input_block);
        frame.render_widget(input, chunks[0]);

        // Search kind switcher
        self.render_kind_bar(frame, chunks[1], theme);

        // Results
        if self.show_hot_list {
            self.draw_hot_list(frame, chunks[2], theme);
        } else if self.loading {
            let loading = Paragraph::new("⏳ 搜索中...")
                .style(Style::default().fg(theme.warning))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.border_unfocused))
                        .title(Span::styled(
                            format!(" 结果 ({}) ", self.total_results),
                            Style::default().fg(theme.fg_secondary),
                        )),
                );
            frame.render_widget(loading, chunks[2]);
        } else if let Some(error) = &self.error_message {
            let error_widget = Paragraph::new(format!("❌ {}", error))
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.border_unfocused)),
                );
            frame.render_widget(error_widget, chunks[2]);
        } else if self.grid.cards.is_empty() {
            let empty = Paragraph::new(self.empty_message())
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.border_unfocused)),
                );
            frame.render_widget(empty, chunks[2]);
        } else {
            // Render with header
            let header = Paragraph::new(Line::from(vec![
                Span::styled(" 搜索结果 ", Style::default().fg(theme.bilibili_pink)),
                Span::styled(
                    format!("({}/{})", self.grid.cards.len(), self.total_results),
                    Style::default().fg(theme.fg_muted),
                ),
                if self.loading_more {
                    Span::styled(" 加载中...", Style::default().fg(theme.warning))
                } else {
                    Span::raw("")
                },
            ]))
            .block(
                Block::default()
                    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_subtle)),
            );

            let header_area = Rect {
                height: 2,
                ..chunks[2]
            };
            let grid_area = Rect {
                y: chunks[2].y + 2,
                height: chunks[2].height.saturating_sub(2),
                ..chunks[2]
            };

            frame.render_widget(header, header_area);
            self.grid.render(frame, grid_area, theme);
        }

        // Help
        let help = if self.input_mode {
            shortcut_footer(
                theme,
                [
                    (keys.confirm.clone(), "搜索".into(), theme.success),
                    (keys.back.clone(), "取消".into(), theme.info),
                    (keys.nav_next_page.clone(), "导航".into(), theme.fg_accent),
                ],
            )
        } else {
            shortcut_footer(
                theme,
                [
                    (
                        format!(
                            "{}/{}",
                            keys.get_arrow_keys_display(),
                            keys.get_nav_keys_display()
                        ),
                        "导航".into(),
                        theme.fg_accent,
                    ),
                    (
                        format!("{}/{}", keys.page_up, keys.page_down),
                        "翻页".into(),
                        theme.fg_accent,
                    ),
                    (keys.confirm.clone(), "详情".into(), theme.success),
                    (keys.search_focus.clone(), "搜索".into(), theme.info),
                    (
                        format!("{}/{}", keys.tab_1, keys.section_next),
                        "分类".into(),
                        theme.info,
                    ),
                    (keys.nav_next_page.clone(), "切换".into(), theme.info),
                ],
            )
        };
        let help = Paragraph::new(help).alignment(Alignment::Center);
        frame.render_widget(help, chunks[3]);
    }

    fn handle_input(
        &mut self,
        key: KeyCode,
        keys: &crate::storage::Keybindings,
    ) -> Option<AppAction> {
        if self.input_mode {
            match key {
                KeyCode::Char(c) => {
                    self.query.push(c);
                    self.show_hot_list = self.hotwords_enabled;
                    if self.hot_selected.is_none() && !self.hotwords.is_empty() {
                        self.hot_selected = Some(0);
                    }
                    Some(AppAction::None)
                }
                KeyCode::Backspace => {
                    self.query.pop();
                    self.show_hot_list = self.hotwords_enabled;
                    if self.hot_selected.is_none() && !self.hotwords.is_empty() {
                        self.hot_selected = Some(0);
                    }
                    Some(AppAction::None)
                }
                KeyCode::Up => {
                    if self.show_hot_list && !self.hotwords.is_empty() {
                        let len = self.hotwords.len();
                        let current = self.hot_selected_index();
                        let next = if current == 0 { len - 1 } else { current - 1 };
                        self.hot_selected = Some(next as u32);
                    }
                    Some(AppAction::None)
                }
                KeyCode::Down => {
                    if self.show_hot_list && !self.hotwords.is_empty() {
                        let len = self.hotwords.len();
                        let current = self.hot_selected_index();
                        let next = (current + 1) % len;
                        self.hot_selected = Some(next as u32);
                    }
                    Some(AppAction::None)
                }
                KeyCode::Enter => {
                    if !self.query.trim().is_empty() {
                        self.loading = true;
                        self.page = 1;
                        self.show_hot_list = false;
                        Some(AppAction::Search(self.query.clone()))
                    } else if self.show_hot_list {
                        self.search_selected_hotword()
                    } else {
                        Some(AppAction::None)
                    }
                }
                KeyCode::Esc => {
                    self.input_mode = false;
                    Some(AppAction::None)
                }
                _ if keys.matches_nav_next(key) => Some(AppAction::NavNext),
                _ if keys.matches_nav_prev(key) => Some(AppAction::NavPrev),
                _ => Some(AppAction::None),
            }
        } else if self.show_hot_list {
            if let Some(action) = self.kind_shortcut(key, keys) {
                return Some(action);
            }
            if keys.matches_up(key) {
                if !self.hotwords.is_empty() {
                    let len = self.hotwords.len();
                    let current = self.hot_selected_index();
                    let next = if current == 0 { len - 1 } else { current - 1 };
                    self.hot_selected = Some(next as u32);
                }
                return Some(AppAction::None);
            }
            if keys.matches_down(key) {
                if !self.hotwords.is_empty() {
                    let len = self.hotwords.len();
                    let current = self.hot_selected_index();
                    let next = (current + 1) % len;
                    self.hot_selected = Some(next as u32);
                }
                return Some(AppAction::None);
            }
            if keys.matches_confirm(key) {
                return self.search_selected_hotword();
            }
            if keys.matches_search_focus(key) {
                self.input_mode = true;
                self.show_hot_list = self.hotwords_enabled;
                return Some(AppAction::None);
            }
            if keys.matches_nav_next(key) {
                return Some(AppAction::NavNext);
            }
            if keys.matches_nav_prev(key) {
                return Some(AppAction::NavPrev);
            }
            if keys.matches_quit(key) {
                return Some(AppAction::Quit);
            }
            Some(AppAction::None)
        } else {
            if let Some(action) = self.kind_shortcut(key, keys) {
                return Some(action);
            }
            if keys.matches_page_down(key) {
                self.grid.move_page_down();
                if self.grid.is_near_bottom(self.grid.cached_visible_rows) && !self.loading_more {
                    return Some(AppAction::LoadMoreSearch);
                }
                return Some(AppAction::None);
            }
            if keys.matches_page_up(key) {
                self.grid.move_page_up();
                return Some(AppAction::None);
            }
            if keys.matches_down(key) {
                self.grid.move_down();
                // Check for pagination
                if self.grid.is_near_bottom(3) && !self.loading_more {
                    return Some(AppAction::LoadMoreSearch);
                }
                return Some(AppAction::None);
            }
            if keys.matches_up(key) {
                self.grid.move_up();
                return Some(AppAction::None);
            }
            if keys.matches_right(key) {
                self.grid.move_right();
                return Some(AppAction::None);
            }
            if keys.matches_left(key) {
                self.grid.move_left();
                return Some(AppAction::None);
            }
            if key == KeyCode::Char('u')
                && let Some(mid) = self.grid.selected_card().and_then(|card| card.uploader_mid)
            {
                return Some(AppAction::OpenUpPage(mid));
            }
            if keys.matches_confirm(key) {
                return Some(self.selected_target_action());
            }
            if keys.matches_search_focus(key) {
                self.input_mode = true;
                self.show_hot_list = self.hotwords_enabled;
                if self.hot_selected.is_none() && !self.hotwords.is_empty() {
                    self.hot_selected = Some(0);
                }
                return Some(AppAction::None);
            }
            if keys.matches_nav_next(key) {
                return Some(AppAction::NavNext);
            }
            if keys.matches_nav_prev(key) {
                return Some(AppAction::NavPrev);
            }
            if keys.matches_quit(key) {
                return Some(AppAction::Quit);
            }
            Some(AppAction::None)
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        // Don't handle mouse in input mode
        if self.input_mode {
            return None;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(2),
            ])
            .split(area);

        // Search kind switcher: click a tab to switch category.
        let kind_area = chunks[1];
        if event.kind == MouseEventKind::Down(MouseButton::Left)
            && kind_area.contains(ratatui::layout::Position::new(event.column, event.row))
        {
            let offset = event.column.saturating_sub(kind_area.x);
            if let Some((kind, _, _)) = Self::kind_bar_ranges()
                .iter()
                .find(|(_, start, end)| offset >= *start && offset < *end)
                && *kind != self.search_kind
            {
                return Some(AppAction::SwitchSearchKind(*kind));
            }
            return Some(AppAction::None);
        }

        // Handle hot list mouse interactions
        if self.show_hot_list {
            let list_area = chunks[2];

            if !list_area.contains(ratatui::layout::Position::new(event.column, event.row)) {
                return None;
            }

            return match event.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    // Convert click position to list index (account for top border)
                    let row_offset = event.row.saturating_sub(list_area.y + 1);
                    let idx = row_offset as usize;
                    if idx < self.hotwords.len() {
                        self.select_hotword(idx);
                        return self.search_selected_hotword();
                    }
                    None
                }
                MouseEventKind::ScrollDown => {
                    if !self.hotwords.is_empty() {
                        let len = self.hotwords.len();
                        let current = self.hot_selected_index();
                        let next = (current + 1) % len;
                        self.hot_selected = Some(next as u32);
                    }
                    Some(AppAction::None)
                }
                MouseEventKind::ScrollUp => {
                    if !self.hotwords.is_empty() {
                        let len = self.hotwords.len();
                        let current = self.hot_selected_index();
                        let next = if current == 0 { len - 1 } else { current - 1 };
                        self.hot_selected = Some(next as u32);
                    }
                    Some(AppAction::None)
                }
                _ => None,
            };
        }

        match event.kind {
            MouseEventKind::ScrollDown => {
                if self.grid.move_down() {
                    // Only check pagination if actually moved
                    if self.grid.is_near_bottom(3) && !self.loading_more {
                        return Some(AppAction::LoadMoreSearch);
                    }
                }
                None
            }
            MouseEventKind::ScrollUp => {
                self.grid.move_up();
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let header_height = 2u16;
                let grid_area = Rect {
                    y: chunks[2].y + header_height,
                    height: chunks[2].height.saturating_sub(header_height),
                    x: chunks[2].x,
                    width: chunks[2].width,
                };

                if !grid_area.contains(ratatui::layout::Position::new(event.column, event.row)) {
                    return None;
                }

                let relative_y = event.row - grid_area.y;
                let click_row = (relative_y / self.grid.card_height) as usize;
                let actual_row = self.grid.scroll_row + click_row;

                let card_width = grid_area.width / self.grid.columns as u16;
                let click_col = (event.column.saturating_sub(grid_area.x) / card_width) as usize;

                let click_idx = actual_row * self.grid.columns + click_col;

                if click_idx < self.grid.cards.len() {
                    let now = Instant::now();
                    let is_double_click = self.grid.selected_index == click_idx
                        && self
                            .last_click
                            .is_some_and(|t| now.duration_since(t).as_millis() < 500);

                    if is_double_click {
                        self.last_click = None;
                        return self.open_target_at(click_idx);
                    } else {
                        self.grid.selected_index = click_idx;
                        self.grid.update_scroll(self.grid.cached_visible_rows);
                        self.last_click = Some(now);
                    }
                }
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod minimal_mode_tests {
    use super::*;

    #[test]
    fn typing_and_refocusing_search_keeps_hotwords_disabled() {
        let mut page = SearchPage::new();
        page.disable_hotwords();
        let keys = Keybindings::default();
        for key in [
            KeyCode::Char('x'),
            KeyCode::Backspace,
            KeyCode::Esc,
            KeyCode::Char('/'),
        ] {
            page.handle_input(key, &keys);
            assert!(!page.show_hot_list);
        }
    }
}

#[cfg(test)]
mod search_kind_tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::KeyModifiers;

    fn keys() -> Keybindings {
        Keybindings::default()
    }

    fn video_item(bvid: &str, aid: i64) -> SearchVideoItem {
        serde_json::from_value(serde_json::json!({
            "aid": aid,
            "bvid": bvid,
            "title": "video <em class=\"keyword\">rust</em>",
            "author": "up",
            "pic": "//i0.hdslb.com/video.jpg",
            "play": 1000,
            "duration": "10:00",
            "description": "",
            "danmaku": 1,
            "mid": 9
        }))
        .expect("video fixture")
    }

    fn media_item(season_id: i64) -> MediaSearchItem {
        serde_json::from_value(serde_json::json!({
            "season_id": season_id,
            "title": "测试<em class=\"keyword\">番剧</em>",
            "cover": "//i0.hdslb.com/bangumi.jpg",
            "areas": "日本",
            "styles": "热血",
            "season_type_name": "番剧",
            "ep_size": 12,
            "media_score": {"score": 9.6, "user_count": 10},
            "is_follow": 0
        }))
        .expect("media fixture")
    }

    fn press(page: &mut SearchPage, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        let action = page.handle_input(key, keys);
        if let Some(AppAction::SwitchSearchKind(kind)) = action {
            page.set_search_kind(kind);
        }
        action
    }

    fn render(page: &mut SearchPage) -> String {
        let mut terminal = ratatui::Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal
            .draw(|frame| {
                page.draw(
                    frame,
                    frame.area(),
                    &Theme::default(),
                    &Keybindings::default(),
                )
            })
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect()
    }

    #[test]
    fn digits_type_in_input_mode_instead_of_switching_kind() {
        let mut page = SearchPage::new();
        let keys = keys();
        assert!(page.input_mode);
        for character in ['2', '3', '[', ']'] {
            page.handle_input(KeyCode::Char(character), &keys);
        }
        assert_eq!(page.query, "23[]");
        assert_eq!(page.search_kind, SearchKind::Video);
        assert!(matches!(
            page.handle_input(KeyCode::Char('1'), &keys),
            Some(AppAction::None)
        ));
        assert_eq!(page.query, "23[]1");
    }

    #[test]
    fn tab_and_section_keys_switch_kind_outside_input_mode() {
        let mut page = SearchPage::new();
        page.input_mode = false;
        let keys = keys();

        assert!(matches!(
            press(&mut page, KeyCode::Char('2'), &keys),
            Some(AppAction::SwitchSearchKind(SearchKind::MediaBangumi))
        ));
        assert!(matches!(
            press(&mut page, KeyCode::Char('3'), &keys),
            Some(AppAction::SwitchSearchKind(SearchKind::MediaFt))
        ));
        assert!(matches!(
            press(&mut page, KeyCode::Char('1'), &keys),
            Some(AppAction::SwitchSearchKind(SearchKind::Video))
        ));

        page.set_search_kind(SearchKind::MediaFt);
        assert!(matches!(
            press(&mut page, KeyCode::Char(']'), &keys),
            Some(AppAction::SwitchSearchKind(SearchKind::Video))
        ));
        assert!(matches!(
            press(&mut page, KeyCode::Char('['), &keys),
            Some(AppAction::SwitchSearchKind(SearchKind::MediaFt))
        ));
        // Already-active kind: not treated as a switch.
        page.set_search_kind(SearchKind::Video);
        assert!(matches!(
            press(&mut page, KeyCode::Char('1'), &keys),
            Some(AppAction::None)
        ));
    }

    #[test]
    fn clicking_the_kind_bar_switches_kind() {
        let mut page = SearchPage::new();
        page.show_hot_list = false;
        page.input_mode = false;
        render(&mut page);
        let (_, start, end) = SearchPage::kind_bar_ranges()
            .iter()
            .find(|(kind, _, _)| *kind == SearchKind::MediaBangumi)
            .copied()
            .expect("bangumi tab range");
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: (start + end) / 2,
            row: 3,
            modifiers: KeyModifiers::NONE,
        };
        assert!(matches!(
            page.handle_mouse(event, Rect::new(0, 0, 100, 30)),
            Some(AppAction::SwitchSearchKind(SearchKind::MediaBangumi))
        ));
    }

    #[test]
    fn bangumi_results_render_cards_and_open_detail() {
        let mut page = SearchPage::new();
        page.disable_hotwords();
        page.input_mode = false;
        page.set_media_results(SearchKind::MediaBangumi, vec![media_item(44219)], 1);
        assert_eq!(page.search_kind, SearchKind::MediaBangumi);
        assert_eq!(
            page.result_targets,
            vec![SearchResultTarget::Bangumi { season_id: 44219 }]
        );

        let text = render(&mut page);
        assert!(text.contains("测试番剧"), "rendered: {text}");
        assert!(!text.contains("<em"), "rendered: {text}");
        assert!(text.contains("[1]视频"), "rendered: {text}");
        assert!(text.contains("[2]番剧"), "rendered: {text}");
        assert!(text.contains("[3]影视"), "rendered: {text}");

        assert!(matches!(
            page.handle_input(KeyCode::Enter, &keys()),
            Some(AppAction::OpenBangumiDetail(44219))
        ));
        // Bangumi cards have no uploader, so `u` must not open an UP page.
        assert!(!matches!(
            page.handle_input(KeyCode::Char('u'), &keys()),
            Some(AppAction::OpenUpPage(_))
        ));
    }

    #[test]
    fn media_ft_results_use_ft_empty_message_and_targets() {
        let mut page = SearchPage::new();
        page.disable_hotwords();
        page.input_mode = false;
        page.query = "movie".to_string();
        page.set_media_results(SearchKind::MediaFt, vec![media_item(7)], 1);
        assert_eq!(
            page.result_targets,
            vec![SearchResultTarget::Bangumi { season_id: 7 }]
        );
        let mut empty = SearchPage::new();
        empty.disable_hotwords();
        empty.input_mode = false;
        empty.query = "nothing".to_string();
        empty.set_search_kind(SearchKind::MediaFt);
        let text = render(&mut empty);
        assert!(text.contains("没有找到相关影视"), "rendered: {text}");
    }

    #[test]
    fn video_results_still_open_video_detail() {
        let mut page = SearchPage::new();
        page.disable_hotwords();
        page.input_mode = false;
        page.set_results(vec![video_item("BV1test", 123)], 1);
        assert_eq!(page.grid.cards.len(), 1);
        assert_eq!(
            page.result_targets,
            vec![SearchResultTarget::Video {
                bvid: "BV1test".to_string(),
                aid: 123
            }]
        );
        assert_eq!(page.grid.cards[0].title, "video rust");
        assert!(matches!(
            page.handle_input(KeyCode::Enter, &keys()),
            Some(AppAction::OpenVideoDetail(bvid, 123)) if bvid == "BV1test"
        ));
    }
}
