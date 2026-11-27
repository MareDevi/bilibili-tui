//! Bangumi detail page showing season info, seasons, episode list and comments

use super::{Component, Theme, shortcut_footer};
use crate::api::bangumi::{BangumiEpisode, SeasonQuery, SeasonResult};
use crate::api::client::ApiClient;
use crate::api::comment::CommentItem;
use crate::application::AppAction;
use crate::storage::Keybindings;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind},
    prelude::*,
    widgets::*,
};
use std::collections::HashSet;
use std::time::Instant;

/// Focusable panes on the bangumi detail page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BangumiDetailFocus {
    Info,
    Seasons,
    Episodes,
    Comments,
}

/// How many comment rows are assumed visible on screen when deciding whether
/// the list is scrolled close enough to the bottom to load more.
const COMMENT_VISIBLE_ROWS: usize = 12;

pub struct BangumiDetailPage {
    pub season_id: i64,
    pub season: Option<SeasonResult>,
    pub loading: bool,
    pub error_message: Option<String>,
    pub episode_scroll: usize,
    pub selected_episode: usize,
    target_episode_id: Option<i64>,
    pub auto_play_pending: bool,
    // Flat list of all episodes for navigation
    flat_episodes: Vec<FlatEpisode>,
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
    // Layout / focus state
    pub focus: BangumiDetailFocus,
    pub selected_season: usize,
    pub info_scroll: u16,
    info_line_count: usize,
    // Episode comments
    pub comments: Vec<CommentItem>,
    pub comments_aid: Option<i64>,
    pub comments_loading: bool,
    pub loading_more_comments: bool,
    pub comment_page: i32,
    pub has_more_comments: bool,
    pub comment_scroll: usize,
    pub comments_error: Option<String>,
}

#[derive(Clone)]
struct FlatEpisode {
    section_title: String,
    _ep_index: usize,
    episode: BangumiEpisode,
}

struct BangumiDetailLayout {
    info: Rect,
    seasons: Option<Rect>,
    main: Rect,
    footer: Rect,
}

impl BangumiDetailPage {
    pub fn new(season_id: i64) -> Self {
        Self {
            season_id,
            season: None,
            loading: true,
            error_message: None,
            episode_scroll: 0,
            selected_episode: 0,
            target_episode_id: None,
            auto_play_pending: false,
            flat_episodes: Vec::new(),
            last_click_time: None,
            last_click_index: None,
            focus: BangumiDetailFocus::Episodes,
            selected_season: 0,
            info_scroll: 0,
            info_line_count: 0,
            comments: Vec::new(),
            comments_aid: None,
            comments_loading: false,
            loading_more_comments: false,
            comment_page: 0,
            has_more_comments: false,
            comment_scroll: 0,
            comments_error: None,
        }
    }

    pub fn new_for_episode(season_id: i64, ep_id: i64, auto_play: bool) -> Self {
        let mut page = Self::new(season_id);
        page.target_episode_id = Some(ep_id);
        page.auto_play_pending = auto_play;
        page
    }

    pub fn set_season(&mut self, season: SeasonResult) {
        self.flat_episodes.clear();
        for section in season.all_sections() {
            for (ep_idx, ep) in section.episodes.iter().enumerate() {
                self.flat_episodes.push(FlatEpisode {
                    section_title: section.title.clone(),
                    _ep_index: ep_idx,
                    episode: ep.clone(),
                });
            }
        }
        self.selected_season = season
            .seasons
            .iter()
            .position(|related| related.season_id == self.season_id)
            .unwrap_or(0);
        if season.seasons.len() <= 1 && self.focus == BangumiDetailFocus::Seasons {
            self.focus = BangumiDetailFocus::Episodes;
        }
        self.season = Some(season);
        self.loading = false;
        self.error_message = None;
        self.info_scroll = 0;
        self.comments.clear();
        self.comments_aid = None;
        self.comments_loading = false;
        self.loading_more_comments = false;
        self.comment_page = 0;
        self.has_more_comments = false;
        self.comment_scroll = 0;
        self.comments_error = None;
        let target_index = self.target_episode_id.and_then(|ep_id| {
            self.flat_episodes
                .iter()
                .position(|episode| episode.episode.id == ep_id)
        });
        self.selected_episode = target_index.unwrap_or(0);
        if self.target_episode_id.is_some() && target_index.is_none() {
            self.auto_play_pending = false;
            self.error_message = Some("未找到历史记录对应的番剧剧集".to_string());
        }
        self.episode_scroll = 0;
        self.update_scroll();
    }

    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
        self.loading = false;
    }

    pub async fn load_data(&mut self, api_client: &ApiClient) {
        self.loading = true;
        self.error_message = None;
        match api_client
            .get_bangumi_season(SeasonQuery::SeasonId(self.season_id))
            .await
        {
            Ok(season) => self.set_season(season),
            Err(e) => self.set_error(format!("加载番剧详情失败: {}", e)),
        }
    }

    fn selected_action(&self) -> Option<AppAction> {
        self.flat_episodes
            .get(self.selected_episode)
            .map(|fe| AppAction::PlayBangumiEpisode {
                ep_id: fe.episode.id,
                season_id: self.season_id,
                title: fe.episode.display_title(),
            })
    }

    pub fn play_action(&self) -> Option<AppAction> {
        self.selected_action()
    }

    /// The `aid` of the currently selected episode, or `0` when the episode
    /// carries no playable video (e.g. a section preview entry).
    pub fn selected_episode_aid(&self) -> i64 {
        self.flat_episodes
            .get(self.selected_episode)
            .map(|episode| episode.episode.aid)
            .unwrap_or(0)
    }

    fn move_down(&mut self, step: usize) {
        if self.flat_episodes.is_empty() {
            return;
        }
        self.selected_episode =
            (self.selected_episode + step).min(self.flat_episodes.len().saturating_sub(1));
        self.update_scroll();
    }

    fn move_up(&mut self, step: usize) {
        self.selected_episode = self.selected_episode.saturating_sub(step);
        self.update_scroll();
    }

    fn update_scroll(&mut self) {
        let visible = 12usize;
        if self.selected_episode < self.episode_scroll {
            self.episode_scroll = self.selected_episode;
        } else if self.selected_episode >= self.episode_scroll + visible {
            self.episode_scroll = self.selected_episode.saturating_sub(visible - 1);
        }
    }

    // ------------------------------------------------------------------
    // Comments state
    // ------------------------------------------------------------------

    /// Mark a comments page as in flight. Page 1 replaces the current list;
    /// later pages append.
    pub fn begin_comments_load(&mut self, aid: i64, page: i32) {
        self.comments_aid = Some(aid);
        self.comments_error = None;
        if page <= 1 {
            self.comment_page = 0;
            self.comments.clear();
            self.comment_scroll = 0;
            self.has_more_comments = false;
            self.comments_loading = true;
            self.loading_more_comments = false;
        } else {
            self.comments_loading = false;
            self.loading_more_comments = true;
        }
    }

    /// Apply a loaded comments page, de-duplicating by `rpid` on later pages.
    pub fn apply_comments(
        &mut self,
        aid: i64,
        page: i32,
        comments: Vec<CommentItem>,
        has_more: bool,
    ) {
        self.comments_aid = Some(aid);
        self.comments_loading = false;
        self.loading_more_comments = false;
        self.comments_error = None;
        self.comment_page = page;
        self.has_more_comments = has_more;
        if page <= 1 {
            self.comments = comments;
            self.comment_scroll = 0;
        } else {
            let existing: HashSet<i64> = self.comments.iter().map(|c| c.rpid).collect();
            self.comments
                .extend(comments.into_iter().filter(|c| !existing.contains(&c.rpid)));
        }
    }

    /// Record a comments failure and make sure no loading state stays stuck.
    pub fn set_comments_error(&mut self, msg: String) {
        self.comments_loading = false;
        self.loading_more_comments = false;
        self.comments_error = Some(msg);
    }

    fn maybe_request_more_comments(&self) -> Option<AppAction> {
        if self.comments.is_empty()
            || !self.has_more_comments
            || self.comments_loading
            || self.loading_more_comments
            || self.comment_page < 1
        {
            return None;
        }
        let near_bottom =
            self.comment_scroll + COMMENT_VISIBLE_ROWS >= self.comments.len().saturating_sub(2);
        if !near_bottom {
            return None;
        }
        let aid = self.comments_aid.filter(|aid| *aid > 0)?;
        Some(AppAction::LoadBangumiComments {
            aid,
            page: self.comment_page + 1,
        })
    }

    fn comments_reload_for_selection(&self) -> Option<AppAction> {
        let aid = self.flat_episodes.get(self.selected_episode)?.episode.aid;
        if aid <= 0 || self.comments_aid == Some(aid) {
            return None;
        }
        Some(AppAction::LoadBangumiComments { aid, page: 1 })
    }

    // ------------------------------------------------------------------
    // Seasons state
    // ------------------------------------------------------------------

    fn has_season_switch(&self) -> bool {
        self.season
            .as_ref()
            .is_some_and(|season| season.seasons.len() > 1)
    }

    fn selected_season_action(&self) -> Option<AppAction> {
        let related = self.season.as_ref()?.seasons.get(self.selected_season)?;
        if related.season_id == self.season_id || related.season_id <= 0 {
            return None;
        }
        Some(AppAction::OpenBangumiDetail(related.season_id))
    }

    fn focus_panes(&self) -> Vec<BangumiDetailFocus> {
        let mut panes = vec![BangumiDetailFocus::Info];
        if self.has_season_switch() {
            panes.push(BangumiDetailFocus::Seasons);
        }
        panes.push(BangumiDetailFocus::Episodes);
        panes.push(BangumiDetailFocus::Comments);
        panes
    }

    fn cycle_focus(&mut self, forward: bool) {
        let panes = self.focus_panes();
        let Some(position) = panes.iter().position(|pane| *pane == self.focus) else {
            self.focus = BangumiDetailFocus::Episodes;
            return;
        };
        let next = if forward {
            (position + 1) % panes.len()
        } else {
            (position + panes.len() - 1) % panes.len()
        };
        self.focus = panes[next];
    }

    fn move_season_cursor(&mut self, forward: bool) {
        let Some(season) = self.season.as_ref() else {
            return;
        };
        let count = season.seasons.len();
        if count == 0 {
            return;
        }
        if forward {
            self.selected_season = (self.selected_season + 1).min(count - 1);
        } else {
            self.selected_season = self.selected_season.saturating_sub(1);
        }
    }

    // ------------------------------------------------------------------
    // Layout
    // ------------------------------------------------------------------

    fn compute_layout(&self, area: Rect) -> BangumiDetailLayout {
        let footer_height = 2.min(area.height);
        let footer = Rect {
            x: area.x,
            y: area.y + area.height - footer_height,
            width: area.width,
            height: footer_height,
        };
        let body = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.saturating_sub(footer_height),
        };
        let info_height = (body.height / 3)
            .clamp(7, 12)
            .min(body.height.saturating_sub(6).max(4));
        let info = Rect {
            height: info_height,
            ..body
        };
        let mut y = body.y + info_height;
        let mut seasons = None;
        if self.has_season_switch() && body.y + body.height >= y + 3 + 4 {
            seasons = Some(Rect {
                x: body.x,
                y,
                width: body.width,
                height: 3,
            });
            y += 3;
        }
        let main = Rect {
            x: body.x,
            y,
            width: body.width,
            height: (body.y + body.height).saturating_sub(y),
        };
        BangumiDetailLayout {
            info,
            seasons,
            main,
            footer,
        }
    }

    fn split_main(&self, main: Rect) -> (Rect, Rect) {
        if main.width < 70 {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main);
            (chunks[0], chunks[1])
        } else {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(main);
            (chunks[0], chunks[1])
        }
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------

    fn render_info(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(
                Style::default().fg(if self.focus == BangumiDetailFocus::Info {
                    theme.border_focused
                } else {
                    theme.border_subtle
                }),
            )
            .title(Span::styled(
                " 📺 番剧信息 ",
                Style::default().fg(theme.bilibili_pink),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let lines = {
            let Some(season) = self.season.as_ref() else {
                return;
            };
            let width = inner.width.saturating_sub(1).max(8) as usize;
            let mut lines: Vec<Line<'static>> = Vec::new();
            lines.push(Line::from(Span::styled(
                season.title.clone(),
                Style::default()
                    .fg(theme.fg_primary)
                    .add_modifier(Modifier::BOLD),
            )));

            let badges = status_badge_spans(season, theme);
            if !badges.is_empty() {
                lines.push(Line::from(badges));
            }

            if let Some(error) = self.error_message.as_deref() {
                lines.push(Line::from(Span::styled(
                    error.to_string(),
                    Style::default().fg(theme.error),
                )));
            }

            lines.push(Line::from(meta_spans(season, theme)));

            if !season.styles.is_empty() {
                lines.push(Line::from(Span::styled(
                    season.styles.join(" / "),
                    Style::default().fg(theme.bilibili_cyan),
                )));
            }

            lines.push(Line::from(stat_spans(season, theme)));

            if let Some(staff) = season.staff.as_deref() {
                push_labeled_text(&mut lines, "制作", staff, theme, width);
            }
            if let Some(actors) = season.actors.as_deref() {
                push_labeled_text(&mut lines, "声优", actors, theme, width);
            }
            if let Some(synopsis) = season.evaluate.as_deref() {
                push_labeled_text(&mut lines, "简介", synopsis, theme, width);
            }
            lines
        };

        self.info_line_count = lines.len();
        let scroll = if self.focus == BangumiDetailFocus::Info {
            let max_scroll = lines.len().saturating_sub(inner.height as usize) as u16;
            let clamped = self.info_scroll.min(max_scroll);
            self.info_scroll = clamped;
            clamped
        } else {
            0
        };
        frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), inner);
    }

    fn render_seasons(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focused = self.focus == BangumiDetailFocus::Seasons;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if focused {
                theme.border_focused
            } else {
                theme.border_subtle
            }))
            .title(Span::styled(
                " 📚 系列 ",
                Style::default().fg(theme.bilibili_pink),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let Some(season) = self.season.as_ref() else {
            return;
        };

        let mut spans: Vec<Span<'static>> = Vec::new();
        for (index, related) in season.seasons.iter().enumerate() {
            if index > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(theme.fg_muted)));
            }
            let label = related
                .season_title
                .as_deref()
                .filter(|title| !title.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("ss{}", related.season_id));
            let label = truncate_str(&label, 16);
            let is_current = related.season_id == self.season_id;
            let is_selected = index == self.selected_season;
            let style = if is_selected && focused {
                Style::default()
                    .fg(theme.bilibili_pink)
                    .bg(theme.bg_highlight)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg_secondary)
            };
            spans.push(Span::styled(
                format!("{}{}", if is_current { "●" } else { "○" }, label),
                style,
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), inner);
    }

    fn render_episodes(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focused = self.focus == BangumiDetailFocus::Episodes;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if focused {
                theme.border_focused
            } else {
                theme.border_subtle
            }))
            .title(Span::styled(
                " 📋 选集 ",
                Style::default().fg(theme.bilibili_pink),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.flat_episodes.is_empty() {
            frame.render_widget(
                Paragraph::new("暂无剧集信息")
                    .style(Style::default().fg(theme.fg_muted))
                    .alignment(Alignment::Center),
                inner,
            );
            return;
        }

        let mut items: Vec<ListItem> = Vec::new();
        let mut last_section = "";

        for (global_idx, fe) in self.flat_episodes.iter().enumerate() {
            if fe.section_title.as_str() != last_section {
                last_section = fe.section_title.as_str();
                items.push(
                    ListItem::new(format!("▸ {}", last_section)).style(
                        Style::default()
                            .fg(theme.fg_secondary)
                            .add_modifier(Modifier::BOLD),
                    ),
                );
            }

            let is_selected = global_idx == self.selected_episode;
            let ep = &fe.episode;
            let badge = ep
                .badge_text()
                .map(|b| format!(" [{}]", b))
                .unwrap_or_default();
            let duration = ep
                .duration
                .filter(|secs| *secs > 0)
                .map(|secs| format!(" ({})", format_duration(secs)))
                .unwrap_or_default();
            let content = format!("  {}{}{}", ep.display_title(), badge, duration);

            let style = if is_selected {
                Style::default()
                    .fg(theme.bilibili_pink)
                    .bg(theme.bg_highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg_primary)
            };

            items.push(ListItem::new(content).style(style));
        }

        let list = List::new(items)
            .style(Style::default().fg(theme.fg_primary))
            .scroll_padding(2);

        let mut state = ListState::default();
        state.select(Some(self.list_index_for_episode(self.selected_episode)));

        frame.render_stateful_widget(list, inner, &mut state);
    }

    fn render_comments(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let focused = self.focus == BangumiDetailFocus::Comments;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if focused {
                theme.border_focused
            } else {
                theme.border_subtle
            }))
            .title(Span::styled(
                " 💬 评论 ",
                Style::default().fg(theme.bilibili_pink),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        if self.comments_loading && self.comments.is_empty() {
            frame.render_widget(
                Paragraph::new("加载评论中...")
                    .style(Style::default().fg(theme.fg_muted))
                    .alignment(Alignment::Center),
                inner,
            );
            return;
        }

        if let Some(error) = self.comments_error.as_deref()
            && self.comments.is_empty()
        {
            frame.render_widget(
                Paragraph::new(error)
                    .style(Style::default().fg(theme.error))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true }),
                inner,
            );
            return;
        }

        if self.comments.is_empty() {
            let message = if self.comments_aid.is_some() {
                "暂无评论"
            } else {
                "暂无评论（剧集未关联视频）"
            };
            frame.render_widget(
                Paragraph::new(message)
                    .style(Style::default().fg(theme.fg_muted))
                    .alignment(Alignment::Center),
                inner,
            );
            return;
        }

        let visible = (inner.height as usize / 3).max(1);
        let message_width = inner.width.saturating_sub(2) as usize;
        let mut items: Vec<ListItem> = Vec::new();
        for (offset, comment) in self
            .comments
            .iter()
            .skip(self.comment_scroll)
            .take(visible)
            .enumerate()
        {
            let is_cursor = offset == 0 && focused;
            let author_style = if is_cursor {
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg_accent)
            };
            let header = Line::from(vec![
                Span::styled(comment.author_name().to_string(), author_style),
                Span::styled(
                    format!("  {}", comment.format_time()),
                    Style::default().fg(theme.fg_muted),
                ),
            ]);
            let body = Line::from(Span::styled(
                truncate_str(comment.message(), message_width),
                Style::default().fg(theme.fg_primary),
            ));
            let meta = Line::from(Span::styled(
                format!(
                    "👍 {}  💬 {} 回复",
                    comment.format_like(),
                    comment.reply_count()
                ),
                Style::default().fg(theme.fg_secondary),
            ));
            let mut item = ListItem::new(vec![header, body, meta]);
            if is_cursor {
                item = item.style(Style::default().bg(theme.bg_highlight));
            }
            items.push(item);
        }

        if self.loading_more_comments {
            items.push(ListItem::new(Line::from(Span::styled(
                "加载更多评论中...",
                Style::default()
                    .fg(theme.fg_muted)
                    .add_modifier(Modifier::ITALIC),
            ))));
        } else if let Some(error) = self.comments_error.as_deref() {
            items.push(ListItem::new(Line::from(Span::styled(
                error.to_string(),
                Style::default().fg(theme.error),
            ))));
        }

        frame.render_widget(List::new(items), inner);
    }

    /// Map episode index to list index (accounting for section headers)
    fn list_index_for_episode(&self, ep_idx: usize) -> usize {
        let mut list_idx = 0;
        let mut last_section = "";
        for (current_ep, fe) in self.flat_episodes.iter().enumerate() {
            if fe.section_title.as_str() != last_section {
                last_section = fe.section_title.as_str();
                list_idx += 1;
            }
            if current_ep == ep_idx {
                return list_idx;
            }
            list_idx += 1;
        }
        list_idx
    }

    fn scroll_comments_down(&mut self, step: usize) -> Option<AppAction> {
        if self.comments.is_empty() {
            return None;
        }
        self.comment_scroll = (self.comment_scroll + step).min(self.comments.len() - 1);
        self.maybe_request_more_comments()
    }

    fn scroll_comments_up(&mut self, step: usize) {
        self.comment_scroll = self.comment_scroll.saturating_sub(step);
    }

    fn toggle_episode_comments_focus(&mut self, forward: bool) {
        match (self.focus, forward) {
            (BangumiDetailFocus::Episodes, true) => self.focus = BangumiDetailFocus::Comments,
            (BangumiDetailFocus::Comments, false) => self.focus = BangumiDetailFocus::Episodes,
            _ => {}
        }
    }
}

impl Component for BangumiDetailPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        if self.loading {
            let spinner = Paragraph::new("加载中...")
                .style(Style::default().fg(theme.fg_muted))
                .alignment(Alignment::Center);
            frame.render_widget(spinner, area);
            return;
        }

        if self.season.is_none() {
            let message = self.error_message.as_deref().unwrap_or("番剧信息不可用");
            let error = Paragraph::new(message)
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            frame.render_widget(error, area);
            return;
        }

        let layout = self.compute_layout(area);
        self.render_info(frame, layout.info, theme);
        if let Some(seasons) = layout.seasons {
            self.render_seasons(frame, seasons, theme);
        }
        let (episodes_area, comments_area) = self.split_main(layout.main);
        self.render_episodes(frame, episodes_area, theme);
        self.render_comments(frame, comments_area, theme);

        let help = Paragraph::new(shortcut_footer(
            theme,
            [
                (
                    format!("{}/{}", keys.nav_up, keys.nav_down),
                    "滚动".into(),
                    theme.fg_accent,
                ),
                (
                    format!("{}/{}", keys.section_prev, keys.section_next),
                    "切换焦点".into(),
                    theme.bilibili_cyan,
                ),
                (keys.confirm.clone(), "播放/选集".into(), theme.success),
                (keys.back.clone(), "返回".into(), theme.info),
            ],
        ))
        .alignment(Alignment::Center);
        frame.render_widget(help, layout.footer);
    }

    fn handle_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        // Global keybindings stay responsive while loading or after failures.
        if keys.matches_quit(key) {
            return Some(AppAction::Quit);
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
        if keys.matches_next_theme(key) {
            return Some(AppAction::NextTheme);
        }
        if keys.matches_open_settings(key) {
            return Some(AppAction::SwitchToSettings);
        }

        if self.loading || self.season.is_none() {
            return Some(AppAction::None);
        }

        if keys.matches_section_next(key) {
            self.cycle_focus(true);
            return Some(AppAction::None);
        }
        if keys.matches_section_prev(key) {
            self.cycle_focus(false);
            return Some(AppAction::None);
        }

        if keys.matches_left(key) {
            match self.focus {
                BangumiDetailFocus::Seasons => self.move_season_cursor(false),
                BangumiDetailFocus::Episodes | BangumiDetailFocus::Comments => {
                    self.toggle_episode_comments_focus(false)
                }
                BangumiDetailFocus::Info => {}
            }
            return Some(AppAction::None);
        }
        if keys.matches_right(key) {
            match self.focus {
                BangumiDetailFocus::Seasons => self.move_season_cursor(true),
                BangumiDetailFocus::Episodes | BangumiDetailFocus::Comments => {
                    self.toggle_episode_comments_focus(true)
                }
                BangumiDetailFocus::Info => {}
            }
            return Some(AppAction::None);
        }

        if keys.matches_down(key) {
            return Some(match self.focus {
                BangumiDetailFocus::Info => {
                    let max_scroll = self.info_line_count.saturating_sub(1);
                    if (self.info_scroll as usize) < max_scroll {
                        self.info_scroll = self.info_scroll.saturating_add(1);
                    }
                    AppAction::None
                }
                BangumiDetailFocus::Seasons => AppAction::None,
                BangumiDetailFocus::Episodes => {
                    self.move_down(1);
                    self.comments_reload_for_selection()
                        .unwrap_or(AppAction::None)
                }
                BangumiDetailFocus::Comments => {
                    self.scroll_comments_down(1).unwrap_or(AppAction::None)
                }
            });
        }
        if keys.matches_up(key) {
            return Some(match self.focus {
                BangumiDetailFocus::Info => {
                    self.info_scroll = self.info_scroll.saturating_sub(1);
                    AppAction::None
                }
                BangumiDetailFocus::Seasons => AppAction::None,
                BangumiDetailFocus::Episodes => {
                    self.move_up(1);
                    self.comments_reload_for_selection()
                        .unwrap_or(AppAction::None)
                }
                BangumiDetailFocus::Comments => {
                    self.scroll_comments_up(1);
                    AppAction::None
                }
            });
        }

        if keys.matches_page_down(key) {
            return Some(match self.focus {
                BangumiDetailFocus::Info => {
                    let max_scroll = self.info_line_count.saturating_sub(1);
                    self.info_scroll = self.info_scroll.saturating_add(5).min(max_scroll as u16);
                    AppAction::None
                }
                BangumiDetailFocus::Seasons => AppAction::None,
                BangumiDetailFocus::Episodes => {
                    self.move_down(5);
                    self.comments_reload_for_selection()
                        .unwrap_or(AppAction::None)
                }
                BangumiDetailFocus::Comments => self
                    .scroll_comments_down(COMMENT_VISIBLE_ROWS)
                    .unwrap_or(AppAction::None),
            });
        }
        if keys.matches_page_up(key) {
            return Some(match self.focus {
                BangumiDetailFocus::Info => {
                    self.info_scroll = self.info_scroll.saturating_sub(5);
                    AppAction::None
                }
                BangumiDetailFocus::Seasons => AppAction::None,
                BangumiDetailFocus::Episodes => {
                    self.move_up(5);
                    self.comments_reload_for_selection()
                        .unwrap_or(AppAction::None)
                }
                BangumiDetailFocus::Comments => {
                    self.scroll_comments_up(COMMENT_VISIBLE_ROWS);
                    AppAction::None
                }
            });
        }

        if keys.matches_play(key) || keys.matches_confirm(key) {
            return match self.focus {
                BangumiDetailFocus::Episodes => self.selected_action(),
                BangumiDetailFocus::Seasons => self.selected_season_action(),
                BangumiDetailFocus::Info | BangumiDetailFocus::Comments => None,
            };
        }

        Some(AppAction::None)
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        let MouseEvent {
            kind, column, row, ..
        } = event;
        let layout = self.compute_layout(area);
        let (episodes_area, comments_area) = self.split_main(layout.main);
        if !episodes_area.contains(Position::new(column, row)) {
            if comments_area.contains(Position::new(column, row)) {
                self.focus = BangumiDetailFocus::Comments;
            }
            return None;
        }
        if row <= episodes_area.y || row >= episodes_area.y + episodes_area.height.saturating_sub(1)
        {
            return None;
        }

        let rel_row = (row - episodes_area.y - 1) as usize + self.episode_scroll;
        let mut list_idx = 0;
        let mut last_section = "";
        let mut target_ep_idx = None;

        for (ep_idx, fe) in self.flat_episodes.iter().enumerate() {
            if fe.section_title.as_str() != last_section {
                last_section = fe.section_title.as_str();
                list_idx += 1;
            }
            if list_idx == rel_row {
                target_ep_idx = Some(ep_idx);
                break;
            }
            list_idx += 1;
        }

        let idx = target_ep_idx?;
        self.focus = BangumiDetailFocus::Episodes;
        self.selected_episode = idx;

        if kind == MouseEventKind::Down(MouseButton::Left) {
            let now = Instant::now();
            let is_double = self
                .last_click_time
                .and_then(|t| now.duration_since(t).as_millis().le(&500).then_some(()))
                .is_some()
                && self.last_click_index == Some(idx);

            self.last_click_time = Some(now);
            self.last_click_index = Some(idx);

            if is_double {
                return self.selected_action();
            }
        }

        self.comments_reload_for_selection()
    }
}

fn status_badge_spans(season: &SeasonResult, theme: &Theme) -> Vec<Span<'static>> {
    let mut labels: Vec<(String, Color)> = Vec::new();
    if season.is_followed() {
        labels.push(("已追番".to_string(), theme.success));
    }
    if season.is_area_limited() {
        labels.push(("地区限制".to_string(), theme.error));
    }

    let badge = season.badge.as_deref().filter(|badge| !badge.is_empty());
    let badge_mentions_vip = badge.is_some_and(|badge| badge.contains("会员"));
    if let Some(badge) = badge {
        labels.push((badge.to_string(), theme.bilibili_pink));
    }
    let vip_promotion = season
        .payment
        .as_ref()
        .and_then(|payment| payment.vip_promotion.as_deref())
        .unwrap_or("");
    if !badge_mentions_vip && (vip_promotion.contains("会员") || season.status == Some(13)) {
        labels.push(("大会员".to_string(), theme.bilibili_pink));
    }
    let paid = season
        .payment
        .as_ref()
        .and_then(|payment| payment.price.as_deref())
        .is_some_and(|price| {
            let price = price.trim();
            !price.is_empty() && price != "0" && price != "0.0"
        });
    if paid && !badge.is_some_and(|badge| badge.contains("付费")) {
        labels.push(("付费".to_string(), theme.warning));
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (label, color) in labels {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(
            format!("[{}]", label),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    spans
}

fn meta_spans(season: &SeasonResult, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let separator = |spans: &mut Vec<Span<'static>>| {
        if !spans.is_empty() {
            spans.push(Span::styled(" · ", Style::default().fg(theme.fg_muted)));
        }
    };

    if let Some(rating) = season.rating.as_ref() {
        separator(&mut spans);
        spans.push(Span::styled(
            format!(
                "⭐ {:.1}分 ({}人评分)",
                rating.score,
                format_count(rating.count)
            ),
            Style::default().fg(theme.warning),
        ));
    }
    if let Some(type_name) = season.resolved_type_name() {
        separator(&mut spans);
        spans.push(Span::styled(
            type_name.to_string(),
            Style::default().fg(theme.fg_secondary),
        ));
    }
    if !season.areas.is_empty() {
        separator(&mut spans);
        let areas = season
            .areas
            .iter()
            .map(|area| area.name.as_str())
            .collect::<Vec<_>>()
            .join("/");
        spans.push(Span::styled(areas, Style::default().fg(theme.fg_secondary)));
    }
    if let Some(index_show) = season.display_index_show() {
        separator(&mut spans);
        spans.push(Span::styled(
            index_show.to_string(),
            Style::default().fg(theme.fg_secondary),
        ));
    }
    if let Some(finished) = season.is_finished() {
        separator(&mut spans);
        spans.push(Span::styled(
            if finished { "已完结" } else { "连载中" },
            Style::default().fg(theme.fg_secondary),
        ));
    }
    spans
}

fn stat_spans(season: &SeasonResult, theme: &Theme) -> Vec<Span<'static>> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(stat) = season.stat.as_ref() {
        if let Some(views) = stat.views {
            parts.push(format!("播放 {}", format_count(views)));
        }
        if let Some(danmakus) = stat.danmakus {
            parts.push(format!("弹幕 {}", format_count(danmakus)));
        }
        if let Some(favorites) = stat.display_favorites() {
            parts.push(format!("追番 {}", format_count(favorites)));
        }
        if let Some(coins) = stat.coins {
            parts.push(format!("投币 {}", format_count(coins)));
        }
        if let Some(share) = stat.share {
            parts.push(format!("分享 {}", format_count(share)));
        }
    }
    let text = parts.join(" · ");
    if text.is_empty() {
        Vec::new()
    } else {
        vec![Span::styled(text, Style::default().fg(theme.fg_secondary))]
    }
}

fn push_labeled_text(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    text: &str,
    theme: &Theme,
    width: usize,
) {
    let normalized = text.trim().replace('\n', " / ");
    if normalized.is_empty() {
        return;
    }
    let indent = label.chars().count() + 1;
    let wrap_width = width.saturating_sub(indent).max(8);
    for (index, chunk) in wrap_text(&normalized, wrap_width).iter().enumerate() {
        if index == 0 {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} ", label),
                    Style::default()
                        .fg(theme.fg_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(chunk.clone(), Style::default().fg(theme.fg_primary)),
            ]));
        } else {
            lines.push(Line::from(Span::styled(
                format!("{}{}", " ".repeat(indent), chunk),
                Style::default().fg(theme.fg_primary),
            )));
        }
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut wrapped = Vec::new();
    for raw_line in text.split('\n') {
        let chars: Vec<char> = raw_line.chars().collect();
        if chars.is_empty() {
            wrapped.push(String::new());
            continue;
        }
        for chunk in chars.chunks(width) {
            wrapped.push(chunk.iter().collect());
        }
    }
    wrapped
}

fn truncate_str(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars()
        .take(max_chars.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}

fn format_count(value: i64) -> String {
    if value >= 100_000_000 {
        format!("{:.1}亿", value as f64 / 100_000_000.0)
    } else if value >= 10_000 {
        format!("{:.1}万", value as f64 / 10_000.0)
    } else {
        value.to_string()
    }
}

fn format_duration(seconds: i64) -> String {
    let minutes = seconds / 60;
    let remainder = seconds % 60;
    format!("{:02}:{:02}", minutes, remainder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn base_season_json() -> serde_json::Value {
        serde_json::json!({
            "title": "测试番剧",
            "season_id": 7,
            "cover": "",
            "square_cover": "",
            "evaluate": "第一行简介\n第二行简介",
            "link": null,
            "episodes": [
                {"aid": 111, "cid": 11, "id": 101, "title": "1", "badge": "会员", "duration": 1440},
                {"aid": 222, "cid": 22, "id": 202, "title": "2"}
            ],
            "section": null,
            "rating": {"score": 9.1, "count": 70000},
            "stat": {"views": 12345678, "danmakus": 10000, "favorites": 50000, "coins": 100, "share": 5},
            "badge": null,
            "is_finish": 1,
            "index_show": "全12话",
            "areas": [{"id": 1, "name": "日本"}],
            "styles": ["漫画改", "奇幻"],
            "staff": "导演：某某\n脚本：某某",
            "actors": "声优A / 声优B",
            "user_status": {"follow": 1},
            "rights": {"area_limit": 328, "can_watch": 1},
            "seasons": [
                {"season_id": 7, "season_title": "第一季"},
                {"season_id": 8, "season_title": "第二季"}
            ],
            "payment": {"price": "0", "vip_promotion": ""},
            "season_type": 1,
            "type_name": "番剧"
        })
    }

    fn season_from(value: serde_json::Value) -> SeasonResult {
        serde_json::from_value(value).unwrap()
    }

    fn set_json_field(value: &mut serde_json::Value, key: &str, new_value: serde_json::Value) {
        if let Some(object) = value.as_object_mut() {
            object.insert(key.to_string(), new_value);
        }
    }

    fn render_content(page: &mut BangumiDetailPage, width: u16, height: u16) -> String {
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
            .filter(|c| !c.is_whitespace())
            .collect()
    }

    fn comment(rpid: i64) -> CommentItem {
        serde_json::from_value(serde_json::json!({
            "rpid": rpid,
            "oid": 111,
            "mid": 1,
            "parent": 0,
            "ctime": 1_700_000_000,
            "like": 5,
            "rcount": 2,
            "member": {"uname": format!("user{}", rpid)},
            "content": {"message": format!("评论内容 {}", rpid)}
        }))
        .unwrap()
    }

    #[test]
    fn history_episode_is_preselected_for_auto_play() {
        let season: SeasonResult = serde_json::from_value(serde_json::json!({
            "title": "season",
            "season_id": 7,
            "cover": "",
            "square_cover": "",
            "evaluate": null,
            "link": null,
            "episodes": [
                {"aid": 1, "cid": 11, "id": 101, "title": "1"},
                {"aid": 2, "cid": 22, "id": 202, "title": "2"}
            ],
            "section": null,
            "rating": null,
            "stat": null,
            "badge": null,
            "is_finish": null,
            "index_show": null
        }))
        .unwrap();
        let mut page = BangumiDetailPage::new_for_episode(7, 202, true);
        page.set_season(season);

        assert_eq!(page.selected_episode, 1);
        assert!(page.auto_play_pending);
        assert!(matches!(
            page.play_action(),
            Some(AppAction::PlayBangumiEpisode { ep_id: 202, .. })
        ));
    }

    #[test]
    fn metadata_panel_renders_staff_actors_areas_rating_stats_and_follow_badge() {
        let mut page = BangumiDetailPage::new(7);
        page.set_season(season_from(base_season_json()));

        let content = render_content(&mut page, 140, 46);

        assert!(content.contains("测试番剧"));
        assert!(content.contains("制作"));
        assert!(content.contains("导演：某某"));
        assert!(content.contains("声优"));
        assert!(content.contains("声优A/声优B"));
        assert!(content.contains("日本"));
        assert!(content.contains("7.0万人评分"));
        assert!(content.contains("播放1234.6万"));
        assert!(content.contains("追番5.0万"));
        assert!(content.contains("已追番"));
        assert!(content.contains("已完结"));
        assert!(!content.contains("地区限制"));
    }

    #[test]
    fn area_limit_badge_only_when_actually_limited() {
        let mut page = BangumiDetailPage::new(7);
        page.set_season(season_from(base_season_json()));
        let content = render_content(&mut page, 140, 46);
        assert!(!content.contains("地区限制"));

        let mut json = base_season_json();
        set_json_field(
            &mut json,
            "user_status",
            serde_json::json!({"follow": 0, "area_limit": 1}),
        );
        let mut limited = BangumiDetailPage::new(7);
        limited.set_season(season_from(json));
        let limited_content = render_content(&mut limited, 140, 46);
        assert!(limited_content.contains("地区限制"));
        assert!(!limited_content.contains("已追番"));
    }

    #[test]
    fn synopsis_is_scrollable_through_info_pane() {
        let mut json = base_season_json();
        set_json_field(
            &mut json,
            "evaluate",
            serde_json::json!(format!("简介开头{}", "内容".repeat(600))),
        );
        let mut page = BangumiDetailPage::new(7);
        page.set_season(season_from(json));

        let content = render_content(&mut page, 140, 46);
        assert!(content.contains("简介"));

        page.focus = BangumiDetailFocus::Info;
        for _ in 0..20 {
            page.handle_input(KeyCode::Down, &Keybindings::default());
        }
        let scrolled = render_content(&mut page, 140, 46);
        assert!(page.info_scroll > 0);
        assert!(scrolled.contains("简介开头"));
    }

    #[test]
    fn seasons_confirm_opens_selected_season() {
        let mut page = BangumiDetailPage::new(7);
        page.set_season(season_from(base_season_json()));
        page.focus = BangumiDetailFocus::Seasons;

        let keys = Keybindings::default();
        page.handle_input(KeyCode::Right, &keys);
        assert_eq!(page.selected_season, 1);

        let action = page.handle_input(KeyCode::Enter, &keys);
        assert!(matches!(action, Some(AppAction::OpenBangumiDetail(8))));
    }

    #[test]
    fn focused_season_confirm_on_current_season_is_noop() {
        let mut page = BangumiDetailPage::new(7);
        page.set_season(season_from(base_season_json()));
        page.focus = BangumiDetailFocus::Seasons;

        let action = page.handle_input(KeyCode::Enter, &Keybindings::default());
        assert!(action.is_none());
    }

    #[test]
    fn comments_pagination_dedupes_and_requests_next_page() {
        let mut page = BangumiDetailPage::new(7);
        page.set_season(season_from(base_season_json()));

        page.begin_comments_load(111, 1);
        assert!(page.comments_loading);
        page.apply_comments(111, 1, vec![comment(1), comment(2)], true);
        assert!(!page.comments_loading);
        assert_eq!(page.comment_page, 1);
        assert_eq!(page.comments.len(), 2);
        assert!(page.has_more_comments);

        page.begin_comments_load(111, 2);
        assert!(page.loading_more_comments);
        page.apply_comments(111, 2, vec![comment(2), comment(3)], true);
        assert!(!page.loading_more_comments);
        assert_eq!(page.comment_page, 2);
        assert_eq!(page.comments.len(), 3);
        assert_eq!(page.comments[2].rpid, 3);

        page.focus = BangumiDetailFocus::Comments;
        let keys = Keybindings::default();
        let action = page.handle_input(KeyCode::PageDown, &keys);
        assert!(matches!(
            action,
            Some(AppAction::LoadBangumiComments { aid: 111, page: 3 })
        ));
    }

    #[test]
    fn comments_error_clears_loading_flags_and_records_message() {
        let mut page = BangumiDetailPage::new(7);
        page.set_season(season_from(base_season_json()));

        page.begin_comments_load(111, 2);
        assert!(page.loading_more_comments);
        page.set_comments_error("加载评论失败: boom".to_string());

        assert!(!page.comments_loading);
        assert!(!page.loading_more_comments);
        assert_eq!(page.comments_error.as_deref(), Some("加载评论失败: boom"));

        page.set_comments_error("again".to_string());
        assert!(!page.comments_loading);
        assert!(!page.loading_more_comments);
    }

    #[test]
    fn selecting_different_aid_requests_first_comment_page() {
        let mut page = BangumiDetailPage::new(7);
        page.set_season(season_from(base_season_json()));
        assert_eq!(page.flat_episodes.len(), 2);

        page.focus = BangumiDetailFocus::Episodes;
        page.comments_aid = Some(222);
        let action = page.handle_input(KeyCode::Up, &Keybindings::default());
        assert!(matches!(
            action,
            Some(AppAction::LoadBangumiComments { aid: 111, page: 1 })
        ));
    }

    #[test]
    fn failed_next_page_keeps_comment_page_for_retry() {
        let mut page = BangumiDetailPage::new(7);
        page.set_season(season_from(base_season_json()));

        page.begin_comments_load(111, 1);
        page.apply_comments(111, 1, vec![comment(1)], true);
        assert_eq!(page.comment_page, 1);

        page.begin_comments_load(111, 2);
        assert!(page.loading_more_comments);
        page.set_comments_error("加载评论失败: boom".to_string());

        assert_eq!(page.comment_page, 1);
        assert!(!page.loading_more_comments);

        page.focus = BangumiDetailFocus::Comments;
        let action = page.handle_input(KeyCode::PageDown, &Keybindings::default());
        assert!(matches!(
            action,
            Some(AppAction::LoadBangumiComments { aid: 111, page: 2 })
        ));
    }

    #[test]
    fn mouse_click_in_comments_pane_does_not_select_episode() {
        use ratatui::crossterm::event::KeyModifiers;

        let mut page = BangumiDetailPage::new(7);
        page.set_season(season_from(base_season_json()));
        let area = Rect::new(0, 0, 140, 46);

        // Row 17 maps to episode index 1 inside the left episodes pane; the same
        // row on the right side belongs to the comments pane.
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 100,
            row: 17,
            modifiers: KeyModifiers::NONE,
        };
        let action = page.handle_mouse(event, area);

        assert!(action.is_none());
        assert_eq!(page.selected_episode, 0);
        assert_eq!(page.focus, BangumiDetailFocus::Comments);
    }
}
