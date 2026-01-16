//! Live streaming detail page with room info and real-time messages

use super::{Component, Theme};
use crate::api::client::ApiClient;
use crate::api::live::LiveRoomInfo;
use crate::api::live_client::LiveClient;
use crate::api::live_ws::LiveMessage;
use crate::app::AppAction;
use crate::storage::Keybindings;
use ratatui::crossterm::event::KeyCode;
use ratatui::{prelude::*, widgets::*};
use std::collections::VecDeque;
use std::time::Instant;

/// Maximum number of messages to keep in buffers
const MAX_MESSAGES: usize = 100;

/// Danmaku item for display
#[derive(Debug, Clone)]
struct DanmakuItem {
    uname: String,
    content: String,
    color: Color,
    #[allow(dead_code)]
    timestamp: Instant,
}

/// Entry message (user entered room)
#[derive(Debug, Clone)]
struct EntryItem {
    uname: String,
    #[allow(dead_code)]
    timestamp: Instant,
}

pub struct LiveDetailPage {
    room_id: i64,
    room_info: Option<LiveRoomInfo>,
    loading: bool,
    error: Option<String>,

    // WebSocket client
    live_client: Option<LiveClient>,
    ws_connected: bool,
    ws_error: Option<String>,

    // Message buffers
    danmakus: VecDeque<DanmakuItem>,
    entries: VecDeque<EntryItem>,
    popularity: Option<u32>,
    history_loaded: bool,
}

impl LiveDetailPage {
    pub fn new(room_id: i64) -> Self {
        Self {
            room_id,
            room_info: None,
            loading: false,
            error: None,
            live_client: None,
            ws_connected: false,
            ws_error: None,
            danmakus: VecDeque::with_capacity(MAX_MESSAGES),
            entries: VecDeque::with_capacity(MAX_MESSAGES),
            popularity: None,
            history_loaded: false,
        }
    }

    pub async fn load_room_info(&mut self, api_client: &ApiClient) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;

        match api_client.get_live_room_info(self.room_id).await {
            Ok(info) => {
                self.room_info = Some(info);
                self.loading = false;
            }
            Err(e) => {
                self.error = Some(format!("加载房间信息失败: {}", e));
                self.loading = false;
            }
        }
    }

    /// Load history danmaku before connecting to WebSocket
    pub async fn load_history_danmaku(&mut self, api_client: &ApiClient) {
        if self.history_loaded {
            return;
        }

        match api_client.get_history_danmaku(self.room_id).await {
            Ok(history) => {
                // Merge admin and room messages, take up to 30 most recent
                let all_messages: Vec<_> = history
                    .admin
                    .iter()
                    .chain(history.room.iter())
                    .take(30)
                    .collect();

                for item in all_messages {
                    self.danmakus.push_back(DanmakuItem {
                        uname: item.nickname.clone(),
                        content: item.text.clone(),
                        color: Color::White, // History messages use default color
                        timestamp: Instant::now(),
                    });
                }
                self.history_loaded = true;
            }
            Err(_) => {
                // Don't show error for history loading failure, just skip
            }
        }
    }

    /// Connect to WebSocket for live messages
    pub async fn connect_ws(&mut self, api_client: &ApiClient, uid: i64) {
        // Load history danmaku first
        if !self.history_loaded {
            self.load_history_danmaku(api_client).await;
        }

        // Get danmu info (token + hosts)
        match api_client.get_danmu_info(self.room_id).await {
            Ok(danmu_info) => {
                // Connect to WebSocket
                match LiveClient::connect(self.room_id, uid, &danmu_info).await {
                    Ok(client) => {
                        self.live_client = Some(client);
                        self.ws_connected = true;
                        self.ws_error = None;
                    }
                    Err(e) => {
                        self.ws_error = Some(format!("WS连接失败: {}", e));
                    }
                }
            }
            Err(e) => {
                // Show detailed error in UI
                self.ws_error = Some(format!("{}", e));
            }
        }
    }

    /// Poll for new messages from WebSocket
    pub fn poll_messages(&mut self) {
        // Collect messages first to avoid borrow issues
        let messages: Vec<LiveMessage> = if let Some(ref mut client) = self.live_client {
            let mut msgs = Vec::new();
            while let Some(msg) = client.try_recv() {
                msgs.push(msg);
            }
            msgs
        } else {
            Vec::new()
        };

        // Process collected messages
        for msg in messages {
            self.handle_message(msg);
        }
    }

    /// Handle a received live message
    fn handle_message(&mut self, msg: LiveMessage) {
        match msg {
            LiveMessage::Danmaku {
                uname,
                content,
                color,
                ..
            } => {
                // Convert color from u32 to ratatui Color
                let r = ((color >> 16) & 0xFF) as u8;
                let g = ((color >> 8) & 0xFF) as u8;
                let b = (color & 0xFF) as u8;
                let color = Color::Rgb(r, g, b);

                self.danmakus.push_back(DanmakuItem {
                    uname,
                    content,
                    color,
                    timestamp: Instant::now(),
                });

                // Keep buffer size limited
                while self.danmakus.len() > MAX_MESSAGES {
                    self.danmakus.pop_front();
                }
            }
            LiveMessage::Enter { uname, .. } => {
                self.entries.push_back(EntryItem {
                    uname,
                    timestamp: Instant::now(),
                });

                while self.entries.len() > MAX_MESSAGES {
                    self.entries.pop_front();
                }
            }
            LiveMessage::Popularity(count) => {
                self.popularity = Some(count);
            }
            LiveMessage::AuthReply { code } => {
                if code != 0 {
                    self.ws_error = Some(format!("WebSocket认证失败: {}", code));
                }
            }
            _ => {}
        }
    }

    fn format_online(online: i64) -> String {
        if online >= 10000 {
            format!("{:.1}万", online as f64 / 10000.0)
        } else {
            format!("{}", online)
        }
    }

    fn format_attention(attention: i64) -> String {
        if attention >= 10000 {
            format!("{:.1}万", attention as f64 / 10000.0)
        } else {
            format!("{}", attention)
        }
    }
}

impl Component for LiveDetailPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        // Poll for new messages
        self.poll_messages();

        // Main block
        let title = if let Some(ref info) = self.room_info {
            format!(" 📺 {} ", info.title)
        } else {
            " 📺 直播详情 ".to_string()
        };

        let block = Block::default()
            .title(Span::styled(
                title,
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading && self.room_info.is_none() {
            let loading = Paragraph::new("加载中...")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.fg_muted));
            frame.render_widget(loading, inner);
            return;
        }

        if let Some(ref err) = self.error {
            let error = Paragraph::new(format!("加载失败: {}", err))
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.bilibili_pink));
            frame.render_widget(error, inner);
            return;
        }

        if let Some(ref info) = self.room_info {
            self.render_room_content(frame, inner, info, theme, keys);
        }
    }

    fn handle_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        if keys.matches_quit(key) || keys.matches_back(key) {
            Some(AppAction::BackToList)
        } else if keys.matches_confirm(key) || keys.matches_play(key) {
            self.room_info.as_ref().map(|info| AppAction::PlayLive {
                room_id: info.room_id,
                title: info.title.clone(),
            })
        } else if keys.matches_next_theme(key) {
            Some(AppAction::NextTheme)
        } else if keys.matches_open_settings(key) {
            Some(AppAction::SwitchToSettings)
        } else {
            Some(AppAction::None)
        }
    }
}

impl LiveDetailPage {
    fn render_room_content(
        &self,
        frame: &mut Frame,
        area: Rect,
        info: &LiveRoomInfo,
        theme: &Theme,
        keys: &Keybindings,
    ) {
        // Layout: main content + bottom hints
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),   // Content
                Constraint::Length(1), // Hints
            ])
            .split(area);

        // Content layout: left info panel + right messages
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30), // Room info
                Constraint::Min(20),    // Messages
            ])
            .split(chunks[0]);

        // Right side: messages + entries
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70), // Danmakus
                Constraint::Percentage(30), // Entries
            ])
            .split(content_chunks[1]);

        // Room info panel
        self.render_info_panel(frame, content_chunks[0], info, theme);

        // Danmaku panel
        self.render_danmaku_panel(frame, right_chunks[0], theme);

        // Entry panel
        self.render_entry_panel(frame, right_chunks[1], theme);

        // Bottom hints
        let hints = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {}/", &keys.confirm),
                Style::default().fg(theme.fg_accent),
            ),
            Span::styled(
                format!("{} ", &keys.play),
                Style::default().fg(theme.fg_accent),
            ),
            Span::styled("播放  ", Style::default().fg(theme.fg_secondary)),
            Span::styled(&keys.back, Style::default().fg(theme.error)),
            Span::styled("/", Style::default().fg(theme.fg_secondary)),
            Span::styled(&keys.quit, Style::default().fg(theme.error)),
            Span::styled(" 返回", Style::default().fg(theme.fg_secondary)),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(hints, chunks[1]);
    }

    fn render_info_panel(&self, frame: &mut Frame, area: Rect, info: &LiveRoomInfo, theme: &Theme) {
        let block = Block::default()
            .title(Span::styled(
                " 房间信息 ",
                Style::default()
                    .fg(theme.fg_secondary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let status_color = match info.live_status {
            1 => theme.success,
            2 => theme.fg_accent,
            _ => theme.fg_muted,
        };

        // Use popularity from WebSocket if available
        let online_str = if let Some(pop) = self.popularity {
            Self::format_online(pop as i64)
        } else {
            Self::format_online(info.online)
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(theme.fg_muted)),
                Span::styled(
                    format!("{}", info.room_id),
                    Style::default().fg(theme.fg_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled("分区: ", Style::default().fg(theme.fg_muted)),
                Span::styled(&info.area_name, Style::default().fg(theme.fg_accent)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("● ", Style::default().fg(status_color)),
                Span::styled(info.status_text(), Style::default().fg(status_color)),
            ]),
            Line::from(vec![
                Span::styled("👁 ", Style::default().fg(theme.fg_muted)),
                Span::styled(online_str, Style::default().fg(theme.fg_accent)),
            ]),
            Line::from(vec![
                Span::styled("❤ ", Style::default().fg(theme.bilibili_pink)),
                Span::styled(
                    Self::format_attention(info.attention),
                    Style::default().fg(theme.bilibili_pink),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, inner);
    }

    fn render_danmaku_panel(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(Span::styled(
                format!(" 弹幕 ({}) ", self.danmakus.len()),
                Style::default()
                    .fg(theme.fg_secondary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.danmakus.is_empty() {
            let msg = if self.ws_connected {
                "等待弹幕...".to_string()
            } else if let Some(ref err) = self.ws_error {
                // Show the actual error message
                format!("连接失败: {}", err)
            } else {
                "弹幕功能加载中...".to_string()
            };
            let placeholder = Paragraph::new(msg)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(theme.error));
            frame.render_widget(placeholder, inner);
            return;
        }

        // Show recent danmakus (auto-scroll to bottom)
        let visible_lines = inner.height as usize;
        let start = self.danmakus.len().saturating_sub(visible_lines);

        let lines: Vec<Line> = self
            .danmakus
            .iter()
            .skip(start)
            .take(visible_lines)
            .map(|d| {
                Line::from(vec![
                    Span::styled(
                        format!("[{}] ", d.uname),
                        Style::default().fg(theme.fg_muted),
                    ),
                    Span::styled(&d.content, Style::default().fg(d.color)),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    fn render_entry_panel(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(Span::styled(
                " 入场 ",
                Style::default()
                    .fg(theme.fg_secondary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_subtle));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.entries.is_empty() {
            let placeholder = Paragraph::new("等待入场消息...")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.fg_muted));
            frame.render_widget(placeholder, inner);
            return;
        }

        // Show recent entries
        let visible_lines = inner.height as usize;
        let start = self.entries.len().saturating_sub(visible_lines);

        let lines: Vec<Line> = self
            .entries
            .iter()
            .skip(start)
            .take(visible_lines)
            .map(|e| {
                Line::from(Span::styled(
                    format!("欢迎 {} 进入直播间", e.uname),
                    Style::default().fg(theme.fg_muted),
                ))
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}
