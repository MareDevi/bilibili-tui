//! Video detail page showing video info, comments, and related videos

use super::video_card::{VideoCard, VideoCardGrid};
use super::{Component, Theme};
use crate::api::client::ApiClient;
use crate::api::comment::CommentItem;
use crate::api::video::{RelatedVideoItem, VideoInfo};
use crate::app::AppAction;
use ratatui::{crossterm::event::KeyCode, prelude::*, widgets::*};

#[derive(Clone, Copy, PartialEq)]
pub enum DetailFocus {
    Comments,
    Related,
}

pub struct VideoDetailPage {
    pub bvid: String,
    pub aid: i64,
    pub video_info: Option<VideoInfo>,
    pub comments: Vec<CommentItem>,
    pub related_videos: Vec<RelatedVideoItem>,
    pub related_card_grid: VideoCardGrid,
    pub loading: bool,
    pub error_message: Option<String>,
    pub comment_page: i32,
    pub comment_scroll: usize,
    pub related_scroll: usize,
    pub focus: DetailFocus,
    pub has_more_comments: bool,
    pub loading_more_comments: bool,
}

impl VideoDetailPage {
    pub fn new(bvid: String, aid: i64) -> Self {
        let mut related_card_grid = VideoCardGrid::new();
        related_card_grid.columns = 2; // Two columns for compact layout
        related_card_grid.card_height = 8; // Compact cards for sidebar

        Self {
            bvid,
            aid,
            video_info: None,
            comments: Vec::new(),
            related_videos: Vec::new(),
            related_card_grid,
            loading: true,
            error_message: None,
            comment_page: 1,
            comment_scroll: 0,
            related_scroll: 0,
            focus: DetailFocus::Comments,
            has_more_comments: true,
            loading_more_comments: false,
        }
    }

    pub async fn load_data(&mut self, api_client: &ApiClient) {
        self.loading = true;
        self.error_message = None;

        // Load video info
        match api_client.get_video_info(&self.bvid).await {
            Ok(info) => {
                self.video_info = Some(info);
            }
            Err(e) => {
                self.error_message = Some(format!("加载视频信息失败: {}", e));
            }
        }

        // Load comments
        match api_client.get_comments(self.aid, 1).await {
            Ok(data) => {
                self.comments = data.replies.unwrap_or_default();
                self.comment_page = 1;
                if let Some(page) = data.page {
                    self.has_more_comments = page.count.unwrap_or(0) > self.comments.len() as i32;
                }
            }
            Err(e) => {
                if self.error_message.is_none() {
                    self.error_message = Some(format!("加载评论失败: {}", e));
                }
            }
        }

        // Load related videos
        match api_client.get_related_videos(&self.bvid).await {
            Ok(videos) => {
                self.related_videos = videos.clone();
                // Populate video card grid
                self.related_card_grid.clear();
                for video in &videos {
                    let card = VideoCard::new(
                        video.bvid.clone(),
                        video.aid,
                        video.title.clone().unwrap_or_else(|| "无标题".to_string()),
                        video.author_name().to_string(),
                        video.format_views(),
                        video.format_duration(),
                        video.cover_url(),
                    );
                    self.related_card_grid.add_card(card);
                }
            }
            Err(e) => {
                if self.error_message.is_none() {
                    self.error_message = Some(format!("加载相关视频失败: {}", e));
                }
            }
        }

        self.loading = false;
    }

    pub async fn load_more_comments(&mut self, api_client: &ApiClient) {
        if !self.has_more_comments || self.loading_more_comments {
            return;
        }

        self.loading_more_comments = true;
        self.comment_page += 1;
        match api_client.get_comments(self.aid, self.comment_page).await {
            Ok(data) => {
                if let Some(replies) = data.replies {
                    if replies.is_empty() {
                        self.has_more_comments = false;
                    } else {
                        self.comments.extend(replies);
                    }
                } else {
                    self.has_more_comments = false;
                }
            }
            Err(_) => {
                self.comment_page -= 1;
            }
        }
        self.loading_more_comments = false;
    }

    /// Poll for completed related video cover downloads
    pub fn poll_cover_results(&mut self) {
        self.related_card_grid.poll_cover_results();
    }

    /// Start background downloads for visible related video covers
    pub fn start_cover_downloads(&mut self) {
        self.related_card_grid.start_cover_downloads();
    }

    /// Check if scrolling near bottom of comments
    fn is_near_comments_bottom(&self, visible_count: usize) -> bool {
        if self.comments.is_empty() {
            return false;
        }
        self.comment_scroll + visible_count >= self.comments.len().saturating_sub(2)
    }

    fn render_video_info(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_unfocused))
            .title(Span::styled(
                " 📹 视频信息 ",
                Style::default().fg(theme.fg_accent),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(ref info) = self.video_info {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Title
                    Constraint::Length(1), // Author
                    Constraint::Length(1), // Stats
                    Constraint::Min(1),    // Description
                ])
                .split(inner);

            // Title
            let title = Paragraph::new(info.title.clone()).style(
                Style::default()
                    .fg(theme.fg_primary)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(title, chunks[0]);

            // Author
            let author = Paragraph::new(format!("UP: {}", info.owner.name))
                .style(Style::default().fg(theme.bilibili_pink));
            frame.render_widget(author, chunks[1]);

            // Stats
            let stats = Paragraph::new(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    info.stat.format_views(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(" · 💬 ", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    info.stat.format_danmaku(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(" · 👍 ", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    info.stat.format_like(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(" · 💰 ", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    info.stat.format_coin(),
                    Style::default().fg(theme.fg_secondary),
                ),
                Span::styled(" · ⭐ ", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    info.stat.format_favorite(),
                    Style::default().fg(theme.fg_secondary),
                ),
            ]));
            frame.render_widget(stats, chunks[2]);

            // Description
            if let Some(ref desc) = info.desc {
                let char_count = desc.chars().count();
                let desc_text: String = if char_count > 100 {
                    desc.chars().take(100).collect::<String>() + "..."
                } else {
                    desc.clone()
                };
                let description = Paragraph::new(desc_text)
                    .style(Style::default().fg(theme.fg_secondary))
                    .wrap(Wrap { trim: true });
                frame.render_widget(description, chunks[3]);
            }
        } else {
            let loading = Paragraph::new("加载中...")
                .style(Style::default().fg(theme.warning))
                .alignment(Alignment::Center);
            frame.render_widget(loading, inner);
        }
    }

    fn render_comments(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.focus == DetailFocus::Comments;
        let border_style = if is_focused {
            Style::default().fg(theme.border_focused)
        } else {
            Style::default().fg(theme.border_unfocused)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Span::styled(
                format!(" 💬 评论 ({}) ", self.comments.len()),
                Style::default().fg(if is_focused {
                    theme.fg_accent
                } else {
                    theme.fg_secondary
                }),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.comments.is_empty() {
            let empty = Paragraph::new("暂无评论")
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
            return;
        }

        // Calculate visible items
        let item_height = 3;
        let visible_count = (inner.height as usize / item_height).max(1);

        let items: Vec<ListItem> = self
            .comments
            .iter()
            .skip(self.comment_scroll)
            .take(visible_count)
            .map(|comment| {
                let lines = vec![
                    Line::from(vec![
                        Span::styled(
                            comment.author_name(),
                            Style::default().fg(theme.bilibili_pink),
                        ),
                        Span::styled(
                            format!("  {}", comment.format_time()),
                            Style::default().fg(theme.fg_secondary),
                        ),
                    ]),
                    Line::from(vec![Span::styled(
                        truncate_str(comment.message(), 60),
                        Style::default().fg(theme.fg_primary),
                    )]),
                    Line::from(vec![Span::styled(
                        format!(
                            "👍 {}  💬 {} 回复",
                            comment.format_like(),
                            comment.reply_count()
                        ),
                        Style::default().fg(theme.fg_secondary),
                    )]),
                ];
                ListItem::new(lines)
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, inner);
    }

    fn render_related(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_focused = self.focus == DetailFocus::Related;
        let border_style = if is_focused {
            Style::default().fg(theme.border_focused)
        } else {
            Style::default().fg(theme.border_unfocused)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Span::styled(
                format!(" 📺 相关推荐 ({}) ", self.related_card_grid.cards.len()),
                Style::default().fg(if is_focused {
                    theme.fg_accent
                } else {
                    theme.fg_secondary
                }),
            ));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.related_card_grid.cards.is_empty() {
            let empty = Paragraph::new("暂无相关视频")
                .style(Style::default().fg(theme.fg_secondary))
                .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
            return;
        }

        // Sync scroll position with grid
        self.related_card_grid.selected_index = self.related_scroll;

        // Render the video card grid
        self.related_card_grid.render(frame, inner, theme);
    }
}

impl Component for VideoDetailPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Video info
                Constraint::Min(10),   // Comments + Related
                Constraint::Length(2), // Help
            ])
            .split(area);

        // Video info
        self.render_video_info(frame, chunks[0], theme);

        if self.loading {
            let loading = Paragraph::new("⏳ 加载中...")
                .style(Style::default().fg(theme.warning))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                );
            frame.render_widget(loading, chunks[1]);
        } else if let Some(ref error) = self.error_message {
            let error_widget = Paragraph::new(format!("❌ {}", error))
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                );
            frame.render_widget(error_widget, chunks[1]);
        } else {
            // Comments and Related split
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(60), // Comments
                    Constraint::Percentage(40), // Related
                ])
                .split(chunks[1]);

            self.render_comments(frame, content_chunks[0], theme);
            self.render_related(frame, content_chunks[1], theme);
        }

        // Help
        let help_text = "[j/k] 滚动  [Tab] 切换焦点  [Enter] 选择相关视频  [p] 播放  [q/Esc] 返回";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(theme.fg_secondary))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => Some(AppAction::BackToList),
            KeyCode::Char('p') => Some(AppAction::PlayVideo(self.bvid.clone())),
            KeyCode::Tab => {
                self.focus = match self.focus {
                    DetailFocus::Comments => DetailFocus::Related,
                    DetailFocus::Related => DetailFocus::Comments,
                };
                Some(AppAction::None)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match self.focus {
                    DetailFocus::Comments => {
                        if self.comment_scroll + 1 < self.comments.len() {
                            self.comment_scroll += 1;
                        }
                        // Check if near bottom to load more comments
                        if self.is_near_comments_bottom(10)
                            && self.has_more_comments
                            && !self.loading_more_comments
                        {
                            return Some(AppAction::LoadMoreComments);
                        }
                    }
                    DetailFocus::Related => {
                        if self.related_card_grid.move_down() {
                            self.related_scroll = self.related_card_grid.selected_index;
                        }
                    }
                }
                Some(AppAction::None)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.focus {
                    DetailFocus::Comments => {
                        if self.comment_scroll > 0 {
                            self.comment_scroll -= 1;
                        }
                    }
                    DetailFocus::Related => {
                        if self.related_card_grid.move_up() {
                            self.related_scroll = self.related_card_grid.selected_index;
                        }
                    }
                }
                Some(AppAction::None)
            }
            KeyCode::Enter => {
                if self.focus == DetailFocus::Related {
                    if let Some(card) = self.related_card_grid.selected_card() {
                        if let Some(ref bvid) = card.bvid {
                            let aid = card.aid.unwrap_or(0);
                            return Some(AppAction::OpenVideoDetail(bvid.clone(), aid));
                        }
                    }
                }
                Some(AppAction::None)
            }
            _ => Some(AppAction::None),
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        s.chars()
            .take(max_len.saturating_sub(3))
            .collect::<String>()
            + "..."
    } else {
        s.to_string()
    }
}
