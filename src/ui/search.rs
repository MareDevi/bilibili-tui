//! Search page with video card grid display

use super::video_card::{VideoCard, VideoCardGrid};
use super::{Component, Theme};
use crate::api::client::ApiClient;
use crate::api::search::SearchVideoItem;
use crate::app::AppAction;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind},
    prelude::*,
    widgets::*,
};
use std::time::Instant;

pub struct SearchPage {
    pub query: String,
    pub grid: VideoCardGrid,
    pub loading: bool,
    pub error_message: Option<String>,
    pub input_mode: bool,
    pub page: i32,
    pub total_results: i32,
    pub loading_more: bool,
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
}

impl SearchPage {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            grid: VideoCardGrid::new(),
            loading: false,
            error_message: None,
            input_mode: true,
            page: 1,
            total_results: 0,
            loading_more: false,
            last_click_time: None,
            last_click_index: None,
        }
    }

    pub fn set_results(&mut self, results: Vec<SearchVideoItem>, total: i32) {
        self.grid.clear();
        for item in results {
            let card = VideoCard::new(
                item.bvid.clone(),
                item.mid,
                item.display_title(),
                item.author_name().to_string(),
                item.format_play(),
                item.duration.clone().unwrap_or_default(),
                item.cover_url(),
            );
            self.grid.add_card(card);
        }
        self.total_results = total;
        self.loading = false;
        self.input_mode = false;
    }

    pub fn append_results(&mut self, results: Vec<SearchVideoItem>) {
        for item in results {
            let card = VideoCard::new(
                item.bvid.clone(),
                item.mid,
                item.display_title(),
                item.author_name().to_string(),
                item.format_play(),
                item.duration.clone().unwrap_or_default(),
                item.cover_url(),
            );
            self.grid.add_card(card);
        }
        self.loading_more = false;
    }

    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
        self.loading = false;
        self.loading_more = false;
    }

    pub async fn load_more(&mut self, api_client: &ApiClient) {
        if self.loading_more || self.query.is_empty() {
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
}

impl Default for SearchPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for SearchPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search input
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
                " 🔍 搜索视频 ",
                Style::default().fg(theme.bilibili_pink),
            ));

        let cursor_char = if self.input_mode { "▌" } else { "" };
        let input = Paragraph::new(format!("{}{}", self.query, cursor_char))
            .style(input_style)
            .block(input_block);
        frame.render_widget(input, chunks[0]);

        // Results
        if self.loading {
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
            frame.render_widget(loading, chunks[1]);
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
            frame.render_widget(error_widget, chunks[1]);
        } else if self.grid.cards.is_empty() {
            let empty = Paragraph::new(if self.query.is_empty() {
                "输入关键词开始搜索"
            } else {
                "没有找到相关视频"
            })
            .style(Style::default().fg(theme.fg_secondary))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_unfocused)),
            );
            frame.render_widget(empty, chunks[1]);
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
                ..chunks[1]
            };
            let grid_area = Rect {
                y: chunks[1].y + 2,
                height: chunks[1].height.saturating_sub(2),
                ..chunks[1]
            };

            frame.render_widget(header, header_area);
            self.grid.render(frame, grid_area, theme);
        }

        // Help
        let help_text = if self.input_mode {
            "[Enter] 搜索  [Esc] 取消  [Tab] 导航"
        } else {
            "[←↑↓→/hjkl] 导航  [Enter] 详情  [/] 搜索  [Tab] 切换"
        };
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(theme.fg_secondary))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction> {
        if self.input_mode {
            match key {
                KeyCode::Char(c) => {
                    self.query.push(c);
                    Some(AppAction::None)
                }
                KeyCode::Backspace => {
                    self.query.pop();
                    Some(AppAction::None)
                }
                KeyCode::Enter => {
                    if !self.query.is_empty() {
                        self.loading = true;
                        self.page = 1;
                        Some(AppAction::Search(self.query.clone()))
                    } else {
                        Some(AppAction::None)
                    }
                }
                KeyCode::Esc => {
                    self.input_mode = false;
                    Some(AppAction::None)
                }
                KeyCode::Tab => Some(AppAction::NavNext),
                _ => Some(AppAction::None),
            }
        } else {
            match key {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.grid.move_down();
                    // Check for pagination
                    if self.grid.is_near_bottom(3) && !self.loading_more {
                        return Some(AppAction::LoadMoreSearch);
                    }
                    Some(AppAction::None)
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.grid.move_up();
                    Some(AppAction::None)
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    self.grid.move_right();
                    Some(AppAction::None)
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    self.grid.move_left();
                    Some(AppAction::None)
                }
                KeyCode::Enter => {
                    if let Some(card) = self.grid.selected_card() {
                        if let (Some(bvid), Some(aid)) = (&card.bvid, card.aid) {
                            return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
                        }
                    }
                    Some(AppAction::None)
                }
                KeyCode::Char('/') | KeyCode::Char('i') => {
                    self.input_mode = true;
                    Some(AppAction::None)
                }
                KeyCode::Tab => Some(AppAction::NavNext),
                KeyCode::Char('q') => Some(AppAction::Quit),
                _ => Some(AppAction::None),
            }
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        // Don't handle mouse in input mode
        if self.input_mode {
            return None;
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
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(2),
                    ])
                    .split(area);

                let header_height = 2u16;
                let grid_area = Rect {
                    y: chunks[1].y + header_height,
                    height: chunks[1].height.saturating_sub(header_height),
                    x: chunks[1].x,
                    width: chunks[1].width,
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
                    let is_double_click = self.last_click_index == Some(click_idx)
                        && self
                            .last_click_time
                            .is_some_and(|t| now.duration_since(t).as_millis() < 500);

                    if is_double_click {
                        self.last_click_time = None;
                        self.last_click_index = None;
                        if let Some(card) = self.grid.cards.get(click_idx) {
                            if let (Some(bvid), Some(aid)) = (&card.bvid, card.aid) {
                                return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
                            }
                        }
                    } else {
                        self.grid.selected_index = click_idx;
                        self.grid.update_scroll(self.grid.cached_visible_rows);
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
