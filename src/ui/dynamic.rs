//! Dynamic feed page with video card grid display

use super::Component;
use super::video_card::{VideoCard, VideoCardGrid};
use crate::api::client::ApiClient;
use crate::api::dynamic::DynamicItem;
use crate::app::AppAction;
use ratatui::{
    crossterm::event::KeyCode,
    prelude::*,
    widgets::*,
};

pub struct DynamicPage {
    pub grid: VideoCardGrid,
    pub loading: bool,
    pub error_message: Option<String>,
    pub offset: Option<String>,
    pub has_more: bool,
    pub loading_more: bool,
}

impl DynamicPage {
    pub fn new() -> Self {
        Self {
            grid: VideoCardGrid::new(),
            loading: true,
            error_message: None,
            offset: None,
            has_more: false,
            loading_more: false,
        }
    }

    pub fn set_feed(&mut self, items: Vec<DynamicItem>, offset: Option<String>, has_more: bool) {
        self.grid.clear();
        
        // Filter only video dynamics and convert to cards
        for item in items.into_iter().filter(|i| i.is_video()) {
            if let Some(bvid) = item.video_bvid() {
                // Try to get aid from DynamicItem
                let aid = item.modules
                    .as_ref()
                    .and_then(|m| m.module_dynamic.as_ref())
                    .and_then(|d| d.major.as_ref())
                    .and_then(|m| m.archive.as_ref())
                    .and_then(|a| a.bvid.as_ref())
                    .and_then(|_| None::<i64>); // We'll parse aid from bvid if needed

                let card = VideoCard::new(
                    Some(bvid.to_string()),
                    aid,
                    item.video_title().unwrap_or("无标题").to_string(),
                    item.author_name().to_string(),
                    format!("▶ {}", item.video_play()),
                    item.video_duration().to_string(),
                    item.video_cover().map(|s| s.to_string()),
                );
                self.grid.add_card(card);
            }
        }
        
        self.offset = offset;
        self.has_more = has_more;
        self.loading = false;
    }

    pub fn append_feed(&mut self, items: Vec<DynamicItem>, offset: Option<String>, has_more: bool) {
        for item in items.into_iter().filter(|i| i.is_video()) {
            if let Some(bvid) = item.video_bvid() {
                let card = VideoCard::new(
                    Some(bvid.to_string()),
                    None,
                    item.video_title().unwrap_or("无标题").to_string(),
                    item.author_name().to_string(),
                    format!("▶ {}", item.video_play()),
                    item.video_duration().to_string(),
                    item.video_cover().map(|s| s.to_string()),
                );
                self.grid.add_card(card);
            }
        }
        
        self.offset = offset;
        self.has_more = has_more;
        self.loading_more = false;
    }

    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
        self.loading = false;
        self.loading_more = false;
    }

    pub async fn load_more(&mut self, api_client: &ApiClient) {
        if self.loading_more || !self.has_more {
            return;
        }

        self.loading_more = true;
        
        match api_client.get_dynamic_feed(self.offset.as_deref()).await {
            Ok(data) => {
                let items = data.items.unwrap_or_default();
                let offset = data.offset;
                let has_more = data.has_more.unwrap_or(false);
                self.append_feed(items, offset, has_more);
            }
            Err(_) => {
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

impl Default for DynamicPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for DynamicPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(10),    // Grid
                Constraint::Length(2),  // Help
            ])
            .split(area);

        // Header
        let header = Paragraph::new(Line::from(vec![
            Span::styled(" 📺 ", Style::default()),
            Span::styled("关注动态", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" ({} 条)", self.grid.cards.len()), Style::default().fg(Color::Rgb(100, 100, 100))),
            if self.loading_more {
                Span::styled(" 加载中...", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 60)))
        )
        .alignment(Alignment::Center);
        frame.render_widget(header, chunks[0]);

        // Content
        if self.loading {
            let loading = Paragraph::new("⏳ 加载动态中...")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 60, 60))));
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
            let empty = Paragraph::new("暂无动态，请先登录并关注UP主")
                .style(Style::default().fg(Color::Rgb(100, 100, 100)))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 60, 60))));
            frame.render_widget(empty, chunks[1]);
        } else {
            self.grid.render(frame, chunks[1]);
        }

        // Help
        let help = Paragraph::new("[←↑↓→/hjkl] 导航  [Enter] 详情  [r] 刷新  [Tab] 切换")
            .style(Style::default().fg(Color::Rgb(80, 80, 80)))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction> {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                self.grid.move_down();
                // Check for pagination
                if self.grid.is_near_bottom(3) && !self.loading_more && self.has_more {
                    return Some(AppAction::LoadMoreDynamic);
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
                    if let Some(ref bvid) = card.bvid {
                        // For dynamic, we need to get aid from the video info
                        // For now, we'll pass 0 as aid and handle it in the detail page
                        return Some(AppAction::OpenVideoDetail(bvid.clone(), 0));
                    }
                }
                Some(AppAction::None)
            }
            KeyCode::Char('r') => {
                self.loading = true;
                self.grid.clear();
                Some(AppAction::RefreshDynamic)
            }
            KeyCode::Tab => Some(AppAction::NavNext),
            KeyCode::Char('q') => Some(AppAction::Quit),
            _ => Some(AppAction::None),
        }
    }
}
