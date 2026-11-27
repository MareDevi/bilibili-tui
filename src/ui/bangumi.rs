//! Bangumi page: ranked list, weekly timeline and season index tabs.
//!
//! Task 3B expands the filter bar and per-tab chrome; this module keeps the
//! three tabs functional and the state transitions wired to `AppAction`.

use super::video_card::{VideoCard, VideoCardGrid};
use super::{Component, Theme, shortcut_footer};
use crate::api::bangumi::{
    BangumiRankDay, BangumiSeasonType, SeasonIndexPage, SeasonIndexQuery, SeasonRankItem,
    TimelineDay, TimelineType,
};
use crate::application::{AppAction, BangumiTab};
use crate::storage::Keybindings;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind},
    prelude::*,
    widgets::*,
};
use std::time::{Duration, Instant};

pub struct BangumiPage {
    pub tab: BangumiTab,
    pub loading: bool,
    pub loading_more: bool,
    pub error_message: Option<String>,
    /// Rank tab state
    pub rank_grid: VideoCardGrid,
    pub rank_season_ids: Vec<i64>,
    pub rank_day: BangumiRankDay,
    pub rank_season_type: BangumiSeasonType,
    /// Timeline tab state
    pub timeline_days: Vec<TimelineDay>,
    pub timeline_selected: usize,
    pub timeline_type: TimelineType,
    pub timeline_before: u8,
    pub timeline_after: u8,
    /// Season index tab state
    pub index_grid: VideoCardGrid,
    pub index_season_ids: Vec<i64>,
    pub season_index_query: SeasonIndexQuery,
    pub index_loaded_page: i32,
    pub index_has_next: bool,
    // Double-click detection
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
}

impl BangumiPage {
    pub fn new() -> Self {
        Self {
            tab: BangumiTab::Rank,
            loading: false,
            loading_more: false,
            error_message: None,
            rank_grid: VideoCardGrid::new(),
            rank_season_ids: Vec::new(),
            rank_day: BangumiRankDay::default(),
            rank_season_type: BangumiSeasonType::default(),
            timeline_days: Vec::new(),
            timeline_selected: 0,
            timeline_type: TimelineType::default(),
            timeline_before: 3,
            timeline_after: 3,
            index_grid: VideoCardGrid::new(),
            index_season_ids: Vec::new(),
            season_index_query: SeasonIndexQuery::default(),
            index_loaded_page: 0,
            index_has_next: true,
            last_click_time: None,
            last_click_index: None,
        }
    }

    pub fn set_tab(&mut self, tab: BangumiTab) {
        if self.tab != tab {
            self.tab = tab;
            self.loading = false;
            self.loading_more = false;
            self.error_message = None;
            self.last_click_index = None;
        }
    }

    /// True when the active tab holds no data yet and needs a request.
    pub fn tab_needs_load(&self) -> bool {
        match self.tab {
            BangumiTab::Rank => self.rank_grid.cards.is_empty(),
            BangumiTab::Timeline => self.timeline_days.is_empty(),
            BangumiTab::Index => self.index_grid.cards.is_empty(),
        }
    }

    pub fn begin_load(&mut self) {
        self.loading = true;
        self.error_message = None;
    }

    pub fn set_rank_items(&mut self, items: Vec<SeasonRankItem>) {
        self.rank_grid.clear();
        self.rank_season_ids.clear();
        for item in items {
            self.rank_season_ids.push(item.season_id);
            let card = VideoCard::new(
                None,
                None,
                item.display_title(),
                item.display_subtitle(),
                item.score_text(),
                item.badge_text().unwrap_or_default().to_string(),
                Some(item.cover_url()),
            );
            self.rank_grid.add_card(card);
        }
        self.loading = false;
        self.loading_more = false;
        self.error_message = None;
    }

    pub fn set_timeline_days(&mut self, days: Vec<TimelineDay>) {
        self.timeline_days = days;
        if self.timeline_selected >= self.timeline_episode_count() {
            self.timeline_selected = 0;
        }
        self.loading = false;
        self.loading_more = false;
        self.error_message = None;
    }

    pub fn set_season_index_page(&mut self, page: SeasonIndexPage, append: bool) {
        if !append {
            self.index_grid.clear();
            self.index_season_ids.clear();
            self.index_loaded_page = 0;
        }
        self.index_loaded_page = if page.num > 0 {
            page.num
        } else {
            self.index_loaded_page + 1
        };
        self.index_has_next = page.has_next_page();
        for item in page.list {
            self.index_season_ids.push(item.season_id);
            let card = VideoCard::new(
                None,
                None,
                item.title.clone(),
                item.display_subtitle(),
                item.score_text(),
                item.badge_text().unwrap_or_default().to_string(),
                Some(item.cover_url()),
            );
            self.index_grid.add_card(card);
        }
        self.loading = false;
        self.loading_more = false;
        self.error_message = None;
    }

    pub fn set_error(&mut self, msg: String) {
        self.loading = false;
        self.loading_more = false;
        self.error_message = Some(msg);
    }

    /// The active tab may only accept an index response for the query it was
    /// asked for; the page number is validated separately.
    pub fn accepts_season_index_query(&self, query: &SeasonIndexQuery) -> bool {
        let mut expected = self.season_index_query.clone();
        expected.page = query.page;
        &expected == query
    }

    fn timeline_episode_count(&self) -> usize {
        self.timeline_days
            .iter()
            .map(|day| day.episodes.len())
            .sum()
    }

    fn timeline_selected_episode(&self) -> Option<(i64, i64)> {
        let mut counter = 0usize;
        for day in &self.timeline_days {
            for episode in &day.episodes {
                if counter == self.timeline_selected {
                    return Some((episode.season_id, episode.episode_id));
                }
                counter += 1;
            }
        }
        None
    }

    fn activate_selected(&self) -> Option<AppAction> {
        match self.tab {
            BangumiTab::Rank => self
                .rank_season_ids
                .get(self.rank_grid.selected_index)
                .copied()
                .filter(|season_id| *season_id > 0)
                .map(AppAction::OpenBangumiDetail),
            BangumiTab::Index => self
                .index_season_ids
                .get(self.index_grid.selected_index)
                .copied()
                .filter(|season_id| *season_id > 0)
                .map(AppAction::OpenBangumiDetail),
            BangumiTab::Timeline => {
                let (season_id, ep_id) = self.timeline_selected_episode()?;
                if season_id <= 0 || ep_id <= 0 {
                    return None;
                }
                Some(AppAction::OpenBangumiEpisode { season_id, ep_id })
            }
        }
    }

    fn active_grid_mut(&mut self) -> Option<&mut VideoCardGrid> {
        match self.tab {
            BangumiTab::Rank => Some(&mut self.rank_grid),
            BangumiTab::Index => Some(&mut self.index_grid),
            BangumiTab::Timeline => None,
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut spans = vec![Span::styled(
            " 🎬 番剧 ",
            Style::default()
                .fg(theme.fg_accent)
                .add_modifier(Modifier::BOLD),
        )];
        for (tab, label) in [
            (BangumiTab::Rank, "排行"),
            (BangumiTab::Timeline, "时间表"),
            (BangumiTab::Index, "索引"),
        ] {
            let style = if tab == self.tab {
                Style::default()
                    .fg(theme.bg_primary)
                    .bg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg_muted)
            };
            spans.push(Span::styled(format!(" {label} "), style));
            spans.push(Span::raw(" "));
        }
        let summary = match self.tab {
            BangumiTab::Rank => format!(
                "{} · {}",
                self.rank_day.label(),
                self.rank_season_type.label()
            ),
            BangumiTab::Timeline => format!(
                "{} · 前{}天 · 后{}天",
                self.timeline_type.label(),
                self.timeline_before,
                self.timeline_after
            ),
            BangumiTab::Index => format!(
                "{} · {} · {} · 第{}页",
                self.season_index_query.season_type.label(),
                self.season_index_query.order.label(),
                self.season_index_query.sort.label(),
                self.index_loaded_page.max(0)
            ),
        };
        let paragraph = Paragraph::new(vec![
            Line::from(spans),
            Line::from(vec![Span::styled(
                summary,
                Style::default().fg(theme.fg_muted),
            )]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_subtle)),
        );
        frame.render_widget(paragraph, area);
    }

    fn timeline_lines(&self, theme: &Theme) -> (Vec<Line<'static>>, Vec<Option<usize>>) {
        let mut lines = Vec::new();
        let mut index_map: Vec<Option<usize>> = Vec::new();
        let mut counter = 0usize;
        for day in &self.timeline_days {
            lines.push(Line::from(vec![Span::styled(
                format!(" {} {} ", day.date, day_of_week_label(day)),
                Style::default()
                    .fg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD),
            )]));
            index_map.push(None);
            if day.episodes.is_empty() {
                lines.push(Line::from(Span::styled(
                    "   （无更新）",
                    Style::default().fg(theme.fg_muted),
                )));
                index_map.push(None);
                continue;
            }
            for episode in &day.episodes {
                let selected = counter == self.timeline_selected;
                counter += 1;
                let delay = if episode.delay_reason.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", episode.delay_reason)
                };
                let label = format!("   {} {} {}", episode.pub_index, episode.title, delay);
                let style = if selected {
                    Style::default()
                        .fg(theme.selection_fg)
                        .bg(theme.selection_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg_primary)
                };
                lines.push(Line::from(Span::styled(label, style)));
                index_map.push(Some(counter - 1));
            }
        }
        (lines, index_map)
    }

    fn render_timeline(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let (lines, index_map) = self.timeline_lines(theme);
        let selected_line = index_map
            .iter()
            .position(|entry| *entry == Some(self.timeline_selected))
            .unwrap_or(0);
        let inner_height = area.height.saturating_sub(2) as usize;
        let offset = if inner_height > 0 && selected_line >= inner_height {
            (selected_line + 1 - inner_height) as u16
        } else {
            0
        };
        let paragraph = Paragraph::new(lines).scroll((offset, 0)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_subtle)),
        );
        frame.render_widget(paragraph, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        let tabs = format!("{}/{}/{}", keys.tab_1, keys.tab_2, keys.tab_3);
        let items = vec![
            ("←→↑↓".to_string(), "浏览".to_string(), theme.fg_secondary),
            (keys.confirm.clone(), "打开".to_string(), theme.success),
            (keys.refresh.clone(), "刷新".to_string(), theme.info),
            (tabs, "切换".to_string(), theme.fg_secondary),
        ];
        let footer = shortcut_footer(theme, items);
        frame.render_widget(Paragraph::new(footer), area);
    }
}

fn day_of_week_label(day: &TimelineDay) -> &'static str {
    if day.is_today != 0 {
        return "今天";
    }
    match day.day_of_week {
        1 => "周一",
        2 => "周二",
        3 => "周三",
        4 => "周四",
        5 => "周五",
        6 => "周六",
        7 => "周日",
        _ => "",
    }
}

impl Default for BangumiPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for BangumiPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(area);

        self.render_header(frame, chunks[0], theme);

        let content = chunks[1];
        let has_data = match self.tab {
            BangumiTab::Rank => !self.rank_grid.cards.is_empty(),
            BangumiTab::Timeline => !self.timeline_days.is_empty(),
            BangumiTab::Index => !self.index_grid.cards.is_empty(),
        };
        if self.loading && !has_data {
            let paragraph = Paragraph::new("⏳ 加载中...")
                .style(Style::default().fg(theme.fg_muted))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.border_subtle)),
                );
            frame.render_widget(paragraph, content);
        } else if let Some(error) = self.error_message.as_deref()
            && !has_data
        {
            let paragraph = Paragraph::new(format!("❌ {error}"))
                .style(Style::default().fg(theme.error))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.border_subtle)),
                );
            frame.render_widget(paragraph, content);
        } else if !has_data {
            let paragraph = Paragraph::new("暂无数据，按 R 刷新")
                .style(Style::default().fg(theme.fg_muted))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.border_subtle)),
                );
            frame.render_widget(paragraph, content);
        } else {
            match self.tab {
                BangumiTab::Rank => self.rank_grid.render(frame, content, theme),
                BangumiTab::Index => self.index_grid.render(frame, content, theme),
                BangumiTab::Timeline => self.render_timeline(frame, content, theme),
            }
        }

        self.render_footer(frame, chunks[2], theme, keys);
    }

    fn handle_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        if keys.matches_refresh(key) {
            return Some(AppAction::RefreshBangumi);
        }
        if keys.matches_tab_1(key) {
            return Some(AppAction::SwitchBangumiTab(BangumiTab::Rank));
        }
        if keys.matches_tab_2(key) {
            return Some(AppAction::SwitchBangumiTab(BangumiTab::Timeline));
        }
        if keys.matches_tab_3(key) {
            return Some(AppAction::SwitchBangumiTab(BangumiTab::Index));
        }
        if keys.matches_confirm(key) || keys.matches_play(key) {
            return self.activate_selected();
        }
        match self.tab {
            BangumiTab::Rank | BangumiTab::Index => {
                let is_index = self.tab == BangumiTab::Index;
                let mut load_more = false;
                if let Some(grid) = self.active_grid_mut() {
                    if keys.matches_down(key) {
                        grid.move_down();
                    } else if keys.matches_up(key) {
                        grid.move_up();
                    } else if keys.matches_left(key) {
                        grid.move_left();
                    } else if keys.matches_right(key) {
                        grid.move_right();
                    } else if keys.matches_page_down(key) {
                        grid.move_page_down();
                        load_more = is_index && grid.is_near_bottom(grid.cached_visible_rows);
                    } else if keys.matches_page_up(key) {
                        grid.move_page_up();
                    }
                }
                if load_more && self.index_has_next && !self.loading_more {
                    return Some(AppAction::LoadMoreBangumi);
                }
            }
            BangumiTab::Timeline => {
                let count = self.timeline_episode_count();
                if count == 0 {
                    return None;
                }
                if keys.matches_down(key) {
                    self.timeline_selected = (self.timeline_selected + 1).min(count - 1);
                } else if keys.matches_up(key) {
                    self.timeline_selected = self.timeline_selected.saturating_sub(1);
                } else if keys.matches_page_down(key) {
                    self.timeline_selected = (self.timeline_selected + 5).min(count - 1);
                } else if keys.matches_page_up(key) {
                    self.timeline_selected = self.timeline_selected.saturating_sub(5);
                }
            }
        }
        None
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        if area.height < 6 || area.width < 10 {
            return None;
        }
        if self.tab == BangumiTab::Timeline {
            return None;
        }
        let content = Rect::new(
            area.x + 1,
            area.y + 3,
            area.width.saturating_sub(2),
            area.height.saturating_sub(5),
        );
        if !content.contains(Position::new(event.column, event.row)) {
            return None;
        }
        if !matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) {
            return None;
        }
        let grid = self.active_grid_mut()?;
        grid.select_at(event.row, content);
        let index = grid.selected_index;
        let now = Instant::now();
        let is_double = self
            .last_click_time
            .map(|last| now.duration_since(last) < Duration::from_millis(500))
            .unwrap_or(false)
            && self.last_click_index == Some(index);
        self.last_click_time = Some(now);
        self.last_click_index = Some(index);
        if is_double {
            self.activate_selected()
        } else {
            None
        }
    }
}
