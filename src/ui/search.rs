//! Search page with video card grid display

use super::Component;
use super::video_card::{VideoCard, VideoCardGrid};
use crate::api::client::ApiClient;
use crate::api::search::SearchVideoItem;
use crate::app::AppAction;
use ratatui::{
    crossterm::event::KeyCode,
    prelude::*,
    widgets::*,
};

pub struct SearchPage {
    pub query: String,
    pub grid: VideoCardGrid,
    pub loading: bool,
    pub error_message: Option<String>,
    pub input_mode: bool,
    pub page: i32,
    pub total_results: i32,
    pub loading_more: bool,
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
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Search input
                Constraint::Min(10),    // Results grid
                Constraint::Length(2),  // Help
            ])
            .split(area);

        // Search input
        let input_style = if self.input_mode {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.input_mode {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Rgb(60, 60, 60))
            })
            .title(Span::styled(" 🔍 搜索视频 ", Style::default().fg(Color::Cyan)));

        let cursor_char = if self.input_mode { "▌" } else { "" };
        let input = Paragraph::new(format!("{}{}", self.query, cursor_char))
            .style(input_style)
            .block(input_block);
        frame.render_widget(input, chunks[0]);

        // Results
        if self.loading {
            let loading = Paragraph::new("⏳ 搜索中...")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 60, 60)))
                    .title(Span::styled(
                        format!(" 结果 ({}) ", self.total_results),
                        Style::default().fg(Color::Rgb(150, 150, 150))
                    )));
            frame.render_widget(loading, chunks[1]);
        } else if let Some(ref error) = self.error_message {
            let error_widget = Paragraph::new(format!("❌ {}", error))
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 60, 60))));
            frame.render_widget(error_widget, chunks[1]);
        } else if self.grid.cards.is_empty() {
            let empty = Paragraph::new(if self.query.is_empty() {
                "输入关键词开始搜索"
            } else {
                "没有找到相关视频"
            })
                .style(Style::default().fg(Color::Rgb(100, 100, 100)))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 60, 60))));
            frame.render_widget(empty, chunks[1]);
        } else {
            // Render with header
            let header = Paragraph::new(Line::from(vec![
                Span::styled(" 搜索结果 ", Style::default().fg(Color::Cyan)),
                Span::styled(format!("({}/{})", self.grid.cards.len(), self.total_results), 
                    Style::default().fg(Color::Rgb(100, 100, 100))),
                if self.loading_more {
                    Span::styled(" 加载中...", Style::default().fg(Color::Yellow))
                } else {
                    Span::raw("")
                },
            ]))
            .block(Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 60))));
            
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
            self.grid.render(frame, grid_area);
        }

        // Help
        let help_text = if self.input_mode {
            "[Enter] 搜索  [Esc] 取消  [Tab] 导航"
        } else {
            "[←↑↓→/hjkl] 导航  [Enter] 详情  [/] 搜索  [Tab] 切换"
        };
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Rgb(80, 80, 80)))
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
                        if let (Some(ref bvid), Some(aid)) = (&card.bvid, card.aid) {
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
}
