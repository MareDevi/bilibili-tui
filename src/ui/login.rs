//! Login page with QR code display

use super::{Component, Theme};
use crate::api::auth::{QrcodeData, QrcodePollStatus};
use crate::api::client::ApiClient;
use crate::app::AppAction;
use crate::storage::Credentials;
use qrcode::QrCode;
use ratatui::{crossterm::event::KeyCode, prelude::*, widgets::*};
use std::time::{Duration, Instant};
use tui_qrcode::QrCodeWidget;

pub struct LoginPage {
    qrcode_data: Option<QrcodeData>,
    error_message: Option<String>,
    poll_status: QrcodePollStatus,
    last_poll: Option<Instant>,
}

impl LoginPage {
    pub fn new() -> Self {
        Self {
            qrcode_data: None,
            error_message: None,
            poll_status: QrcodePollStatus::Waiting,
            last_poll: None,
        }
    }

    pub async fn load_qrcode(&mut self, api_client: &ApiClient) {
        match api_client.get_qrcode_data().await {
            Ok(data) => {
                self.qrcode_data = Some(data);
                self.error_message = None;
                self.poll_status = QrcodePollStatus::Waiting;
                self.last_poll = None;
            }
            Err(e) => {
                self.error_message = Some(format!("获取二维码失败: {}", e));
            }
        }
    }

    pub async fn tick(&mut self, api_client: &ApiClient) -> Option<AppAction> {
        // Only poll if we have a QR code and haven't succeeded/expired
        let qrcode_key = match &self.qrcode_data {
            Some(data) => data.qrcode_key.clone(),
            None => return None,
        };

        // Don't poll if already successful or expired
        if matches!(
            self.poll_status,
            QrcodePollStatus::Success | QrcodePollStatus::Expired
        ) {
            return None;
        }

        // Poll every 2 seconds
        let should_poll = self
            .last_poll
            .map(|t| t.elapsed() > Duration::from_secs(2))
            .unwrap_or(true);

        if !should_poll {
            return None;
        }

        self.last_poll = Some(Instant::now());

        match api_client.poll_qrcode(&qrcode_key).await {
            Ok(result) => {
                if let Some(data) = result.data {
                    self.poll_status = QrcodePollStatus::from(data.code);

                    if self.poll_status == QrcodePollStatus::Success {
                        // Extract credentials from cookies
                        if let Some(creds) =
                            Credentials::from_cookies(&result.cookies, Some(data.refresh_token))
                        {
                            return Some(AppAction::LoginSuccess(creds));
                        }
                    }
                }
            }
            Err(e) => {
                self.error_message = Some(format!("轮询失败: {}", e));
            }
        }

        None
    }

    fn status_text(&self, theme: &Theme) -> (&str, Color) {
        match self.poll_status {
            QrcodePollStatus::Waiting => ("⏳ 等待扫描二维码...", theme.warning),
            QrcodePollStatus::Scanned => ("📱 已扫描，请在手机上确认登录", theme.info),
            QrcodePollStatus::Success => ("✅ 登录成功！", theme.success),
            QrcodePollStatus::Expired => ("❌ 二维码已过期，请按 r 刷新", theme.error),
            QrcodePollStatus::Unknown(_) => ("❓ 未知状态", theme.fg_secondary),
        }
    }
}

impl Default for LoginPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for LoginPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Layout: title, QR code, status, help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(20),   // QR code
                Constraint::Length(3), // Status
                Constraint::Length(2), // Help
            ])
            .split(area);

        // Title with Bilibili branding
        let title_line = Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(
                "B",
                Style::default()
                    .fg(Color::Rgb(251, 114, 153))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "ilibili ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("登录", Style::default().fg(Color::Cyan)),
        ]);

        let title = Paragraph::new(title_line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 60, 60)))
                    .title(Span::styled(
                        " Login ",
                        Style::default()
                            .fg(theme.fg_accent)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);

        // QR code area
        let qr_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_unfocused))
            .title(Span::styled(
                " 扫码登录 ",
                Style::default().fg(theme.fg_secondary),
            ));

        if let Some(error) = &self.error_message {
            let error_widget = Paragraph::new(format!("❌ {}", error))
                .style(Style::default().fg(theme.error))
                .alignment(Alignment::Center)
                .block(qr_block);
            frame.render_widget(error_widget, chunks[1]);
        } else if let Some(qrcode_data) = &self.qrcode_data {
            frame.render_widget(qr_block.clone(), chunks[1]);
            let inner_area = qr_block.inner(chunks[1]);

            if let Ok(qr_code) = QrCode::new(&qrcode_data.url) {
                // Center the QR code
                let qr_area = centered_rect(60, 90, inner_area);
                let qr_widget = QrCodeWidget::new(qr_code);
                frame.render_widget(qr_widget, qr_area);
            }
        } else {
            let loading = Paragraph::new("⏳ 加载中...")
                .style(
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::ITALIC),
                )
                .alignment(Alignment::Center)
                .block(qr_block);
            frame.render_widget(loading, chunks[1]);
        }

        // Status with enhanced styling
        let (status_text, status_color) = self.status_text(theme);
        let status = Paragraph::new(status_text)
            .style(
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_unfocused))
                    .title(Span::styled(
                        " 状态 ",
                        Style::default().fg(theme.fg_secondary),
                    )),
            );
        frame.render_widget(status, chunks[2]);

        // Help with styled shortcuts
        let help_line = Line::from(vec![
            Span::styled(" [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                "r",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("刷新二维码", Style::default().fg(theme.fg_secondary)),
            Span::styled("  [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                "q",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("退出", Style::default().fg(theme.fg_secondary)),
        ]);
        let help = Paragraph::new(help_line).alignment(Alignment::Center);
        frame.render_widget(help, chunks[3]);
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction> {
        match key {
            KeyCode::Char('q') => Some(AppAction::Quit),
            KeyCode::Char('r') => {
                // Request refresh - will be handled by App
                self.qrcode_data = None;
                self.poll_status = QrcodePollStatus::Waiting;
                Some(AppAction::SwitchToLogin)
            }
            _ => Some(AppAction::None),
        }
    }
}

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
