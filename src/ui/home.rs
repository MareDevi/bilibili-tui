//! Homepage with video recommendations in a grid layout with cover images

use super::Component;
use crate::api::client::ApiClient;
use crate::api::recommend::VideoItem;
use crate::app::AppAction;
use image::DynamicImage;
use ratatui::{
    crossterm::event::KeyCode,
    prelude::*,
    widgets::*,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use std::collections::HashSet;
use std::sync::Arc;
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
}

impl HomePage {
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
            columns: 3,
            card_height: 12,
            cover_tx,
            cover_rx,
            pending_downloads: HashSet::new(),
            fresh_idx: 1,
            loading_more: false,
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
                    .map(|video| VideoCard {
                        video,
                        cover: None,
                    })
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

    pub async fn load_more(&mut self, api_client: &ApiClient) {
        if self.loading_more {
            return;
        }

        self.loading_more = true;
        self.fresh_idx += 1;

        match api_client.get_recommendations_paged(self.fresh_idx).await {
            Ok(videos) => {
                for video in videos {
                    self.videos.push(VideoCard {
                        video,
                        cover: None,
                    });
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
        let total = self.total_rows();
        current_row + 2 >= total.saturating_sub(1) && total > visible_rows
    }

    /// Start background downloads for visible covers (non-blocking)
    pub fn start_cover_downloads(&mut self) {
        if self.videos.is_empty() {
            return;
        }

        // Calculate visible range
        let start = self.scroll_row * self.columns;
        let end = (start + self.columns * 4).min(self.videos.len()); // Prefetch extra rows
        
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
                        let _ = tx.send(CoverResult { index: idx, protocol }).await;
                    }
                });
            }
        }
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
    }

    async fn download_image(url: &str) -> Option<DynamicImage> {
        let response = reqwest::get(url).await.ok()?;
        let bytes = response.bytes().await.ok()?;
        image::load_from_memory(&bytes).ok()
    }

    fn visible_rows(&self, height: u16) -> usize {
        let available_height = height.saturating_sub(5);
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

    fn total_rows(&self) -> usize {
        (self.videos.len() + self.columns - 1) / self.columns
    }
}

impl Default for HomePage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for HomePage {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(2),
            ])
            .split(area);

        // Header with enhanced styling
        let title = Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled("B", Style::default().fg(Color::Rgb(251, 114, 153)).add_modifier(Modifier::BOLD)),
            Span::styled("ilibili ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("推荐", Style::default().fg(Color::Cyan)),
            Span::styled(" │ ", Style::default().fg(Color::Rgb(80, 80, 80))),
            Span::styled(format!("{}", self.videos.len()), Style::default().fg(Color::Yellow)),
            Span::styled(" 个视频 │ ", Style::default().fg(Color::Rgb(80, 80, 80))),
            Span::styled(format!("{}", self.selected_row() + 1), Style::default().fg(Color::Green)),
            Span::styled("/", Style::default().fg(Color::Rgb(80, 80, 80))),
            Span::styled(format!("{}", self.total_rows()), Style::default().fg(Color::Green)),
            Span::styled(" 行 ", Style::default().fg(Color::Rgb(80, 80, 80))),
        ]);
        
        let header = Paragraph::new(title)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 60, 60)))
                    .title(Span::styled(" 首页 ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            )
            .alignment(Alignment::Center);
        frame.render_widget(header, chunks[0]);

        // Video grid
        if self.loading {
            let loading = Paragraph::new("⏳ 加载中...")
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC))
                .alignment(Alignment::Center);
            frame.render_widget(loading, chunks[1]);
        } else if let Some(ref error) = self.error_message {
            let error_widget = Paragraph::new(format!("❌ {}", error))
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            frame.render_widget(error_widget, chunks[1]);
        } else if self.videos.is_empty() {
            let empty = Paragraph::new("📭 暂无推荐视频")
                .style(Style::default().fg(Color::Rgb(100, 100, 100)))
                .alignment(Alignment::Center);
            frame.render_widget(empty, chunks[1]);
        } else {
            self.render_grid(frame, chunks[1]);
        }

        // Help with styled shortcuts
        let help_line = Line::from(vec![
            Span::styled(" [", Style::default().fg(Color::Rgb(60, 60, 60))),
            Span::styled("←↑↓→", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("/", Style::default().fg(Color::Rgb(60, 60, 60))),
            Span::styled("hjkl", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("] ", Style::default().fg(Color::Rgb(60, 60, 60))),
            Span::styled("导航", Style::default().fg(Color::Rgb(120, 120, 120))),
            Span::styled("  [", Style::default().fg(Color::Rgb(60, 60, 60))),
            Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("] ", Style::default().fg(Color::Rgb(60, 60, 60))),
            Span::styled("播放", Style::default().fg(Color::Rgb(120, 120, 120))),
            Span::styled("  [", Style::default().fg(Color::Rgb(60, 60, 60))),
            Span::styled("r", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("] ", Style::default().fg(Color::Rgb(60, 60, 60))),
            Span::styled("刷新", Style::default().fg(Color::Rgb(120, 120, 120))),
            Span::styled("  [", Style::default().fg(Color::Rgb(60, 60, 60))),
            Span::styled("q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled("] ", Style::default().fg(Color::Rgb(60, 60, 60))),
            Span::styled("退出", Style::default().fg(Color::Rgb(120, 120, 120))),
        ]);
        let help = Paragraph::new(help_line).alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction> {
        match key {
            KeyCode::Char('q') => Some(AppAction::Quit),
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.videos.is_empty() {
                    let new_idx = self.selected_index + self.columns;
                    if new_idx < self.videos.len() {
                        self.selected_index = new_idx;
                    }
                    self.update_scroll(3);
                    // Check for pagination
                    if self.is_near_bottom(3) && !self.loading_more {
                        return Some(AppAction::LoadMoreRecommendations);
                    }
                }
                Some(AppAction::None)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.videos.is_empty() && self.selected_index >= self.columns {
                    self.selected_index -= self.columns;
                    self.update_scroll(3);
                }
                Some(AppAction::None)
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if !self.videos.is_empty() && self.selected_index + 1 < self.videos.len() {
                    self.selected_index += 1;
                    self.update_scroll(3);
                }
                Some(AppAction::None)
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if !self.videos.is_empty() && self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.update_scroll(3);
                }
                Some(AppAction::None)
            }
            KeyCode::Enter => {
                if let Some(card) = self.videos.get(self.selected_index) {
                    if let Some(bvid) = &card.video.bvid {
                        let aid = card.video.id;
                        return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
                    }
                }
                Some(AppAction::None)
            }
            KeyCode::Char('p') => {
                // Direct play without going to detail
                if let Some(card) = self.videos.get(self.selected_index) {
                    if let Some(bvid) = &card.video.bvid {
                        return Some(AppAction::PlayVideo(bvid.clone()));
                    }
                }
                Some(AppAction::None)
            }
            KeyCode::Char('r') => {
                self.loading = true;
                self.videos.clear();
                self.pending_downloads.clear();
                Some(AppAction::SwitchToHome)
            }
            KeyCode::Tab => Some(AppAction::NavNext),
            _ => Some(AppAction::None),
        }
    }
}

impl HomePage {
    fn render_grid(&mut self, frame: &mut Frame, area: Rect) {
        let visible_rows = self.visible_rows(area.height);
        
        // Use fixed card width for consistent layout, cap at available width
        let fixed_card_width: u16 = 45;
        let card_width = fixed_card_width.min(area.width / self.columns as u16);
        
        let row_constraints: Vec<Constraint> = (0..visible_rows)
            .map(|_| Constraint::Length(self.card_height))
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

            // Calculate centering margin based on fixed card width
            let total_cards_width = card_width * self.columns as u16;
            let margin = row_area.width.saturating_sub(total_cards_width) / 2;

            let col_constraints: Vec<Constraint> = (0..self.columns)
                .map(|_| Constraint::Length(card_width))
                .collect();
            
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .horizontal_margin(margin)
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
            self.render_video_card(frame, col_area, video_idx, is_selected);
        }
    }

    fn render_video_card(&mut self, frame: &mut Frame, area: Rect, video_idx: usize, is_selected: bool) {
        // Enhanced border styling
        let (border_style, border_type) = if is_selected {
            (
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                BorderType::Rounded
            )
        } else {
            (
                Style::default().fg(Color::Rgb(50, 50, 50)),
                BorderType::Rounded
            )
        };

        let title_span = if is_selected {
            Span::styled(" ▶ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
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
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(4),
                Constraint::Length(4),
            ])
            .split(inner);

        // Cover area - render with StatefulImage
        let cover_area = card_chunks[0];
        if let Some(ref mut cover) = self.videos[video_idx].cover {
            // Render actual image using StatefulImage
            let image_widget = StatefulImage::new();
            frame.render_stateful_widget(image_widget, cover_area, cover);
        } else {
            // Loading placeholder with spinner animation hint
            let is_pending = self.pending_downloads.contains(&video_idx);
            let placeholder_text = if is_pending { "📺 加载中..." } else { "📺" };
            let placeholder = Paragraph::new(placeholder_text)
                .style(Style::default().fg(Color::Rgb(60, 60, 60)))
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
            title.chars().take(max_title_len.saturating_sub(3)).collect::<String>() + "..."
        } else {
            title.to_string()
        };

        // Multi-styled info text
        let title_style = if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(200, 200, 200))
        };
        
        let meta_style = Style::default().fg(Color::Rgb(100, 100, 100));
        
        let info_text = Text::from(vec![
            Line::from(Span::styled(&display_title, title_style)),
            Line::from(Span::styled(author, Style::default().fg(Color::Rgb(150, 150, 150)))),
            Line::from(vec![
                Span::styled(&views, meta_style),
                Span::styled(" · ", meta_style),
                Span::styled(&duration, Style::default().fg(Color::Rgb(80, 180, 80))),
            ]),
        ]);

        let info = Paragraph::new(info_text).wrap(Wrap { trim: true });
        frame.render_widget(info, info_area);
    }
}
