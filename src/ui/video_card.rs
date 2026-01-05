//! Shared video card components for grid display across pages

use super::Theme;
use image::DynamicImage;
use ratatui::prelude::*;
use ratatui::widgets::*;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Message for completed cover download
pub struct CoverResult {
    pub index: usize,
    pub protocol: StatefulProtocol,
}

/// A video card with cover image
pub struct VideoCard {
    pub bvid: Option<String>,
    pub aid: Option<i64>,
    pub title: String,
    pub author: String,
    pub views: String,
    pub duration: String,
    pub pic_url: Option<String>,
    pub cover: Option<StatefulProtocol>,
}

impl VideoCard {
    pub fn new(
        bvid: Option<String>,
        aid: Option<i64>,
        title: String,
        author: String,
        views: String,
        duration: String,
        pic_url: Option<String>,
    ) -> Self {
        Self {
            bvid,
            aid,
            title,
            author,
            views,
            duration,
            pic_url,
            cover: None,
        }
    }

    /// Render a single video card
    pub fn render(&mut self, frame: &mut Frame, area: Rect, is_selected: bool, theme: &Theme) {
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
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(4),    // Cover
                Constraint::Length(4), // Info
            ])
            .split(inner);

        // Cover area - center the image horizontally
        let cover_area = card_chunks[0];

        // Calculate centered cover area (assuming 16:9 aspect ratio for video covers)
        // The image will be centered within the available cover_area
        let target_width = cover_area.width.saturating_sub(2); // Leave 1 char margin on each side
        let centered_cover = Rect {
            x: cover_area.x + (cover_area.width.saturating_sub(target_width)) / 2,
            y: cover_area.y,
            width: target_width,
            height: cover_area.height,
        };

        if let Some(ref mut cover) = self.cover {
            let image_widget = StatefulImage::new();
            frame.render_stateful_widget(image_widget, centered_cover, cover);
        } else {
            let placeholder = Paragraph::new("📺")
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center);
            frame.render_widget(placeholder, cover_area);
        }

        // Video info
        let info_area = card_chunks[1];
        let max_title_len = (info_area.width as usize).saturating_sub(2);
        let display_title: String = if self.title.chars().count() > max_title_len {
            self.title
                .chars()
                .take(max_title_len.saturating_sub(3))
                .collect::<String>()
                + "..."
        } else {
            self.title.clone()
        };

        let title_style = if is_selected {
            Style::default()
                .fg(theme.fg_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg_secondary)
        };

        let meta_style = Style::default().fg(theme.fg_secondary);

        let info_text = Text::from(vec![
            Line::from(Span::styled(&display_title, title_style)),
            Line::from(Span::styled(
                &self.author,
                Style::default().fg(theme.fg_secondary),
            )),
            Line::from(vec![
                Span::styled(&self.views, meta_style),
                Span::styled(" · ", meta_style),
                Span::styled(&self.duration, Style::default().fg(theme.success)),
            ]),
        ]);

        let info = Paragraph::new(info_text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        frame.render_widget(info, info_area);
    }
}

/// Video card grid manager for async cover loading
pub struct VideoCardGrid {
    pub cards: Vec<VideoCard>,
    pub selected_index: usize,
    pub scroll_row: usize,
    pub columns: usize,
    pub card_height: u16,
    pub picker: Arc<Picker>,
    pub cover_tx: mpsc::Sender<CoverResult>,
    pub cover_rx: mpsc::Receiver<CoverResult>,
    pub pending_downloads: HashSet<usize>,
}

impl VideoCardGrid {
    pub fn new() -> Self {
        let picker = Arc::new(Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks()));
        let (cover_tx, cover_rx) = mpsc::channel(32);

        Self {
            cards: Vec::new(),
            selected_index: 0,
            scroll_row: 0,
            columns: 3,
            card_height: 12,
            picker,
            cover_tx,
            cover_rx,
            pending_downloads: HashSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.cards.clear();
        self.selected_index = 0;
        self.scroll_row = 0;
        self.pending_downloads.clear();
    }

    pub fn add_card(&mut self, card: VideoCard) {
        self.cards.push(card);
    }

    pub fn visible_rows(&self, height: u16) -> usize {
        let available_height = height.saturating_sub(1);
        (available_height / self.card_height).max(1) as usize
    }

    pub fn selected_row(&self) -> usize {
        self.selected_index / self.columns
    }

    pub fn total_rows(&self) -> usize {
        self.cards.len().div_ceil(self.columns)
    }

    pub fn update_scroll(&mut self, visible_rows: usize) {
        let current_row = self.selected_row();
        if current_row < self.scroll_row {
            self.scroll_row = current_row;
        } else if current_row >= self.scroll_row + visible_rows {
            self.scroll_row = current_row - visible_rows + 1;
        }
    }

    pub fn move_down(&mut self) -> bool {
        if !self.cards.is_empty() {
            let new_idx = self.selected_index + self.columns;
            if new_idx < self.cards.len() {
                self.selected_index = new_idx;
                self.update_scroll(5);
                return true;
            }
        }
        false
    }

    pub fn move_up(&mut self) -> bool {
        if !self.cards.is_empty() && self.selected_index >= self.columns {
            self.selected_index -= self.columns;
            self.update_scroll(5);
            return true;
        }
        false
    }

    pub fn move_right(&mut self) -> bool {
        if !self.cards.is_empty() && self.selected_index + 1 < self.cards.len() {
            self.selected_index += 1;
            self.update_scroll(3);
            return true;
        }
        false
    }

    pub fn move_left(&mut self) -> bool {
        if !self.cards.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
            self.update_scroll(3);
            return true;
        }
        false
    }

    /// Check if near bottom for pagination
    pub fn is_near_bottom(&self, visible_rows: usize) -> bool {
        if self.cards.is_empty() {
            return false;
        }
        let current_row = self.selected_row();
        let total = self.total_rows();
        current_row + 2 >= total.saturating_sub(1) && total > visible_rows
    }

    /// Start background downloads for visible covers
    pub fn start_cover_downloads(&mut self) {
        if self.cards.is_empty() {
            return;
        }

        let start = self.scroll_row * self.columns;
        let end = (start + self.columns * 4).min(self.cards.len());

        for idx in start..end {
            if self.cards[idx].cover.is_some() || self.pending_downloads.contains(&idx) {
                continue;
            }

            if let Some(pic_url) = self.cards[idx].pic_url.clone() {
                self.pending_downloads.insert(idx);
                let tx = self.cover_tx.clone();
                let picker = Arc::clone(&self.picker);

                tokio::spawn(async move {
                    if let Some(img) = download_image(&pic_url).await {
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

    /// Poll for completed cover downloads
    pub fn poll_cover_results(&mut self) {
        while let Ok(result) = self.cover_rx.try_recv() {
            if result.index < self.cards.len() {
                self.cards[result.index].cover = Some(result.protocol);
                self.pending_downloads.remove(&result.index);
            }
        }
    }

    /// Render the grid
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let visible_rows = self.visible_rows(area.height);

        let row_constraints: Vec<Constraint> = (0..visible_rows)
            .map(|_| Constraint::Min(self.card_height))
            .collect();

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(area);

        let mut card_areas: Vec<(usize, Rect)> = Vec::new();

        for (row_offset, row_area) in rows.iter().enumerate() {
            let actual_row = self.scroll_row + row_offset;
            let start_idx = actual_row * self.columns;

            if start_idx >= self.cards.len() {
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
                if video_idx >= self.cards.len() {
                    break;
                }
                card_areas.push((video_idx, *col_area));
            }
        }

        for (video_idx, col_area) in card_areas {
            let is_selected = video_idx == self.selected_index;
            self.cards[video_idx].render(frame, col_area, is_selected, theme);
        }
    }

    pub fn selected_card(&self) -> Option<&VideoCard> {
        self.cards.get(self.selected_index)
    }
}

impl Default for VideoCardGrid {
    fn default() -> Self {
        Self::new()
    }
}

async fn download_image(url: &str) -> Option<DynamicImage> {
    let response = reqwest::get(url).await.ok()?;
    let bytes = response.bytes().await.ok()?;
    image::load_from_memory(&bytes).ok()
}
