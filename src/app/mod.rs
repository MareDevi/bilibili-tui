mod action;

pub use action::AppAction;

use crate::api::client::ApiClient;
use crate::storage::Credentials;
use crate::ui::{Component, DynamicPage, HomePage, LoginPage, NavItem, Page, SearchPage, Sidebar, VideoDetailPage};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    prelude::*,
    DefaultTerminal, Frame,
};
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Previous page for back navigation
#[derive(Clone)]
pub enum PreviousPage {
    Home,
    Search,
    Dynamic,
}

/// Main application state
pub struct App {
    pub current_page: Page,
    pub should_quit: bool,
    pub api_client: Arc<Mutex<ApiClient>>,
    pub credentials: Option<Credentials>,
    pub sidebar: Sidebar,
    pub show_sidebar: bool,
    pub previous_page: Option<PreviousPage>,
}

impl App {
    pub fn new() -> Self {
        let credentials = crate::storage::load_credentials().ok();
        let api_client = if let Some(ref creds) = credentials {
            ApiClient::with_cookies(creds)
        } else {
            ApiClient::new()
        };

        // Start on login page if no credentials, otherwise go to home
        let current_page = if credentials.is_some() {
            Page::Home(HomePage::new())
        } else {
            Page::Login(LoginPage::new())
        };

        Self {
            current_page,
            should_quit: false,
            api_client: Arc::new(Mutex::new(api_client)),
            credentials,
            sidebar: Sidebar::new(),
            show_sidebar: true,
            previous_page: None,
        }
    }

    /// Main run loop
    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        // Initialize the first page
        self.init_current_page().await;

        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_input(key.code).await;
                    }
                }
            }

            // Handle background tasks (like QR code polling)
            self.tick().await;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        
        // Login page and VideoDetail don't show sidebar
        if matches!(self.current_page, Page::Login(_) | Page::VideoDetail(_)) {
            match &mut self.current_page {
                Page::Login(page) => page.draw(frame, area),
                Page::VideoDetail(page) => page.draw(frame, area),
                _ => {}
            }
            return;
        }

        // Main layout with sidebar
        let chunks = if self.show_sidebar {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(16),  // Sidebar
                    Constraint::Min(40),     // Content
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(40)])
                .split(area)
        };

        if self.show_sidebar && chunks.len() > 1 {
            self.sidebar.draw(frame, chunks[0]);
            self.draw_page(frame, chunks[1]);
        } else {
            self.draw_page(frame, chunks[0]);
        }
    }

    fn draw_page(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.current_page {
            Page::Login(page) => page.draw(frame, area),
            Page::Home(page) => page.draw(frame, area),
            Page::Search(page) => page.draw(frame, area),
            Page::Dynamic(page) => page.draw(frame, area),
            Page::VideoDetail(page) => page.draw(frame, area),
        }
    }

    async fn handle_input(&mut self, key: KeyCode) {
        let action = match &mut self.current_page {
            Page::Login(page) => page.handle_input(key),
            Page::Home(page) => page.handle_input(key),
            Page::Search(page) => page.handle_input(key),
            Page::Dynamic(page) => page.handle_input(key),
            Page::VideoDetail(page) => page.handle_input(key),
        };

        if let Some(action) = action {
            self.handle_action(action).await;
        }
    }

    async fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::Quit => self.should_quit = true,
            AppAction::SwitchToHome => {
                self.sidebar.select(NavItem::Home);
                self.current_page = Page::Home(HomePage::new());
                self.init_current_page().await;
            }
            AppAction::SwitchToLogin => {
                self.current_page = Page::Login(LoginPage::new());
                self.init_current_page().await;
            }
            AppAction::LoginSuccess(creds) => {
                // Save credentials
                if let Err(e) = crate::storage::save_credentials(&creds) {
                    eprintln!("Failed to save credentials: {}", e);
                }
                self.credentials = Some(creds.clone());
                // Update API client with new cookies
                {
                    let client = self.api_client.lock().await;
                    client.set_credentials(&creds);
                }
                // Switch to home
                self.current_page = Page::Home(HomePage::new());
                self.init_current_page().await;
            }
            AppAction::PlayVideo(bvid) => {
                // Launch mpv player
                if let Err(e) = crate::player::play_video(&bvid, self.credentials.as_ref()).await {
                    eprintln!("Failed to play video: {}", e);
                }
            }
            AppAction::NavNext => {
                // Don't navigate if on video detail page
                if !matches!(self.current_page, Page::VideoDetail(_)) {
                    self.sidebar.next();
                    self.switch_to_nav_page().await;
                }
            }
            AppAction::NavPrev => {
                if !matches!(self.current_page, Page::VideoDetail(_)) {
                    self.sidebar.prev();
                    self.switch_to_nav_page().await;
                }
            }
            AppAction::Search(keyword) => {
                if let Page::Search(page) = &mut self.current_page {
                    let client = self.api_client.lock().await;
                    match client.search_videos(&keyword, 1).await {
                        Ok(data) => {
                            let results = data.result.unwrap_or_default();
                            let total = data.num_results.unwrap_or(0);
                            page.set_results(results, total);
                        }
                        Err(e) => {
                            page.set_error(format!("搜索失败: {}", e));
                        }
                    }
                }
            }
            AppAction::RefreshDynamic => {
                if let Page::Dynamic(page) = &mut self.current_page {
                    let client = self.api_client.lock().await;
                    match client.get_dynamic_feed(None).await {
                        Ok(data) => {
                            let items = data.items.unwrap_or_default();
                            let offset = data.offset;
                            let has_more = data.has_more.unwrap_or(false);
                            page.set_feed(items, offset, has_more);
                        }
                        Err(e) => {
                            page.set_error(format!("加载动态失败: {}", e));
                        }
                    }
                }
            }
            AppAction::OpenVideoDetail(bvid, aid) => {
                // Remember previous page
                self.previous_page = match &self.current_page {
                    Page::Home(_) => Some(PreviousPage::Home),
                    Page::Search(_) => Some(PreviousPage::Search),
                    Page::Dynamic(_) => Some(PreviousPage::Dynamic),
                    _ => None,
                };
                
                let mut detail_page = VideoDetailPage::new(bvid, aid);
                let client = self.api_client.lock().await;
                detail_page.load_data(&client).await;
                drop(client);
                self.current_page = Page::VideoDetail(detail_page);
            }
            AppAction::BackToList => {
                match self.previous_page.take() {
                    Some(PreviousPage::Home) => {
                        self.sidebar.select(NavItem::Home);
                        self.current_page = Page::Home(HomePage::new());
                        self.init_current_page().await;
                    }
                    Some(PreviousPage::Search) => {
                        self.sidebar.select(NavItem::Search);
                        self.current_page = Page::Search(SearchPage::new());
                    }
                    Some(PreviousPage::Dynamic) => {
                        self.sidebar.select(NavItem::Dynamic);
                        self.current_page = Page::Dynamic(DynamicPage::new());
                        self.init_current_page().await;
                    }
                    None => {
                        // Default to home
                        self.sidebar.select(NavItem::Home);
                        self.current_page = Page::Home(HomePage::new());
                        self.init_current_page().await;
                    }
                }
            }
            AppAction::LoadMoreRecommendations => {
                if let Page::Home(page) = &mut self.current_page {
                    let client = self.api_client.lock().await;
                    page.load_more(&client).await;
                }
            }
            AppAction::LoadMoreSearch => {
                if let Page::Search(page) = &mut self.current_page {
                    let client = self.api_client.lock().await;
                    page.load_more(&client).await;
                }
            }
            AppAction::LoadMoreDynamic => {
                if let Page::Dynamic(page) = &mut self.current_page {
                    let client = self.api_client.lock().await;
                    page.load_more(&client).await;
                }
            }
            AppAction::None => {}
        }
    }

    async fn switch_to_nav_page(&mut self) {
        match self.sidebar.selected {
            NavItem::Home => {
                if !matches!(self.current_page, Page::Home(_)) {
                    self.current_page = Page::Home(HomePage::new());
                    self.init_current_page().await;
                }
            }
            NavItem::Search => {
                if !matches!(self.current_page, Page::Search(_)) {
                    self.current_page = Page::Search(SearchPage::new());
                }
            }
            NavItem::Dynamic => {
                if !matches!(self.current_page, Page::Dynamic(_)) {
                    self.current_page = Page::Dynamic(DynamicPage::new());
                    self.init_current_page().await;
                }
            }
        }
    }

    async fn init_current_page(&mut self) {
        match &mut self.current_page {
            Page::Login(page) => {
                let client = self.api_client.lock().await;
                page.load_qrcode(&client).await;
            }
            Page::Home(page) => {
                let client = self.api_client.lock().await;
                page.load_recommendations(&client).await;
            }
            Page::Search(_) => {
                // Search page doesn't need initialization
            }
            Page::Dynamic(page) => {
                let client = self.api_client.lock().await;
                match client.get_dynamic_feed(None).await {
                    Ok(data) => {
                        let items = data.items.unwrap_or_default();
                        let offset = data.offset;
                        let has_more = data.has_more.unwrap_or(false);
                        page.set_feed(items, offset, has_more);
                    }
                    Err(e) => {
                        page.set_error(format!("加载动态失败: {}", e));
                    }
                }
            }
            Page::VideoDetail(_) => {
                // VideoDetail is initialized when created
            }
        }
    }

    async fn tick(&mut self) {
        match &mut self.current_page {
            Page::Login(page) => {
                let client = self.api_client.lock().await;
                if let Some(action) = page.tick(&client).await {
                    drop(client);
                    self.handle_action(action).await;
                }
            }
            Page::Home(page) => {
                // Non-blocking: poll completed downloads and start new ones
                page.poll_cover_results();
                page.start_cover_downloads();
            }
            Page::Search(page) => {
                page.poll_cover_results();
                page.start_cover_downloads();
            }
            Page::Dynamic(page) => {
                page.poll_cover_results();
                page.start_cover_downloads();
            }
            _ => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

