//! Login page with QR code display

use super::{Component, Theme};
use crate::api::auth::{QrcodeData, QrcodePollStatus};
use crate::api::client::ApiClient;
use crate::app::AppAction;
use crate::storage::{Credentials, Keybindings};
use qrcode::QrCode;
use ratatui::{crossterm::event::KeyCode, prelude::*, widgets::*};
use std::time::{Duration, Instant};
use tui_qrcode::{Colors, QrCodeWidget, QuietZone};

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
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
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
                "▌",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "B",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "ilibili ",
                Style::default()
                    .fg(theme.fg_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("登录", Style::default().fg(theme.bilibili_cyan)),
        ]);

        let title = Paragraph::new(title_line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_subtle))
                    .title(Span::styled(
                        " Login ",
                        Style::default()
                            .fg(theme.bilibili_pink)
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
                // Create QR code widget with optimized settings for scanning:
                // - Inverted colors: black modules on white background (standard QR format)
                // - QuietZone::Enabled: white border around QR for better scanning
                let qr_widget = QrCodeWidget::new(qr_code)
                    .colors(Colors::Inverted)
                    .quiet_zone(QuietZone::Enabled)
                    .style(Style::default().fg(Color::Black).bg(Color::White));

                // Get the actual size the QR code will render at
                let qr_size = qr_widget.size(inner_area);

                // Center the QR code based on its actual size
                let x_offset = (inner_area.width.saturating_sub(qr_size.width)) / 2;
                let y_offset = (inner_area.height.saturating_sub(qr_size.height)) / 2;

                let qr_area = Rect::new(
                    inner_area.x + x_offset,
                    inner_area.y + y_offset,
                    qr_size.width.min(inner_area.width),
                    qr_size.height.min(inner_area.height),
                );

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
                &keys.refresh,
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.fg_secondary)),
            Span::styled("刷新二维码", Style::default().fg(theme.fg_secondary)),
            Span::styled("  [", Style::default().fg(theme.fg_secondary)),
            Span::styled(
                &keys.quit,
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

    fn handle_input(
        &mut self,
        key: KeyCode,
        keys: &crate::storage::Keybindings,
    ) -> Option<AppAction> {
        if keys.matches_quit(key) {
            return Some(AppAction::Quit);
        }
        if keys.matches_refresh(key) {
            // Request refresh - will be handled by App
            self.qrcode_data = None;
            self.poll_status = QrcodePollStatus::Waiting;
            return Some(AppAction::SwitchToLogin);
        }
        Some(AppAction::None)
    }
}
