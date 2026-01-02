use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    prelude::*,
    widgets::*,
    DefaultTerminal,
};
use std::io;

trait Component {
    fn draw(&self, frame: &mut Frame, area: Rect);
    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction>;
}

enum AppAction {
    Quit,
    SwitchToSettings,
    SwitchToHome,
    None,
}

struct HomePage {
    counter: i32,
}
impl HomePage {
    fn new() -> Self { Self { counter: 0 } }
}
impl Component for HomePage {
    fn draw(&self, frame: &mut Frame, area: Rect) {
        let text = format!("【主页】\n计数: {}\n按 <J> 增加 | <Tab> 切换页面 | <Q> 退出", self.counter);
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
    fn new() -> Self { Self { is_active: false } }
}
impl Component for SettingsPage {
    fn draw(&self, frame: &mut Frame, area: Rect) {
        let state_text = if self.is_active { "开 (ON)" } else { "关 (OFF)" };
        let text = format!("【设置页】\n当前状态: {}\n按 <Space> 切换 | <Tab> 返回主页", state_text);
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
    Home(HomePage),
    Settings(SettingsPage),
}

pub struct App {
    current_page: Page,
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_page: Page::Home(HomePage::new()),
            should_quit: false,
        }
    }

    // 主运行循环
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_quit {
            // 1. 绘制
            terminal.draw(|frame| self.draw(frame))?;

            // 2. 处理事件
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    self.handle_input(key.code);
                }
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        match &self.current_page {
            Page::Home(page) => page.draw(frame, area),
            Page::Settings(page) => page.draw(frame, area),
        }
    }

    fn handle_input(&mut self, key: KeyCode) {
        // 先获取当前页面返回的 Action
        let action = match &mut self.current_page {
            Page::Home(page) => page.handle_input(key),
            Page::Settings(page) => page.handle_input(key),
        };

        // 再处理全局 Action
        if let Some(action) = action {
            match action {
                AppAction::Quit => self.should_quit = true,
                AppAction::SwitchToSettings => self.current_page = Page::Settings(SettingsPage::new()),
                AppAction::SwitchToHome => self.current_page = Page::Home(HomePage::new()),
                AppAction::None => {}
            }
        }
    }
}