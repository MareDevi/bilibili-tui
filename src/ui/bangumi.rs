//! Bangumi page: ranked list, weekly timeline and season index tabs.
//!
//! One page hosts three views. `Rank` filters by period and category,
//! `Timeline` groups episodes by day and marks published/delayed entries, and
//! `Index` exposes a seven-chip filter bar with deduplicated infinite
//! pagination.
//!
//! Loading, loaded and error state are tracked per tab: a legitimate empty
//! response (for example `综艺` on the rank endpoint) counts as loaded, and a
//! failure on one tab never clears another tab's content. `App` keeps the
//! whole page when a detail page opens, so returning restores the tab,
//! filters, loaded rows and cursor.

use super::video_card::{VideoCard, VideoCardGrid};
use super::{Component, Theme, shortcut_footer};
use crate::api::bangumi::{
    BangumiRankDay, BangumiSeasonType, SeasonIndexOrder, SeasonIndexPage, SeasonIndexQuery,
    SeasonIndexSort, SeasonRankItem, TimelineDay, TimelineType,
};
use crate::application::{AppAction, BangumiTab};
use crate::storage::Keybindings;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind},
    prelude::*,
    widgets::*,
};
use std::collections::HashSet;
use std::time::{Duration, Instant};

const PAGE_TITLE: &str = " 番剧  ";
const SOURCE_TITLE: &str = " 番剧 ";
const SOURCE_COUNT: usize = 3;
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(500);
const TIMELINE_PAGE_STEP: usize = 5;

/// Which part of the page owns keyboard input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BangumiFocus {
    Sources,
    Filter,
    List,
}

/// Rank filter chips, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankFilterField {
    Day,
    SeasonType,
}

impl RankFilterField {
    const ALL: [Self; 2] = [Self::Day, Self::SeasonType];

    fn label(self) -> &'static str {
        match self {
            Self::Day => "周期",
            Self::SeasonType => "类别",
        }
    }
}

/// Timeline filter chips, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineFilterField {
    Type,
    Before,
    After,
}

impl TimelineFilterField {
    const ALL: [Self; 3] = [Self::Type, Self::Before, Self::After];

    fn label(self) -> &'static str {
        match self {
            Self::Type => "类别",
            Self::Before => "前",
            Self::After => "后",
        }
    }
}

/// Season index filter chips, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFilterField {
    Category,
    Order,
    Sort,
    Year,
    Status,
    Payment,
    Area,
}

impl IndexFilterField {
    const ALL: [Self; 7] = [
        Self::Category,
        Self::Order,
        Self::Sort,
        Self::Year,
        Self::Status,
        Self::Payment,
        Self::Area,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Category => "类别",
            Self::Order => "排序",
            Self::Sort => "方向",
            Self::Year => "年份",
            Self::Status => "状态",
            Self::Payment => "付费",
            Self::Area => "地区",
        }
    }
}

struct YearOption {
    id: &'static str,
    label: &'static str,
}

/// Year ranges accepted by `/pgc/season/index/result` (live-verified ids).
const INDEX_YEARS: [YearOption; 12] = [
    YearOption {
        id: "-1",
        label: "全部",
    },
    YearOption {
        id: "[2026,2027)",
        label: "2026",
    },
    YearOption {
        id: "[2025,2026)",
        label: "2025",
    },
    YearOption {
        id: "[2024,2025)",
        label: "2024",
    },
    YearOption {
        id: "[2023,2024)",
        label: "2023",
    },
    YearOption {
        id: "[2022,2023)",
        label: "2022",
    },
    YearOption {
        id: "[2021,2022)",
        label: "2021",
    },
    YearOption {
        id: "[2020,2021)",
        label: "2020",
    },
    YearOption {
        id: "[2015,2020)",
        label: "2015-2019",
    },
    YearOption {
        id: "[2010,2015)",
        label: "2010-2014",
    },
    YearOption {
        id: "[1990,2000)",
        label: "90年代",
    },
    YearOption {
        id: "[,1980)",
        label: "更早",
    },
];

/// Order options in the display order users expect.
const INDEX_ORDER_VALUES: [SeasonIndexOrder; 7] = [
    SeasonIndexOrder::Follow,
    SeasonIndexOrder::UpdateTime,
    SeasonIndexOrder::Score,
    SeasonIndexOrder::Play,
    SeasonIndexOrder::PubDate,
    SeasonIndexOrder::ReleaseDate,
    SeasonIndexOrder::Danmaku,
];
/// Completion filter (`is_finish`).
const INDEX_STATUS_VALUES: [(i32, &str); 3] = [(-1, "全部"), (1, "完结"), (0, "连载")];
/// Payment filter (`season_status`).
const INDEX_PAYMENT_VALUES: [(&str, &str); 4] = [
    ("-1", "全部"),
    ("1", "免费"),
    ("2,6", "付费"),
    ("4,6", "大会员"),
];
/// Area filter (`area`).
const INDEX_AREA_VALUES: [(&str, &str); 5] = [
    ("-1", "全部"),
    ("2", "日本"),
    ("1", "中国大陆"),
    ("3", "美国"),
    ("16", "其他"),
];

fn cycle_value<T: Copy + PartialEq>(values: &[T], current: T, forward: bool) -> T {
    let len = values.len();
    if len == 0 {
        return current;
    }
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    let next = if forward {
        (index + 1) % len
    } else {
        (index + len - 1) % len
    };
    values[next]
}

fn cycle_inclusive_u8(current: u8, max: u8, forward: bool) -> u8 {
    let count = max as usize + 1;
    let index = current.min(max) as usize;
    let next = if forward {
        (index + 1) % count
    } else {
        (index + count - 1) % count
    };
    next as u8
}

fn year_label(year: &str) -> &'static str {
    INDEX_YEARS
        .iter()
        .find(|option| option.id == year)
        .map(|option| option.label)
        .unwrap_or("全部")
}

fn status_label(value: i32) -> &'static str {
    INDEX_STATUS_VALUES
        .iter()
        .find(|(status, _)| *status == value)
        .map(|(_, label)| *label)
        .unwrap_or("全部")
}

fn payment_label(value: &str) -> &'static str {
    INDEX_PAYMENT_VALUES
        .iter()
        .find(|(id, _)| *id == value)
        .map(|(_, label)| *label)
        .unwrap_or("全部")
}

fn area_label(value: &str) -> &'static str {
    INDEX_AREA_VALUES
        .iter()
        .find(|(id, _)| *id == value)
        .map(|(_, label)| *label)
        .unwrap_or("全部")
}

fn weekday_label(day_of_week: i32) -> &'static str {
    match day_of_week {
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

fn tab_index(tab: BangumiTab) -> usize {
    match tab {
        BangumiTab::Rank => 0,
        BangumiTab::Timeline => 1,
        BangumiTab::Index => 2,
    }
}

fn source_tab(index: usize) -> BangumiTab {
    match index {
        1 => BangumiTab::Timeline,
        2 => BangumiTab::Index,
        _ => BangumiTab::Rank,
    }
}

fn tab_label(tab: BangumiTab) -> &'static str {
    match tab {
        BangumiTab::Rank => "番剧排行",
        BangumiTab::Timeline => "新番时间线",
        BangumiTab::Index => "番剧索引",
    }
}

fn source_label(index: usize) -> &'static str {
    tab_label(source_tab(index))
}

/// Horizontal split shared by drawing and mouse hit-testing.
struct PageLayout {
    sources: Rect,
    main: Rect,
    header: Rect,
    filters: Rect,
    content: Rect,
    footer: Rect,
}

impl PageLayout {
    fn new(area: Rect) -> Self {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(30)])
            .split(area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(2),
            ])
            .split(panes[1]);
        Self {
            sources: panes[0],
            main: panes[1],
            header: chunks[0],
            filters: chunks[1],
            content: chunks[2],
            footer: chunks[3],
        }
    }
}

pub struct BangumiPage {
    pub tab: BangumiTab,
    pub focus: BangumiFocus,
    /// Left-pane view cursor, kept in sync with `tab`.
    pub selected_source: usize,
    pub filter_cursor: usize,
    /// Rank tab state.
    pub rank_grid: VideoCardGrid,
    pub rank_season_ids: Vec<i64>,
    pub rank_day: BangumiRankDay,
    pub rank_season_type: BangumiSeasonType,
    pub rank_loaded: bool,
    pub rank_loading: bool,
    pub rank_error: Option<String>,
    /// Timeline tab state.
    pub timeline_days: Vec<TimelineDay>,
    pub timeline_selected: usize,
    pub timeline_type: TimelineType,
    pub timeline_before: u8,
    pub timeline_after: u8,
    pub timeline_loaded: bool,
    pub timeline_loading: bool,
    pub timeline_error: Option<String>,
    /// Season index tab state.
    pub index_grid: VideoCardGrid,
    pub index_season_ids: Vec<i64>,
    /// Season ids already merged into the grid, used to dedupe appended pages.
    pub index_seen_ids: HashSet<i64>,
    pub season_index_query: SeasonIndexQuery,
    pub index_loaded_page: i32,
    pub index_has_next: bool,
    pub index_total: i64,
    pub index_loaded: bool,
    pub index_loading: bool,
    pub index_loading_more: bool,
    pub index_error: Option<String>,
    /// Non-destructive notice, e.g. a failed load-more request.
    pub index_notice: Option<String>,
    /// Filter chip hit columns, recorded while drawing.
    filter_hit_ranges: Vec<(usize, u16, u16)>,
    /// Timeline line -> flat episode index mapping from the last draw.
    timeline_line_map: Vec<Option<usize>>,
    timeline_scroll: u16,
    // Double-click detection
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
    last_click_tab: Option<BangumiTab>,
}

impl BangumiPage {
    pub fn new() -> Self {
        Self {
            tab: BangumiTab::Rank,
            focus: BangumiFocus::Sources,
            selected_source: 0,
            filter_cursor: 0,
            rank_grid: VideoCardGrid::new_list(),
            rank_season_ids: Vec::new(),
            rank_day: BangumiRankDay::default(),
            rank_season_type: BangumiSeasonType::default(),
            rank_loaded: false,
            rank_loading: false,
            rank_error: None,
            timeline_days: Vec::new(),
            timeline_selected: 0,
            timeline_type: TimelineType::default(),
            timeline_before: 3,
            timeline_after: 3,
            timeline_loaded: false,
            timeline_loading: false,
            timeline_error: None,
            index_grid: VideoCardGrid::new_list(),
            index_season_ids: Vec::new(),
            index_seen_ids: HashSet::new(),
            season_index_query: SeasonIndexQuery::default(),
            index_loaded_page: 0,
            index_has_next: true,
            index_total: 0,
            index_loaded: false,
            index_loading: false,
            index_loading_more: false,
            index_error: None,
            index_notice: None,
            filter_hit_ranges: Vec::new(),
            timeline_line_map: Vec::new(),
            timeline_scroll: 0,
            last_click_time: None,
            last_click_index: None,
            last_click_tab: None,
        }
    }

    pub fn set_tab(&mut self, tab: BangumiTab) {
        self.selected_source = tab_index(tab);
        if self.tab != tab {
            self.tab = tab;
            self.focus = BangumiFocus::List;
            self.filter_cursor = 0;
            self.last_click_index = None;
            self.last_click_tab = None;
            self.last_click_time = None;
        }
    }

    pub fn is_loaded(&self, tab: BangumiTab) -> bool {
        match tab {
            BangumiTab::Rank => self.rank_loaded,
            BangumiTab::Timeline => self.timeline_loaded,
            BangumiTab::Index => self.index_loaded,
        }
    }

    /// True when the active tab has never received a successful response.
    pub fn tab_needs_load(&self) -> bool {
        !self.is_loaded(self.tab)
    }

    /// Start a request for a tab whose data is about to be replaced (filter
    /// change or first load). Clears the tab's rows so stale filters never
    /// linger under the spinner.
    pub fn begin_load(&mut self, tab: BangumiTab) {
        match tab {
            BangumiTab::Rank => {
                self.rank_loading = true;
                self.rank_error = None;
                self.rank_loaded = false;
                self.rank_grid.clear();
                self.rank_season_ids.clear();
            }
            BangumiTab::Timeline => {
                self.timeline_loading = true;
                self.timeline_error = None;
                self.timeline_loaded = false;
                self.timeline_days.clear();
                self.timeline_selected = 0;
                self.timeline_line_map.clear();
                self.timeline_scroll = 0;
            }
            BangumiTab::Index => {
                self.index_loading = true;
                self.index_loading_more = false;
                self.index_error = None;
                self.index_notice = None;
                self.index_loaded = false;
                self.index_has_next = true;
                self.index_loaded_page = 0;
                self.index_total = 0;
                self.index_grid.clear();
                self.index_season_ids.clear();
                self.index_seen_ids.clear();
                self.season_index_query.page = 1;
            }
        }
    }

    /// Refresh the active tab in place, keeping filters and (for rank and
    /// timeline) the rows already on screen. Index restarts at page 1.
    pub fn force_reload(&mut self, tab: BangumiTab) {
        match tab {
            BangumiTab::Rank => {
                self.rank_loading = true;
                self.rank_error = None;
            }
            BangumiTab::Timeline => {
                self.timeline_loading = true;
                self.timeline_error = None;
            }
            BangumiTab::Index => {
                self.index_loading = true;
                self.index_loading_more = false;
                self.index_error = None;
                self.index_notice = None;
                self.index_loaded_page = 0;
                self.index_has_next = true;
                self.index_total = 0;
                self.index_grid.clear();
                self.index_season_ids.clear();
                self.index_seen_ids.clear();
                self.season_index_query.page = 1;
            }
        }
    }

    pub fn begin_index_load_more(&mut self) {
        self.index_loading_more = true;
        self.index_notice = None;
    }

    /// Apply a rank response. Returns `false` when it belongs to another
    /// filter combination and must be ignored.
    pub fn apply_rank(
        &mut self,
        day: BangumiRankDay,
        season_type: BangumiSeasonType,
        items: Vec<SeasonRankItem>,
    ) -> bool {
        if self.rank_day != day || self.rank_season_type != season_type {
            return false;
        }
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
        self.rank_loaded = true;
        self.rank_loading = false;
        self.rank_error = None;
        true
    }

    /// Apply a timeline response, selecting today's first episode by default.
    pub fn apply_timeline(
        &mut self,
        types: TimelineType,
        before: u8,
        after: u8,
        days: Vec<TimelineDay>,
    ) -> bool {
        if self.timeline_type != types
            || self.timeline_before != before
            || self.timeline_after != after
        {
            return false;
        }
        self.timeline_days = days;
        self.timeline_selected = self.default_timeline_selection();
        self.timeline_loading = false;
        self.timeline_error = None;
        self.timeline_loaded = true;
        true
    }

    /// Apply an index page. Appends deduplicate by `season_id`.
    pub fn apply_season_index_page(
        &mut self,
        query: &SeasonIndexQuery,
        append: bool,
        page: SeasonIndexPage,
    ) -> bool {
        if !self.accepts_season_index_query(query) {
            return false;
        }
        if !append {
            self.index_grid.clear();
            self.index_season_ids.clear();
            self.index_seen_ids.clear();
        }
        self.season_index_query.page = query.page;
        self.index_loaded_page = query.page;
        self.index_total = page.total;
        self.index_has_next = page.has_next_page();
        for item in page.list {
            if !self.index_seen_ids.insert(item.season_id) {
                continue;
            }
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
        self.index_loading = false;
        self.index_loading_more = false;
        self.index_error = None;
        self.index_notice = None;
        self.index_loaded = true;
        true
    }

    /// Record a failure for one tab. Loaded rows stay visible; an in-flight
    /// load-more failure becomes a notice instead of an error panel.
    pub fn fail(&mut self, tab: BangumiTab, message: String) {
        match tab {
            BangumiTab::Rank => {
                self.rank_loading = false;
                self.rank_error = Some(message);
            }
            BangumiTab::Timeline => {
                self.timeline_loading = false;
                self.timeline_error = Some(message);
            }
            BangumiTab::Index => {
                if self.index_loading_more {
                    self.index_loading_more = false;
                    self.index_error = None;
                    self.index_notice = Some(message);
                } else {
                    self.index_loading = false;
                    self.index_error = Some(message);
                }
            }
        }
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

    fn default_timeline_selection(&self) -> usize {
        let mut counter = 0usize;
        for day in &self.timeline_days {
            if day.is_today != 0 && !day.episodes.is_empty() {
                return counter;
            }
            counter += day.episodes.len();
        }
        0
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

    fn active_grid(&self) -> Option<&VideoCardGrid> {
        match self.tab {
            BangumiTab::Rank => Some(&self.rank_grid),
            BangumiTab::Index => Some(&self.index_grid),
            BangumiTab::Timeline => None,
        }
    }

    fn active_grid_mut(&mut self) -> Option<&mut VideoCardGrid> {
        match self.tab {
            BangumiTab::Rank => Some(&mut self.rank_grid),
            BangumiTab::Index => Some(&mut self.index_grid),
            BangumiTab::Timeline => None,
        }
    }

    fn filter_count(&self) -> usize {
        match self.tab {
            BangumiTab::Rank => RankFilterField::ALL.len(),
            BangumiTab::Timeline => TimelineFilterField::ALL.len(),
            BangumiTab::Index => IndexFilterField::ALL.len(),
        }
    }

    fn toggle_filter_focus(&mut self) {
        match self.focus {
            BangumiFocus::Filter => self.focus = BangumiFocus::List,
            _ => {
                self.focus = BangumiFocus::Filter;
                self.filter_cursor = self
                    .filter_cursor
                    .min(self.filter_count().saturating_sub(1));
            }
        }
    }

    fn cycle_rank_filter(&mut self, forward: bool) -> AppAction {
        let field = RankFilterField::ALL[self.filter_cursor.min(RankFilterField::ALL.len() - 1)];
        match field {
            RankFilterField::Day => {
                self.rank_day = cycle_value(&BangumiRankDay::ALL, self.rank_day, forward)
            }
            RankFilterField::SeasonType => {
                self.rank_season_type =
                    cycle_value(&BangumiSeasonType::ALL, self.rank_season_type, forward)
            }
        }
        AppAction::LoadBangumiRank {
            day: self.rank_day,
            season_type: self.rank_season_type,
        }
    }

    fn cycle_timeline_filter(&mut self, forward: bool) -> AppAction {
        let field =
            TimelineFilterField::ALL[self.filter_cursor.min(TimelineFilterField::ALL.len() - 1)];
        match field {
            TimelineFilterField::Type => {
                self.timeline_type = cycle_value(&TimelineType::ALL, self.timeline_type, forward)
            }
            TimelineFilterField::Before => {
                self.timeline_before = cycle_inclusive_u8(self.timeline_before, 7, forward)
            }
            TimelineFilterField::After => {
                self.timeline_after = cycle_inclusive_u8(self.timeline_after, 7, forward)
            }
        }
        AppAction::LoadBangumiTimeline {
            types: self.timeline_type,
            before: self.timeline_before,
            after: self.timeline_after,
        }
    }

    fn cycle_index_filter(&mut self, forward: bool) -> AppAction {
        let field = IndexFilterField::ALL[self.filter_cursor.min(IndexFilterField::ALL.len() - 1)];
        let query = &mut self.season_index_query;
        match field {
            IndexFilterField::Category => {
                query.season_type = cycle_value(&BangumiSeasonType::ALL, query.season_type, forward)
            }
            IndexFilterField::Order => {
                query.order = cycle_value(&INDEX_ORDER_VALUES, query.order, forward)
            }
            IndexFilterField::Sort => {
                query.sort = cycle_value(
                    &[SeasonIndexSort::Desc, SeasonIndexSort::Asc],
                    query.sort,
                    forward,
                )
            }
            IndexFilterField::Year => {
                let index = INDEX_YEARS
                    .iter()
                    .position(|option| option.id == query.year)
                    .unwrap_or(0);
                let len = INDEX_YEARS.len();
                let next = if forward {
                    (index + 1) % len
                } else {
                    (index + len - 1) % len
                };
                query.year = INDEX_YEARS[next].id.to_string();
            }
            IndexFilterField::Status => {
                let index = INDEX_STATUS_VALUES
                    .iter()
                    .position(|(value, _)| *value == query.is_finish)
                    .unwrap_or(0);
                let len = INDEX_STATUS_VALUES.len();
                let next = if forward {
                    (index + 1) % len
                } else {
                    (index + len - 1) % len
                };
                query.is_finish = INDEX_STATUS_VALUES[next].0;
            }
            IndexFilterField::Payment => {
                let index = INDEX_PAYMENT_VALUES
                    .iter()
                    .position(|(value, _)| *value == query.season_status)
                    .unwrap_or(0);
                let len = INDEX_PAYMENT_VALUES.len();
                let next = if forward {
                    (index + 1) % len
                } else {
                    (index + len - 1) % len
                };
                query.season_status = INDEX_PAYMENT_VALUES[next].0.to_string();
            }
            IndexFilterField::Area => {
                let index = INDEX_AREA_VALUES
                    .iter()
                    .position(|(value, _)| *value == query.area)
                    .unwrap_or(0);
                let len = INDEX_AREA_VALUES.len();
                let next = if forward {
                    (index + 1) % len
                } else {
                    (index + len - 1) % len
                };
                query.area = INDEX_AREA_VALUES[next].0.to_string();
            }
        }
        query.page = 1;
        AppAction::LoadSeasonIndex {
            query: query.clone(),
            append: false,
        }
    }

    fn index_filter_value(&self, field: IndexFilterField) -> String {
        let query = &self.season_index_query;
        match field {
            IndexFilterField::Category => query.season_type.label().to_string(),
            IndexFilterField::Order => query.order.label().to_string(),
            IndexFilterField::Sort => match query.sort {
                SeasonIndexSort::Desc => "降序",
                SeasonIndexSort::Asc => "增序",
            }
            .to_string(),
            IndexFilterField::Year => year_label(&query.year).to_string(),
            IndexFilterField::Status => status_label(query.is_finish).to_string(),
            IndexFilterField::Payment => payment_label(&query.season_status).to_string(),
            IndexFilterField::Area => area_label(&query.area).to_string(),
        }
    }

    fn filter_entries(&self) -> Vec<(&'static str, String)> {
        match self.tab {
            BangumiTab::Rank => vec![
                (
                    RankFilterField::Day.label(),
                    self.rank_day.label().to_string(),
                ),
                (
                    RankFilterField::SeasonType.label(),
                    self.rank_season_type.label().to_string(),
                ),
            ],
            BangumiTab::Timeline => vec![
                (
                    TimelineFilterField::Type.label(),
                    self.timeline_type.label().to_string(),
                ),
                (
                    TimelineFilterField::Before.label(),
                    format!("{} 天", self.timeline_before),
                ),
                (
                    TimelineFilterField::After.label(),
                    format!("{} 天", self.timeline_after),
                ),
            ],
            BangumiTab::Index => IndexFilterField::ALL
                .iter()
                .map(|field| (field.label(), self.index_filter_value(*field)))
                .collect(),
        }
    }

    fn can_load_more(&self) -> bool {
        self.tab == BangumiTab::Index
            && self.index_has_next
            && !self.index_loading
            && !self.index_loading_more
            && self
                .index_grid
                .is_near_bottom(self.index_grid.cached_visible_rows)
    }

    /// True when the page has a non-empty filter bar in focus.
    fn meta(&self, theme: &Theme) -> (String, Style) {
        let mut style = Style::default().fg(theme.fg_muted);
        let text = match self.tab {
            BangumiTab::Rank => {
                let mut text = format!(
                    "{} · {}",
                    self.rank_day.label(),
                    self.rank_season_type.label()
                );
                if self.rank_loading && self.rank_loaded {
                    text.push_str(" · ⟳ 刷新中...");
                }
                if self.rank_loaded
                    && let Some(error) = &self.rank_error
                {
                    text.push_str(&format!(" · ❌ {error}"));
                    style = Style::default().fg(theme.error);
                }
                text
            }
            BangumiTab::Timeline => {
                let mut text = format!(
                    "{} · 前{}天 · 后{}天",
                    self.timeline_type.label(),
                    self.timeline_before,
                    self.timeline_after
                );
                if self.timeline_loading && self.timeline_loaded {
                    text.push_str(" · ⟳ 刷新中...");
                }
                if self.timeline_loaded
                    && let Some(error) = &self.timeline_error
                {
                    text.push_str(&format!(" · ❌ {error}"));
                    style = Style::default().fg(theme.error);
                }
                text
            }
            BangumiTab::Index => {
                let loaded = self.index_season_ids.len();
                let mut text = format!(
                    "{} · {} · 已加载 {}",
                    self.season_index_query.season_type.label(),
                    self.season_index_query.order.label(),
                    loaded
                );
                if self.index_total > 0 {
                    text.push_str(&format!("/{}", self.index_total));
                }
                if self.index_loading_more {
                    text.push_str(" · 加载更多...");
                }
                if let Some(notice) = &self.index_notice {
                    text.push_str(&format!(" · ⚠ {notice}"));
                    style = Style::default().fg(theme.warning);
                }
                if self.index_loaded
                    && let Some(error) = &self.index_error
                {
                    text.push_str(&format!(" · ❌ {error}"));
                    style = Style::default().fg(theme.error);
                }
                text
            }
        };
        (text, style)
    }

    fn draw_sources(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items = (0..SOURCE_COUNT)
            .map(|index| {
                ListItem::new(source_label(index)).style(if index == self.selected_source {
                    Style::default()
                        .fg(if self.focus == BangumiFocus::Sources {
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
                    .title(SOURCE_TITLE),
            )
            .highlight_symbol("▶ ");
        let mut state = ListState::default().with_selected(Some(self.selected_source));
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_header(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let (meta, meta_style) = self.meta(theme);
        let header = Paragraph::new(Line::from(vec![
            Span::styled(PAGE_TITLE, Style::default().fg(theme.fg_accent)),
            Span::styled(
                tab_label(self.tab),
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ·  ", Style::default().fg(theme.fg_muted)),
            Span::styled(meta, meta_style),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
        frame.render_widget(header, area);
    }

    fn render_filter_bar(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let filters_focused = self.focus == BangumiFocus::Filter;
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut ranges = Vec::new();
        let mut cursor: u16 = 0;
        for (index, (label, value)) in self.filter_entries().into_iter().enumerate() {
            let style = if filters_focused && index == self.filter_cursor {
                Style::default()
                    .fg(theme.selection_fg)
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else if filters_focused {
                Style::default().fg(theme.fg_primary)
            } else {
                Style::default().fg(theme.fg_muted)
            };
            let span = Span::styled(format!(" {label}: {value} "), style);
            let start = cursor;
            cursor += span.width() as u16;
            spans.push(span);
            ranges.push((index, start, cursor));
            spans.push(Span::raw(" "));
            cursor += 1;
        }
        self.filter_hit_ranges = ranges;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if filters_focused {
                theme.border_focused
            } else {
                theme.border_subtle
            }));
        frame.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
    }

    fn render_placeholder(&self, frame: &mut Frame, area: Rect, theme: &Theme, text: &str) {
        frame.render_widget(
            Paragraph::new(text.to_string())
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center),
            area,
        );
    }

    fn render_error_placeholder(&self, frame: &mut Frame, area: Rect, theme: &Theme, text: &str) {
        frame.render_widget(
            Paragraph::new(text.to_string())
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center),
            area,
        );
    }

    fn render_loading_placeholder(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        frame.render_widget(
            Paragraph::new("⏳ 加载中…")
                .style(Style::default().fg(theme.warning))
                .alignment(Alignment::Center),
            area,
        );
    }

    fn render_content(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let (loaded, loading, error, has_data, empty_text) = match self.tab {
            BangumiTab::Rank => (
                self.rank_loaded,
                self.rank_loading,
                self.rank_error.as_deref(),
                !self.rank_grid.cards.is_empty(),
                "暂无排行数据",
            ),
            BangumiTab::Timeline => (
                self.timeline_loaded,
                self.timeline_loading,
                self.timeline_error.as_deref(),
                !self.timeline_days.is_empty(),
                "暂无时间线数据",
            ),
            BangumiTab::Index => (
                self.index_loaded,
                self.index_loading,
                self.index_error.as_deref(),
                !self.index_grid.cards.is_empty(),
                "暂无符合条件的剧集",
            ),
        };

        if !loaded {
            if loading {
                self.render_loading_placeholder(frame, area, theme);
            } else if let Some(error) = error {
                self.render_error_placeholder(frame, area, theme, &format!("❌ {error}"));
            } else {
                self.render_placeholder(frame, area, theme, "按刷新键加载数据");
            }
            return;
        }
        if !has_data {
            self.render_placeholder(frame, area, theme, empty_text);
            return;
        }
        match self.tab {
            BangumiTab::Rank => self.rank_grid.render(frame, area, theme),
            BangumiTab::Index => self.index_grid.render(frame, area, theme),
            BangumiTab::Timeline => self.render_timeline(frame, area, theme),
        }
    }

    fn timeline_lines(&self, theme: &Theme) -> (Vec<Line<'static>>, Vec<Option<usize>>) {
        let mut lines = Vec::new();
        let mut index_map: Vec<Option<usize>> = Vec::new();
        let mut counter = 0usize;
        for day in &self.timeline_days {
            let today = day.is_today != 0;
            let mut header = format!(" 📅 {} {}", day.date, weekday_label(day.day_of_week));
            if today {
                header.push_str(" [今天]");
            }
            let header_style = if today {
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme.fg_accent)
                    .add_modifier(Modifier::BOLD)
            };
            lines.push(Line::from(Span::styled(header, header_style)));
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
                let mut spans = vec![
                    Span::styled(
                        if selected { " ▶ " } else { "   " },
                        Style::default().fg(theme.bilibili_pink),
                    ),
                    Span::styled(
                        format!("{} ", episode.pub_time),
                        Style::default().fg(theme.fg_muted),
                    ),
                ];
                if episode.published == 1 {
                    spans.push(Span::styled("[已播] ", Style::default().fg(theme.success)));
                } else {
                    spans.push(Span::styled("[未播] ", Style::default().fg(theme.warning)));
                }
                let mut title_style = Style::default().fg(theme.fg_primary);
                if selected {
                    title_style = title_style.add_modifier(Modifier::BOLD);
                }
                spans.push(Span::styled(episode.title.clone(), title_style));
                if !episode.pub_index.is_empty() {
                    spans.push(Span::styled(
                        format!(" {}", episode.pub_index),
                        Style::default().fg(theme.bilibili_cyan),
                    ));
                }
                if episode.delay != 0 {
                    let label = if episode.delay_reason.trim().is_empty() {
                        format!(" [延期 {}]", episode.delay_index)
                    } else {
                        format!(" [延期: {}]", episode.delay_reason)
                    };
                    spans.push(Span::styled(label, Style::default().fg(theme.error)));
                }
                if selected {
                    for span in &mut spans {
                        span.style = span.style.bg(theme.selection_bg);
                    }
                }
                lines.push(Line::from(spans));
                index_map.push(Some(counter - 1));
            }
        }
        (lines, index_map)
    }

    fn render_timeline(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
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
        self.timeline_scroll = offset;
        self.timeline_line_map = index_map;
        let paragraph = Paragraph::new(lines).scroll((offset, 0)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_subtle)),
        );
        frame.render_widget(paragraph, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        let items = match self.focus {
            BangumiFocus::Sources => vec![
                (
                    format!("{}/{}", keys.nav_up, keys.nav_down),
                    "切换视图".to_string(),
                    theme.fg_secondary,
                ),
                (
                    format!("{}/{}", keys.nav_right, keys.confirm),
                    "进入".to_string(),
                    theme.success,
                ),
                (
                    format!("{}/{}/{}", keys.tab_1, keys.tab_2, keys.tab_3),
                    "直达".to_string(),
                    theme.fg_secondary,
                ),
                (keys.refresh.clone(), "刷新".to_string(), theme.info),
            ],
            BangumiFocus::Filter => vec![
                (
                    format!("{}/{}", keys.nav_left, keys.nav_right),
                    "选项".to_string(),
                    theme.fg_secondary,
                ),
                (
                    format!("{}/{}", keys.nav_up, keys.confirm),
                    "切换值".to_string(),
                    theme.success,
                ),
                (
                    format!("{}/{}", keys.section_prev, keys.section_next),
                    "返回列表".to_string(),
                    theme.info,
                ),
            ],
            BangumiFocus::List => vec![
                (
                    format!("{}/{}", keys.nav_up, keys.nav_down),
                    "浏览".to_string(),
                    theme.fg_secondary,
                ),
                (
                    format!("{}/{}", keys.page_up, keys.page_down),
                    "翻页".to_string(),
                    theme.fg_secondary,
                ),
                (
                    keys.nav_left.clone(),
                    "切换面板".to_string(),
                    theme.fg_secondary,
                ),
                (keys.confirm.clone(), "打开".to_string(), theme.success),
                (
                    format!("{}/{}", keys.section_prev, keys.section_next),
                    "筛选".to_string(),
                    theme.bilibili_cyan,
                ),
                (keys.refresh.clone(), "刷新".to_string(), theme.info),
            ],
        };
        frame.render_widget(
            Paragraph::new(shortcut_footer(theme, items)).alignment(Alignment::Center),
            area,
        );
    }

    fn handle_sources_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        if keys.matches_down(key) {
            self.selected_source = (self.selected_source + 1) % SOURCE_COUNT;
        } else if keys.matches_up(key) {
            self.selected_source = if self.selected_source == 0 {
                SOURCE_COUNT - 1
            } else {
                self.selected_source - 1
            };
        } else if keys.matches_right(key) || keys.matches_confirm(key) {
            let target = source_tab(self.selected_source);
            self.focus = BangumiFocus::List;
            if target != self.tab {
                return Some(AppAction::SwitchBangumiTab(target));
            }
        }
        Some(AppAction::None)
    }

    fn handle_filter_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        if keys.matches_left(key) {
            if self.filter_cursor == 0 {
                self.selected_source = tab_index(self.tab);
                self.focus = BangumiFocus::Sources;
            } else {
                self.filter_cursor -= 1;
            }
            return Some(AppAction::None);
        }
        if keys.matches_right(key) {
            let max = self.filter_count().saturating_sub(1);
            self.filter_cursor = self.filter_cursor.saturating_add(1).min(max);
            return Some(AppAction::None);
        }
        if keys.matches_down(key) || key == KeyCode::Esc {
            self.focus = BangumiFocus::List;
            return Some(AppAction::None);
        }
        let forward = if keys.matches_up(key) {
            Some(false)
        } else if keys.matches_confirm(key) || key == KeyCode::Char(' ') {
            Some(true)
        } else {
            None
        };
        let Some(forward) = forward else {
            return Some(AppAction::None);
        };
        match self.tab {
            BangumiTab::Rank => Some(self.cycle_rank_filter(forward)),
            BangumiTab::Timeline => Some(self.cycle_timeline_filter(forward)),
            BangumiTab::Index => Some(self.cycle_index_filter(forward)),
        }
    }

    fn handle_list_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        if keys.matches_confirm(key) || keys.matches_play(key) {
            return self.activate_selected();
        }
        if keys.matches_left(key) {
            self.selected_source = tab_index(self.tab);
            self.focus = BangumiFocus::Sources;
            return Some(AppAction::None);
        }
        match self.tab {
            BangumiTab::Rank | BangumiTab::Index => {
                let mut moved_down = false;
                if keys.matches_up(key) {
                    let at_top = self
                        .active_grid()
                        .map(|grid| grid.selected_row() == 0)
                        .unwrap_or(true);
                    if at_top {
                        self.focus = BangumiFocus::Filter;
                        return Some(AppAction::None);
                    }
                    if let Some(grid) = self.active_grid_mut() {
                        grid.move_up();
                    }
                } else if keys.matches_down(key) {
                    if let Some(grid) = self.active_grid_mut() {
                        grid.move_down();
                    }
                    moved_down = true;
                } else if keys.matches_page_down(key) {
                    if let Some(grid) = self.active_grid_mut() {
                        grid.move_page_down();
                    }
                    moved_down = true;
                } else if keys.matches_page_up(key)
                    && let Some(grid) = self.active_grid_mut()
                {
                    grid.move_page_up();
                }
                if moved_down && self.can_load_more() {
                    return Some(AppAction::LoadMoreBangumi);
                }
                Some(AppAction::None)
            }
            BangumiTab::Timeline => {
                let count = self.timeline_episode_count();
                if count == 0 {
                    if keys.matches_up(key) {
                        self.focus = BangumiFocus::Filter;
                    }
                    return Some(AppAction::None);
                }
                if keys.matches_up(key) {
                    if self.timeline_selected == 0 {
                        self.focus = BangumiFocus::Filter;
                        return Some(AppAction::None);
                    }
                    self.timeline_selected -= 1;
                } else if keys.matches_down(key) {
                    self.timeline_selected = (self.timeline_selected + 1).min(count - 1);
                } else if keys.matches_page_down(key) {
                    self.timeline_selected =
                        (self.timeline_selected + TIMELINE_PAGE_STEP).min(count - 1);
                } else if keys.matches_page_up(key) {
                    self.timeline_selected =
                        self.timeline_selected.saturating_sub(TIMELINE_PAGE_STEP);
                }
                Some(AppAction::None)
            }
        }
    }

    fn register_click(&mut self, index: usize) -> Option<AppAction> {
        let now = Instant::now();
        let is_double = self.last_click_tab == Some(self.tab)
            && self.last_click_index == Some(index)
            && self
                .last_click_time
                .map(|last| now.duration_since(last) < DOUBLE_CLICK_WINDOW)
                .unwrap_or(false);
        self.last_click_time = Some(now);
        self.last_click_index = Some(index);
        self.last_click_tab = Some(self.tab);
        if is_double {
            self.activate_selected()
        } else {
            None
        }
    }

    fn handle_content_mouse(&mut self, event: MouseEvent, content: Rect) -> Option<AppAction> {
        match self.tab {
            BangumiTab::Rank | BangumiTab::Index => {
                let grid = self.active_grid_mut()?;
                let columns = grid.columns.max(1);
                let card_width = content.width / columns as u16;
                if card_width == 0 || grid.card_height == 0 {
                    return None;
                }
                let click_row = (event.row.saturating_sub(content.y) / grid.card_height) as usize;
                let click_col = (event.column.saturating_sub(content.x) / card_width) as usize;
                let click_col = click_col.min(columns - 1);
                let click_idx = (grid.scroll_row + click_row) * columns + click_col;
                if click_idx >= grid.cards.len() {
                    return None;
                }
                grid.selected_index = click_idx;
                grid.update_scroll(grid.cached_visible_rows);
                self.register_click(click_idx)
            }
            BangumiTab::Timeline => {
                // Text is drawn inside the block border: rows content.y and
                // content.y + height - 1 are border rows.
                if event.row <= content.y
                    || event.row >= content.y + content.height.saturating_sub(1)
                {
                    return None;
                }
                let line =
                    self.timeline_scroll
                        .saturating_add(event.row - (content.y + 1)) as usize;
                let index = match self.timeline_line_map.get(line).copied() {
                    Some(Some(index)) => index,
                    _ => return None,
                };
                self.timeline_selected = index;
                self.register_click(index)
            }
        }
    }
}

impl Default for BangumiPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for BangumiPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        let layout = PageLayout::new(area);
        self.draw_sources(frame, layout.sources, theme);
        self.render_header(frame, layout.header, theme);
        self.render_filter_bar(frame, layout.filters, theme);
        self.render_content(frame, layout.content, theme);
        self.render_footer(frame, layout.footer, theme, keys);
    }

    fn handle_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        // Global keybindings stay responsive while loading or after failures.
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
        if keys.matches_section_prev(key)
            || keys.matches_section_next(key)
            || key == KeyCode::Char('f')
        {
            self.toggle_filter_focus();
            return Some(AppAction::None);
        }

        match self.focus {
            BangumiFocus::Sources => self.handle_sources_input(key, keys),
            BangumiFocus::Filter => self.handle_filter_input(key, keys),
            BangumiFocus::List => self.handle_list_input(key, keys),
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        if area.height < 8 || area.width < 40 {
            return None;
        }
        let layout = PageLayout::new(area);
        let position = Position::new(event.column, event.row);

        if layout.sources.contains(position) {
            match event.kind {
                MouseEventKind::ScrollDown => {
                    self.selected_source = (self.selected_source + 1) % SOURCE_COUNT;
                    self.focus = BangumiFocus::Sources;
                }
                MouseEventKind::ScrollUp => {
                    self.selected_source = if self.selected_source == 0 {
                        SOURCE_COUNT - 1
                    } else {
                        self.selected_source - 1
                    };
                    self.focus = BangumiFocus::Sources;
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if event.row <= layout.sources.y
                        || event.row >= layout.sources.bottom().saturating_sub(1)
                    {
                        return Some(AppAction::None);
                    }
                    let row = (event.row - (layout.sources.y + 1)) as usize;
                    if row < SOURCE_COUNT {
                        self.selected_source = row;
                        let target = source_tab(row);
                        if target != self.tab {
                            return Some(AppAction::SwitchBangumiTab(target));
                        }
                        self.focus = BangumiFocus::Sources;
                    }
                }
                _ => {}
            }
            return Some(AppAction::None);
        }

        if !layout.main.contains(position) {
            return None;
        }

        if layout.filters.contains(position) {
            if matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.focus = BangumiFocus::Filter;
                let inner_x = layout.filters.x.saturating_add(1);
                let inner_y = layout.filters.y.saturating_add(1);
                if event.row == inner_y {
                    let column = event.column.saturating_sub(inner_x);
                    if let Some(index) = self
                        .filter_hit_ranges
                        .iter()
                        .find(|(_, start, end)| column >= *start && column < *end)
                        .map(|(index, _, _)| *index)
                    {
                        self.filter_cursor = index;
                    }
                }
            }
            return Some(AppAction::None);
        }

        if layout.content.contains(position) {
            match event.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    self.focus = BangumiFocus::List;
                    return self.handle_content_mouse(event, layout.content);
                }
                MouseEventKind::Down(MouseButton::Middle) => {
                    self.focus = BangumiFocus::List;
                    return self.activate_selected();
                }
                MouseEventKind::ScrollDown => {
                    self.focus = BangumiFocus::List;
                    if let Some(grid) = self.active_grid_mut() {
                        grid.move_down();
                        let near_bottom = grid.is_near_bottom(grid.cached_visible_rows);
                        if near_bottom && self.can_load_more() {
                            return Some(AppAction::LoadMoreBangumi);
                        }
                    } else if self.tab == BangumiTab::Timeline {
                        let count = self.timeline_episode_count();
                        if count > 0 {
                            self.timeline_selected = (self.timeline_selected + 1).min(count - 1);
                        }
                    }
                }
                MouseEventKind::ScrollUp => {
                    self.focus = BangumiFocus::List;
                    if let Some(grid) = self.active_grid_mut() {
                        grid.move_up();
                    } else if self.tab == BangumiTab::Timeline {
                        self.timeline_selected = self.timeline_selected.saturating_sub(1);
                    }
                }
                _ => {}
            }
            return Some(AppAction::None);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::KeyModifiers;

    fn keys() -> Keybindings {
        Keybindings::default()
    }

    fn render(page: &mut BangumiPage, width: u16, height: u16) -> String {
        let mut terminal = ratatui::Terminal::new(TestBackend::new(width, height)).unwrap();
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

    fn rank_item(season_id: i64) -> SeasonRankItem {
        serde_json::from_value(serde_json::json!({
            "badge": "独家",
            "cover": "//i0.hdslb.com/rank.jpg",
            "new_ep": {"index_show": "全12话"},
            "season_id": season_id,
            "title": format!("rank-{season_id}"),
            "rating": "9.1",
            "stat": {"view": 12_000}
        }))
        .unwrap()
    }

    fn index_page(num: i32, has_next: i32, season_ids: &[i64]) -> SeasonIndexPage {
        serde_json::from_value(serde_json::json!({
            "has_next": has_next,
            "list": season_ids
                .iter()
                .map(|season_id| serde_json::json!({
                    "season_id": season_id,
                    "media_id": 1000 + season_id,
                    "title": format!("index-{season_id}"),
                    "subTitle": "更新至第3话",
                    "cover": "//i0.hdslb.com/index.jpg",
                    "index_show": "全12话",
                    "score": "8.8",
                    "badge": "独家",
                    "is_finish": 0,
                    "link": "https://www.bilibili.com/bangumi/play/ss1"
                }))
                .collect::<Vec<_>>(),
            "num": num,
            "size": 20,
            "total": 100
        }))
        .unwrap()
    }

    fn episode(
        season_id: i64,
        episode_id: i64,
        title: &str,
        published: i32,
        delay: i32,
        delay_reason: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "cover": "//i0.hdslb.com/cover.jpg",
            "delay": delay,
            "delay_id": 4,
            "delay_index": "2",
            "delay_reason": delay_reason,
            "ep_cover": "//i0.hdslb.com/ep.jpg",
            "episode_id": episode_id,
            "follows": "1.0万",
            "plays": "2.0万",
            "pub_index": "第1话",
            "pub_time": "21:00",
            "pub_ts": 1_764_201_600_i64,
            "published": published,
            "season_id": season_id,
            "square_cover": "//i0.hdslb.com/square.jpg",
            "title": title.to_string()
        })
    }

    fn timeline_day(
        date: &str,
        day_of_week: i32,
        is_today: i32,
        episodes: Vec<serde_json::Value>,
    ) -> TimelineDay {
        serde_json::from_value(serde_json::json!({
            "date": date,
            "date_ts": 1_764_201_600_i64,
            "day_of_week": day_of_week,
            "is_today": is_today,
            "episodes": episodes
        }))
        .unwrap()
    }

    fn loaded_index(page: &mut BangumiPage, ids: &[i64], has_next: i32) {
        page.begin_load(BangumiTab::Index);
        let query = page.season_index_query.clone();
        assert!(page.apply_season_index_page(&query, false, index_page(1, has_next, ids)));
    }

    #[test]
    fn tabs_switch_via_keys_and_global_keys_stay_responsive() {
        let mut page = BangumiPage::new();
        let keys = keys();

        assert!(matches!(
            page.handle_input(KeyCode::Char('q'), &keys),
            Some(AppAction::Quit)
        ));
        assert!(matches!(
            page.handle_input(KeyCode::Tab, &keys),
            Some(AppAction::NavNext)
        ));
        assert!(matches!(
            page.handle_input(KeyCode::BackTab, &keys),
            Some(AppAction::NavPrev)
        ));
        assert!(matches!(
            page.handle_input(KeyCode::Char('t'), &keys),
            Some(AppAction::NextTheme)
        ));
        assert!(matches!(
            page.handle_input(KeyCode::Char('s'), &keys),
            Some(AppAction::SwitchToSettings)
        ));

        // Global keys work while a request is in flight.
        page.begin_load(BangumiTab::Rank);
        assert!(page.rank_loading);
        assert!(matches!(
            page.handle_input(KeyCode::Char('q'), &keys),
            Some(AppAction::Quit)
        ));

        assert!(matches!(
            page.handle_input(KeyCode::Char('2'), &keys),
            Some(AppAction::SwitchBangumiTab(BangumiTab::Timeline))
        ));
        assert!(matches!(
            page.handle_input(KeyCode::Char('3'), &keys),
            Some(AppAction::SwitchBangumiTab(BangumiTab::Index))
        ));
        assert!(matches!(
            page.handle_input(KeyCode::Char('1'), &keys),
            Some(AppAction::SwitchBangumiTab(BangumiTab::Rank))
        ));
    }

    #[test]
    fn all_three_tabs_render_data_empty_and_error_states() {
        let mut page = BangumiPage::new();

        // Rank: an empty response still counts as loaded (综艺 returns 0 items).
        page.rank_season_type = BangumiSeasonType::Variety;
        assert!(page.apply_rank(
            BangumiRankDay::ThreeDays,
            BangumiSeasonType::Variety,
            Vec::new(),
        ));
        assert!(page.rank_loaded);
        assert!(!page.tab_needs_load());
        let text = render(&mut page, 100, 30);
        assert!(text.contains("番剧排行"));
        assert!(text.contains("新番时间线"));
        assert!(text.contains("番剧索引"));
        assert!(text.contains("番剧番剧排行"));
        assert!(text.contains("暂无排行数据"));

        // Rank data state.
        page.rank_season_type = BangumiSeasonType::Anime;
        assert!(page.apply_rank(
            BangumiRankDay::ThreeDays,
            BangumiSeasonType::Anime,
            vec![rank_item(4)],
        ));
        let text = render(&mut page, 100, 30);
        assert!(text.contains("rank-4"));

        // Rank initial error state.
        page.rank_loaded = false;
        page.rank_error = None;
        page.fail(BangumiTab::Rank, "加载番剧排行失败: 网络错误".to_string());
        let text = render(&mut page, 100, 30);
        assert!(text.contains("加载番剧排行失败"));
        assert!(text.contains('❌'));

        // Timeline empty.
        page.set_tab(BangumiTab::Timeline);
        page.timeline_loaded = true;
        let text = render(&mut page, 100, 30);
        assert!(text.contains("暂无时间线数据"));

        // Index empty and initial error.
        page.set_tab(BangumiTab::Index);
        page.index_loaded = true;
        let text = render(&mut page, 100, 30);
        assert!(text.contains("暂无符合条件的剧集"));
        page.index_loaded = false;
        page.fail(BangumiTab::Index, "加载番剧索引失败".to_string());
        let text = render(&mut page, 100, 30);
        assert!(text.contains("加载番剧索引失败"));
    }

    #[test]
    fn rank_filters_cycle_and_emit_load_actions() {
        let mut page = BangumiPage::new();
        let keys = keys();

        page.handle_input(KeyCode::Char('f'), &keys);
        assert_eq!(page.focus, BangumiFocus::Filter);
        assert_eq!(page.filter_cursor, 0);

        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadBangumiRank { day, season_type }) => {
                assert_eq!(day, BangumiRankDay::Week);
                assert_eq!(season_type, BangumiSeasonType::Anime);
            }
            other => panic!("expected rank load, got {other:?}"),
        }

        page.handle_input(KeyCode::Char('l'), &keys);
        assert_eq!(page.filter_cursor, 1);
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadBangumiRank { day, season_type }) => {
                assert_eq!(day, BangumiRankDay::Week);
                assert_eq!(season_type, BangumiSeasonType::Movie);
            }
            other => panic!("expected rank load, got {other:?}"),
        }

        // Up cycles the same chip backwards.
        match page.handle_input(KeyCode::Up, &keys) {
            Some(AppAction::LoadBangumiRank { season_type, .. }) => {
                assert_eq!(season_type, BangumiSeasonType::Anime);
            }
            other => panic!("expected rank load, got {other:?}"),
        }

        // Down leaves the filter bar.
        page.handle_input(KeyCode::Down, &keys);
        assert_eq!(page.focus, BangumiFocus::List);
    }

    #[test]
    fn timeline_groups_days_marks_status_and_opens_episode() {
        let mut page = BangumiPage::new();
        page.set_tab(BangumiTab::Timeline);
        let keys = keys();

        page.begin_load(BangumiTab::Timeline);
        assert!(page.apply_timeline(
            TimelineType::Anime,
            3,
            3,
            vec![
                timeline_day(
                    "2026-11-26",
                    4,
                    0,
                    vec![episode(7, 70, "昨天的那集", 1, 0, "")]
                ),
                timeline_day(
                    "2026-11-27",
                    5,
                    1,
                    vec![
                        episode(7, 71, "今天的第一集", 1, 0, ""),
                        episode(8, 81, "今天的第二集", 0, 1, "延期原因"),
                    ]
                ),
                timeline_day("2026-11-28", 6, 0, Vec::new()),
            ],
        ));

        // Today's first episode is selected by default.
        assert_eq!(page.timeline_selected, 1);
        let text = render(&mut page, 100, 40);
        assert!(text.contains("📅"));
        assert!(text.contains("今天"));
        assert!(text.contains("已播"));
        assert!(text.contains("未播"));
        assert!(text.contains("延期:延期原因"));
        assert!(text.contains("21:00"));
        assert!(text.contains("（无更新）"));

        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::OpenBangumiEpisode { season_id, ep_id }) => {
                assert_eq!((season_id, ep_id), (7, 71));
            }
            other => panic!("expected episode, got {other:?}"),
        }

        // Down moves to the next flat episode, skipping day headers.
        page.handle_input(KeyCode::Down, &keys);
        assert_eq!(page.timeline_selected, 2);
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::OpenBangumiEpisode { season_id, ep_id }) => {
                assert_eq!((season_id, ep_id), (8, 81));
            }
            other => panic!("expected episode, got {other:?}"),
        }
    }

    #[test]
    fn timeline_defaults_to_first_today_episode_and_filter_updates() {
        let mut page = BangumiPage::new();
        page.set_tab(BangumiTab::Timeline);
        let keys = keys();

        page.begin_load(BangumiTab::Timeline);
        assert!(page.apply_timeline(
            TimelineType::Anime,
            3,
            3,
            vec![
                timeline_day("2026-11-25", 3, 0, vec![episode(1, 11, "a", 1, 0, "")]),
                timeline_day("2026-11-27", 5, 1, vec![episode(2, 21, "b", 1, 0, "")]),
            ],
        ));
        assert_eq!(page.timeline_selected, 1);

        page.handle_input(KeyCode::Char('f'), &keys);
        // Category chip (index 0) cycles to Movie.
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadBangumiTimeline {
                types,
                before,
                after,
            }) => {
                assert_eq!(types, TimelineType::Movie);
                assert_eq!((before, after), (3, 3));
            }
            other => panic!("expected timeline load, got {other:?}"),
        }

        // Before chip (index 1) cycles 3 -> 4.
        page.handle_input(KeyCode::Char('l'), &keys);
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadBangumiTimeline { before, .. }) => assert_eq!(before, 4),
            other => panic!("expected timeline load, got {other:?}"),
        }

        // After chip (index 2) cycles 3 -> 4.
        page.handle_input(KeyCode::Char('l'), &keys);
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadBangumiTimeline { after, .. }) => assert_eq!(after, 4),
            other => panic!("expected timeline load, got {other:?}"),
        }

        // Values wrap inside the live-verified 0..=7 window.
        page.timeline_after = 7;
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadBangumiTimeline { after, .. }) => assert_eq!(after, 0),
            other => panic!("expected timeline load, got {other:?}"),
        }
    }

    #[test]
    fn index_filters_cycle_and_reset_to_page_one() {
        let mut page = BangumiPage::new();
        page.set_tab(BangumiTab::Index);
        let keys = keys();

        page.handle_input(KeyCode::Char('f'), &keys);
        assert_eq!(page.focus, BangumiFocus::Filter);

        // Category: Anime -> Movie.
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadSeasonIndex { query, append }) => {
                assert!(!append);
                assert_eq!(query.page, 1);
                assert_eq!(query.season_type, BangumiSeasonType::Movie);
                assert_eq!(query, page.season_index_query);
            }
            other => panic!("expected index load, got {other:?}"),
        }

        // Order chip: Follow -> UpdateTime.
        page.handle_input(KeyCode::Char('l'), &keys);
        assert_eq!(page.filter_cursor, 1);
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadSeasonIndex { query, .. }) => {
                assert_eq!(query.order, SeasonIndexOrder::UpdateTime);
                assert_eq!(query.page, 1);
            }
            other => panic!("expected index load, got {other:?}"),
        }

        // Direction chip.
        page.handle_input(KeyCode::Char('l'), &keys);
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadSeasonIndex { query, .. }) => {
                assert_eq!(query.sort, SeasonIndexSort::Asc);
            }
            other => panic!("expected index load, got {other:?}"),
        }

        // Year chip: 全部 -> 2026.
        page.handle_input(KeyCode::Char('l'), &keys);
        assert_eq!(page.filter_cursor, 3);
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadSeasonIndex { query, .. }) => {
                assert_eq!(query.year, "[2026,2027)");
                assert_eq!(query.page, 1);
            }
            other => panic!("expected index load, got {other:?}"),
        }

        // A page-2 cursor still reloads page 1 after a filter change.
        page.season_index_query.page = 3;
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadSeasonIndex { query, append }) => {
                assert!(!append);
                assert_eq!(query.page, 1);
            }
            other => panic!("expected index load, got {other:?}"),
        }

        // Status / payment / area chips expose the live-verified values.
        page.filter_cursor = 4;
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadSeasonIndex { query, .. }) => assert_eq!(query.is_finish, 1),
            other => panic!("expected index load, got {other:?}"),
        }
        page.filter_cursor = 5;
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadSeasonIndex { query, .. }) => {
                assert_eq!(query.season_status, "1");
                assert_eq!(query.copyright, "-1");
            }
            other => panic!("expected index load, got {other:?}"),
        }
        page.filter_cursor = 6;
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadSeasonIndex { query, .. }) => assert_eq!(query.area, "2"),
            other => panic!("expected index load, got {other:?}"),
        }
    }

    #[test]
    fn index_pagination_deduplicates_and_stops_at_the_end() {
        let mut page = BangumiPage::new();
        page.set_tab(BangumiTab::Index);
        let keys = keys();

        loaded_index(&mut page, &[1, 2, 3], 1);
        assert_eq!(page.index_season_ids, vec![1, 2, 3]);
        assert_eq!(page.index_loaded_page, 1);
        assert_eq!(page.index_total, 100);
        assert!(page.index_has_next);
        let text = render(&mut page, 100, 40);
        assert!(text.contains("已加载3/100"));

        // Load-more request: page 2, append with an overlapping id.
        page.begin_index_load_more();
        assert!(page.index_loading_more);
        let mut next = page.season_index_query.clone();
        next.page = 2;
        assert!(page.apply_season_index_page(&next, true, index_page(2, 1, &[2, 3, 4])));
        assert_eq!(page.index_season_ids, vec![1, 2, 3, 4]);
        assert_eq!(page.index_loaded_page, 2);
        assert!(!page.index_loading_more);
        assert!(page.index_has_next);

        // Last page: has_next == 0, so downward navigation never asks again.
        page.begin_index_load_more();
        let mut next = page.season_index_query.clone();
        next.page = 3;
        assert!(page.apply_season_index_page(&next, true, index_page(3, 0, &[5])));
        assert_eq!(page.index_season_ids, vec![1, 2, 3, 4, 5]);
        assert!(!page.index_has_next);
        assert!(!page.can_load_more());

        page.index_grid.cached_visible_rows = 1;
        let action = page.handle_input(KeyCode::Down, &keys);
        assert!(matches!(action, Some(AppAction::None)));

        // Re-enable pagination and verify the near-bottom trigger fires.
        page.index_has_next = true;
        let action = page.handle_input(KeyCode::Down, &keys);
        assert!(matches!(action, Some(AppAction::LoadMoreBangumi)));
    }

    #[test]
    fn index_load_more_failure_keeps_rows_and_clears_loading() {
        let mut page = BangumiPage::new();
        page.set_tab(BangumiTab::Index);

        loaded_index(&mut page, &[1, 2], 1);
        page.begin_index_load_more();
        page.fail(BangumiTab::Index, "加载更多失败".to_string());

        assert_eq!(page.index_season_ids, vec![1, 2]);
        assert!(!page.index_loading_more);
        assert!(page.index_loaded);
        assert!(page.index_has_next);
        assert_eq!(page.index_notice.as_deref(), Some("加载更多失败"));

        let text = render(&mut page, 100, 40);
        assert!(text.contains("加载更多失败"));

        // A fresh page replaces the notice.
        page.begin_index_load_more();
        let mut next = page.season_index_query.clone();
        next.page = 2;
        assert!(page.apply_season_index_page(&next, true, index_page(2, 0, &[3])));
        assert!(page.index_notice.is_none());
    }

    #[test]
    fn mouse_click_on_left_pane_selects_and_switches_views() {
        let mut page = BangumiPage::new();
        let area = Rect::new(0, 0, 100, 30);
        render(&mut page, area.width, area.height);
        let sources = PageLayout::new(area).sources;
        page.selected_source = 2;

        // Border rows are ignored.
        assert!(matches!(
            page.handle_mouse(click(3, sources.y), area),
            Some(AppAction::None)
        ));
        assert_eq!(page.selected_source, 2);

        // Clicking the Timeline row switches and selects it.
        match page.handle_mouse(click(3, sources.y + 2), area) {
            Some(AppAction::SwitchBangumiTab(tab)) => assert_eq!(tab, BangumiTab::Timeline),
            other => panic!("expected view switch, got {other:?}"),
        }
        assert_eq!(page.selected_source, 1);

        // Clicking the active view's row only focuses the left pane.
        page.set_tab(BangumiTab::Index);
        page.focus = BangumiFocus::List;
        assert!(matches!(
            page.handle_mouse(click(3, sources.y + 3), area),
            Some(AppAction::None)
        ));
        assert_eq!(page.selected_source, 2);
        assert_eq!(page.focus, BangumiFocus::Sources);

        // Wheel scrolling cycles the views.
        page.handle_mouse(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 3,
                row: sources.y + 2,
                modifiers: KeyModifiers::NONE,
            },
            area,
        );
        assert_eq!(page.selected_source, 0);
        assert_eq!(page.focus, BangumiFocus::Sources);
    }

    #[test]
    fn sources_pane_keyboard_cycles_and_enters_views() {
        let mut page = BangumiPage::new();
        let keys = keys();
        assert_eq!(page.focus, BangumiFocus::Sources);
        assert_eq!(page.selected_source, 0);

        page.handle_input(KeyCode::Down, &keys);
        assert_eq!(page.selected_source, 1);
        assert_eq!(page.tab, BangumiTab::Rank);
        page.handle_input(KeyCode::Down, &keys);
        assert_eq!(page.selected_source, 2);
        page.handle_input(KeyCode::Down, &keys);
        assert_eq!(page.selected_source, 0);
        page.handle_input(KeyCode::Up, &keys);
        assert_eq!(page.selected_source, 2);

        // Enter on a different view emits a switch and lands in the list.
        page.selected_source = 1;
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::SwitchBangumiTab(tab)) => assert_eq!(tab, BangumiTab::Timeline),
            other => panic!("expected view switch, got {other:?}"),
        }
        assert_eq!(page.focus, BangumiFocus::List);

        // Enter on the active view enters the list without an action.
        page.focus = BangumiFocus::Sources;
        page.selected_source = 0;
        assert!(matches!(
            page.handle_input(KeyCode::Right, &keys),
            Some(AppAction::None)
        ));
        assert_eq!(page.focus, BangumiFocus::List);
    }

    #[test]
    fn list_and_filter_return_focus_to_sources_synced_with_tab() {
        let mut page = BangumiPage::new();
        let keys = keys();

        page.set_tab(BangumiTab::Timeline);
        assert_eq!(page.focus, BangumiFocus::List);
        page.selected_source = 0;
        page.handle_input(KeyCode::Left, &keys);
        assert_eq!(page.focus, BangumiFocus::Sources);
        assert_eq!(page.selected_source, 1);

        page.set_tab(BangumiTab::Index);
        page.handle_input(KeyCode::Char('f'), &keys);
        assert_eq!(page.focus, BangumiFocus::Filter);
        assert_eq!(page.filter_cursor, 0);
        page.selected_source = 0;
        page.handle_input(KeyCode::Left, &keys);
        assert_eq!(page.focus, BangumiFocus::Sources);
        assert_eq!(page.selected_source, 2);
    }

    #[test]
    fn grids_use_single_column_list_layout() {
        let page = BangumiPage::new();
        for grid in [&page.rank_grid, &page.index_grid] {
            assert_eq!(grid.columns, 1);
            assert!(grid.list_layout);
            assert_eq!(grid.card_height, 7);
        }
    }

    #[test]
    fn renders_two_pane_layout_header_filters_and_footer() {
        let mut page = BangumiPage::new();
        let text = render(&mut page, 120, 40);
        assert!(text.contains("番剧番剧排行"));
        assert!(text.contains("番剧索引"));
        assert!(text.contains("周期"));
        assert!(text.contains("类别"));
        assert!(text.contains("切换视图"));
        assert!(text.contains("按刷新键加载数据"));
    }

    fn click(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn index_payment_filter_updates_season_status_only() {
        let mut page = BangumiPage::new();
        page.set_tab(BangumiTab::Index);
        let keys = keys();

        page.handle_input(KeyCode::Char('f'), &keys);
        assert_eq!(page.focus, BangumiFocus::Filter);
        page.filter_cursor = 5;
        assert_eq!(
            page.filter_cursor,
            IndexFilterField::ALL
                .iter()
                .position(|field| *field == IndexFilterField::Payment)
                .unwrap()
        );

        for expected in ["1", "2,6", "4,6"] {
            match page.handle_input(KeyCode::Enter, &keys) {
                Some(AppAction::LoadSeasonIndex { query, append }) => {
                    assert!(!append);
                    assert_eq!(query.page, 1);
                    assert_eq!(query.season_status, expected);
                    assert_eq!(query.copyright, "-1");
                }
                other => panic!("expected index load, got {other:?}"),
            }
        }
        assert_eq!(page.index_filter_value(IndexFilterField::Payment), "大会员");

        // Cycling wraps back to 全部.
        match page.handle_input(KeyCode::Enter, &keys) {
            Some(AppAction::LoadSeasonIndex { query, .. }) => {
                assert_eq!(query.season_status, "-1");
                assert_eq!(query.copyright, "-1");
            }
            other => panic!("expected index load, got {other:?}"),
        }
    }

    #[test]
    fn grid_click_selects_clicked_row() {
        let mut page = BangumiPage::new();
        page.begin_load(BangumiTab::Rank);
        let day = page.rank_day;
        let season_type = page.rank_season_type;
        assert!(page.apply_rank(day, season_type, (1..=6).map(rank_item).collect::<Vec<_>>(),));

        let area = Rect::new(0, 0, 120, 40);
        render(&mut page, area.width, area.height);
        let content = PageLayout::new(area).content;
        assert_eq!(page.rank_grid.columns, 1);

        // Rows are stacked one card per row.
        assert!(
            page.handle_mouse(click(content.x + 1, content.y + 1), area)
                .is_none()
        );
        assert_eq!(page.rank_grid.selected_index, 0);
        assert_eq!(page.focus, BangumiFocus::List);

        assert!(
            page.handle_mouse(
                click(content.x + 1, content.y + page.rank_grid.card_height + 1),
                area,
            )
            .is_none()
        );
        assert_eq!(page.rank_grid.selected_index, 1);

        // A click past the loaded cards selects nothing.
        assert!(
            page.handle_mouse(
                click(
                    content.x + 1,
                    content.y + page.rank_grid.card_height * 2 + 1
                ),
                area,
            )
            .is_none()
        );
        assert_eq!(page.rank_grid.selected_index, 2);
        assert!(
            page.handle_mouse(
                click(
                    content.x + 1,
                    content.y + page.rank_grid.card_height * 6 + 1
                ),
                area,
            )
            .is_none()
        );
        assert_eq!(page.rank_grid.selected_index, 2);
    }

    #[test]
    fn content_mouse_scroll_and_middle_click_on_index() {
        let mut page = BangumiPage::new();
        page.set_tab(BangumiTab::Index);
        loaded_index(&mut page, &[11, 12, 13], 1);

        let area = Rect::new(0, 0, 120, 40);
        render(&mut page, area.width, area.height);
        // Force a one-row viewport so the near-bottom trigger fires.
        page.index_grid.cached_visible_rows = 1;
        let content = PageLayout::new(area).content;

        // Left click selects the first row and focuses the list.
        assert!(
            page.handle_mouse(click(content.x + 1, content.y + 1), area)
                .is_none()
        );
        assert_eq!(page.index_grid.selected_index, 0);
        assert_eq!(page.focus, BangumiFocus::List);

        // Scrolling down moves one card and near the bottom asks for more.
        let action = page.handle_mouse(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: content.x + 1,
                row: content.y + 1,
                modifiers: KeyModifiers::NONE,
            },
            area,
        );
        assert!(matches!(action, Some(AppAction::LoadMoreBangumi)));
        assert_eq!(page.index_grid.selected_index, 1);

        // Middle click opens the selection.
        match page.handle_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Middle),
                column: content.x + 1,
                row: content.y + 1,
                modifiers: KeyModifiers::NONE,
            },
            area,
        ) {
            Some(AppAction::OpenBangumiDetail(season_id)) => assert_eq!(season_id, 12),
            other => panic!("expected detail open, got {other:?}"),
        }
    }

    #[test]
    fn timeline_click_accounts_for_top_border() {
        let mut page = BangumiPage::new();
        page.set_tab(BangumiTab::Timeline);
        page.begin_load(BangumiTab::Timeline);
        assert!(page.apply_timeline(
            TimelineType::Anime,
            3,
            3,
            vec![timeline_day(
                "2026-11-27",
                5,
                1,
                vec![episode(7, 71, "a", 1, 0, ""), episode(8, 81, "b", 0, 0, ""),],
            )],
        ));

        let area = Rect::new(0, 0, 100, 40);
        render(&mut page, area.width, area.height);
        let content = PageLayout::new(area).content;
        page.timeline_selected = 1;

        // The top border and the day header are not selectable.
        assert!(
            page.handle_mouse(click(content.x + 2, content.y), area)
                .is_none()
        );
        assert_eq!(page.timeline_selected, 1);
        assert!(
            page.handle_mouse(click(content.x + 2, content.y + 1), area)
                .is_none()
        );
        assert_eq!(page.timeline_selected, 1);

        // The first episode is the second inner line, not the first.
        assert!(
            page.handle_mouse(click(content.x + 2, content.y + 2), area)
                .is_none()
        );
        assert_eq!(page.timeline_selected, 0);
    }
}
