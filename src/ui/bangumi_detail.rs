//! Bangumi detail page showing season info and episode list

use super::{Component, Theme, shortcut_footer};
use crate::api::bangumi::{BangumiEpisode, SeasonQuery, SeasonResult};
use crate::api::client::ApiClient;
use crate::application::AppAction;
use crate::storage::Keybindings;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind},
    prelude::*,
    widgets::*,
};
use std::time::Instant;

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
}

#[derive(Clone)]
struct FlatEpisode {
    section_title: String,
    _ep_index: usize,
    episode: BangumiEpisode,
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
        self.season = Some(season);
        self.loading = false;
        self.error_message = None;
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

    fn move_down(&mut self) {
        if self.selected_episode + 1 < self.flat_episodes.len() {
            self.selected_episode += 1;
            self.update_scroll();
        }
    }

    fn move_up(&mut self) {
        if self.selected_episode > 0 {
            self.selected_episode -= 1;
            self.update_scroll();
        }
    }

    fn update_scroll(&mut self) {
        let visible = 12usize;
        if self.selected_episode < self.episode_scroll {
            self.episode_scroll = self.selected_episode;
        } else if self.selected_episode >= self.episode_scroll + visible {
            self.episode_scroll = self.selected_episode.saturating_sub(visible - 1);
        }
    }

    fn render_info(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " 📺 番剧信息 ",
                Style::default().fg(theme.bilibili_pink),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(ref season) = self.season {
            let mut constraints = vec![
                Constraint::Length(1), // Title
                Constraint::Length(1), // Score / status
            ];
            if season.evaluate.is_some() {
                constraints.push(Constraint::Min(2));
            }
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(inner);

            // Title
            let title = Paragraph::new(season.title.clone()).style(
                Style::default()
                    .fg(theme.fg_primary)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(title, chunks[0]);

            // Score & status
            let mut stats_spans = Vec::new();
            if let Some(ref rating) = season.rating {
                stats_spans.push(Span::styled(
                    format!("⭐ {:.1}分", rating.score),
                    Style::default().fg(theme.warning),
                ));
                stats_spans.push(Span::styled(" · ", Style::default().fg(theme.fg_muted)));
            }
            if let Some(ref index_show) = season.index_show {
                stats_spans.push(Span::styled(
                    index_show,
                    Style::default().fg(theme.fg_secondary),
                ));
            }
            if let Some(is_finish) = season.is_finish {
                stats_spans.push(Span::styled(
                    if is_finish == 1 {
                        " · 已完结"
                    } else {
                        " · 连载中"
                    },
                    Style::default().fg(theme.fg_secondary),
                ));
            }
            if let Some(ref badge) = season.badge
                && !badge.is_empty()
            {
                stats_spans.push(Span::styled(
                    format!(" · {}", badge),
                    Style::default().fg(theme.bilibili_pink),
                ));
            }
            frame.render_widget(Paragraph::new(Line::from(stats_spans)), chunks[1]);

            // Description
            if let Some(ref desc) = season.evaluate
                && !desc.is_empty()
            {
                let desc_text = if desc.chars().count() > 200 {
                    desc.chars().take(197).collect::<String>() + "…"
                } else {
                    desc.clone()
                };
                frame.render_widget(
                    Paragraph::new(desc_text)
                        .style(Style::default().fg(theme.fg_muted))
                        .wrap(Wrap { trim: true }),
                    chunks[2],
                );
            }
        }
    }

    fn render_episodes(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.flat_episodes.is_empty() {
            let msg = Paragraph::new("暂无剧集信息")
                .style(Style::default().fg(theme.fg_muted))
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle))
            .title(Span::styled(
                " 📋 选集 ",
                Style::default().fg(theme.bilibili_pink),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut items: Vec<ListItem> = Vec::new();
        let mut last_section = String::new();

        for (global_idx, fe) in self.flat_episodes.iter().enumerate() {
            // Section header when section changes
            if fe.section_title != last_section {
                last_section = fe.section_title.clone();
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
            let content = format!("  {}{}", ep.display_title(), badge);

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
        // Adjust for section headers in scroll - we render headers inline so
        // the selected index maps directly to items including headers.
        // Wait, we need to account for section headers in the list.
        // Let me recalculate: each section header adds 1 item before its episodes.
        // Actually, I need to pre-compute the mapping.
        state.select(Some(self.list_index_for_episode(self.selected_episode)));

        frame.render_stateful_widget(list, inner, &mut state);
    }

    /// Map episode index to list index (accounting for section headers)
    fn list_index_for_episode(&self, ep_idx: usize) -> usize {
        let mut list_idx = 0;
        let mut last_section = String::new();
        for (current_ep, fe) in self.flat_episodes.iter().enumerate() {
            if fe.section_title != last_section {
                last_section = fe.section_title.clone();
                list_idx += 1;
            }
            if current_ep == ep_idx {
                return list_idx;
            }
            list_idx += 1;
        }
        list_idx
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

        if let Some(ref err) = self.error_message {
            let error = Paragraph::new(err.as_str())
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            frame.render_widget(error, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Info
                Constraint::Min(6),    // Episodes
                Constraint::Length(2), // Help
            ])
            .split(area);

        self.render_info(frame, chunks[0], theme);
        self.render_episodes(frame, chunks[1], theme);

        // Help
        let help = Paragraph::new(shortcut_footer(
            theme,
            [
                (
                    format!("{}/{}", keys.nav_up, keys.nav_down),
                    "滚动".into(),
                    theme.fg_accent,
                ),
                (keys.confirm.clone(), "播放".into(), theme.success),
                (keys.back.clone(), "返回".into(), theme.info),
            ],
        ))
        .alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }

    fn handle_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        // Global keybindings
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

        if self.loading {
            return Some(AppAction::None);
        }

        if keys.matches_down(key) {
            self.move_down();
            return Some(AppAction::None);
        }
        if keys.matches_up(key) {
            self.move_up();
            return Some(AppAction::None);
        }
        if keys.matches_play(key) || keys.matches_confirm(key) {
            return self.selected_action();
        }

        Some(AppAction::None)
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        let MouseEvent { kind, row, .. } = event;

        // Only process in episode list area (lower chunk)
        let info_height = 6;
        let list_top = area.y + info_height + 1; // +1 for border
        if row < list_top {
            return None;
        }

        let rel_row = (row - list_top) as usize + self.episode_scroll;

        // Map list row to episode index (skip section headers)
        let mut list_idx = 0;
        let mut last_section = String::new();
        let mut target_ep_idx = None;

        for (ep_idx, fe) in self.flat_episodes.iter().enumerate() {
            if fe.section_title != last_section {
                last_section = fe.section_title.clone();
                list_idx += 1;
            }
            if list_idx == rel_row {
                target_ep_idx = Some(ep_idx);
                break;
            }
            list_idx += 1;
        }

        if let Some(idx) = target_ep_idx {
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
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
