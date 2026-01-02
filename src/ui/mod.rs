use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode},
    prelude::*,
    widgets::*,
};
use std::io;

use qrcode::QrCode;
use ratatui::{Frame};
use tui_qrcode::QrCodeWidget;

use crate::api::client::{ApiClient, QrcodeData};

trait Component {
    fn draw(&self, frame: &mut Frame, area: Rect);
    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction>;
}

enum AppAction {
    Quit,
    SwitchToSettings,
    SwitchToHome,
    SwitchToLogin,
    None,
}

struct LoginPage {
    qrcode_data: Option<QrcodeData>,
    error_message: Option<String>,
}

impl LoginPage {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            qrcode_data: None,
            error_message: None,
        }
    }

    #[allow(dead_code)]
    async fn load_qrcode(&mut self, api_client: &ApiClient) {
        match api_client.get_qrcode_data().await {
            Ok(data) => {
                self.qrcode_data = Some(data);
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("获取二维码失败: {}", e));
            }
        }
    }
}

impl Component for LoginPage {
    fn draw(&self, frame: &mut Frame, area: Rect) {
        let [info_area, qr_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .areas(area);

        let title = Paragraph::new("【登录】\n使用二维码登录 Bilibili")
            .block(Block::default().borders(Borders::ALL).title("Login"))
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        frame.render_widget(title, info_area);

        if let Some(error) = &self.error_message {
            let error_widget = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            frame.render_widget(error_widget, qr_area);
        } else if let Some(qrcode_data) = &self.qrcode_data {
            if let Ok(qr_code) = QrCode::new(&qrcode_data.url) {
                let qr_widget = QrCodeWidget::new(qr_code);
                frame.render_widget(qr_widget, qr_area);
            }
        } else {
            let loading = Paragraph::new("加载中...")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            frame.render_widget(loading, qr_area);
        }
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction> {
        match key {
            KeyCode::Tab => Some(AppAction::SwitchToSettings),
            KeyCode::Char('q') => Some(AppAction::Quit),
            _ => Some(AppAction::None),
        }
    }
    
}

struct HomePage {
    counter: i32,
}
impl HomePage {
    fn new() -> Self {
        Self { counter: 0 }
    }
}
impl Component for HomePage {
    fn draw(&self, frame: &mut Frame, area: Rect) {
        let text = format!(
            "【主页】\n计数: {}\n按 <J> 增加 | <L> 登录 | <Tab> 切换页面 | <Q> 退出",
            self.counter
        );
        let p = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Home"))
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center);
        frame.render_widget(p, area);
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction> {
        match key {
            KeyCode::Char('j') => {
                self.counter += 1;
                Some(AppAction::None)
            }
            KeyCode::Char('l') => Some(AppAction::SwitchToLogin),
            KeyCode::Tab => Some(AppAction::SwitchToSettings),
            KeyCode::Char('q') => Some(AppAction::Quit),
            _ => Some(AppAction::None),
        }
    }
}

struct SettingsPage {
    is_active: bool,
}
impl SettingsPage {
    fn new() -> Self {
        Self { is_active: false }
    }
}
impl Component for SettingsPage {
    fn draw(&self, frame: &mut Frame, area: Rect) {
        let state_text = if self.is_active {
            "开 (ON)"
        } else {
            "关 (OFF)"
        };
        let text = format!(
            "【设置页】\n当前状态: {}\n按 <Space> 切换 | <Tab> 返回主页",
            state_text
        );
        let p = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Settings"))
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        frame.render_widget(p, area);
    }

    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction> {
        match key {
            KeyCode::Char(' ') => {
                self.is_active = !self.is_active;
                Some(AppAction::None)
            }
            KeyCode::Tab => Some(AppAction::SwitchToHome),
            KeyCode::Char('q') => Some(AppAction::Quit),
            _ => Some(AppAction::None),
        }
    }
}

enum Page {
    Login(LoginPage),
    Home(HomePage),
    Settings(SettingsPage),
}

pub struct App {
    current_page: Page,
    should_quit: bool,
    api_client: ApiClient,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_page: Page::Home(HomePage::new()),
            should_quit: false,
            api_client: ApiClient::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_login() -> Self {
        Self {
            current_page: Page::Login(LoginPage::new()),
            should_quit: false,
            api_client: ApiClient::new(),
        }
    }

    #[allow(dead_code)]
    pub async fn switch_to_login(&mut self) {
        let mut login_page = LoginPage::new();
        login_page.load_qrcode(&self.api_client).await;
        self.current_page = Page::Login(login_page);
    }

    // 主运行循环
    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_quit {
            // 1. 绘制
            terminal.draw(|frame| self.draw(frame))?;

            // 2. 处理事件
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    self.handle_input_async(key.code).await;
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn handle_input_sync(&mut self, _key: KeyCode) {
        // 保留用于未来扩展，当前不使用
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        match &self.current_page {
            Page::Login(page) => page.draw(frame, area),
            Page::Home(page) => page.draw(frame, area),
            Page::Settings(page) => page.draw(frame, area),
        }
    }

    async fn handle_input_async(&mut self, key: KeyCode) {
        // 先获取当前页面返回的 Action
        let action = match &mut self.current_page {
            Page::Login(page) => page.handle_input(key),
            Page::Home(page) => page.handle_input(key),
            Page::Settings(page) => page.handle_input(key),
        };

        // 再处理全局 Action
        if let Some(action) = action {
            match action {
                AppAction::Quit => self.should_quit = true,
                AppAction::SwitchToSettings => {
                    self.current_page = Page::Settings(SettingsPage::new())
                }
                AppAction::SwitchToHome => self.current_page = Page::Home(HomePage::new()),
                AppAction::SwitchToLogin => {
                    let mut login_page = LoginPage::new();
                    login_page.load_qrcode(&self.api_client).await;
                    self.current_page = Page::Login(login_page);
                }
                AppAction::None => {}
            }
        }
    }
}
