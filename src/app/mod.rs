mod action;

pub use action::AppAction;

use crate::api::client::ApiClient;
use crate::storage::{AppConfig, Credentials, Keybindings};
use crate::ui::{
    Component, DynamicPage, HistoryPage, HomePage, LiveDetailPage, LivePage, LoginPage, NavItem,
    Page, SearchPage, SettingsPage, Sidebar, Theme, ThemeVariant, VideoDetailPage,
};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEvent},
    prelude::*,
    DefaultTerminal, Frame,
};
use std::io;
use std::sync::Arc;

/// Previous page for back navigation
#[derive(Clone)]
pub enum PreviousPage {
    Home,
    Search,
    Dynamic,
    History,
    Live,
}

/// Main application state
pub struct App {
    pub current_page: Page,
    pub should_quit: bool,
    pub api_client: Arc<ApiClient>,
    pub credentials: Option<Credentials>,
    pub sidebar: Sidebar,
    pub show_sidebar: bool,

    pub previous_page: Option<PreviousPage>,
    pub theme: Theme,
    pub theme_variant: ThemeVariant,
    pub config: AppConfig,
    pub keybindings: Keybindings,

    /// Cached home page to avoid refresh when switching tabs
    pub cached_home: Option<HomePage>,
}

impl App {
    pub fn new() -> Self {
        let credentials = crate::storage::load_credentials().ok();
        let api_client = if let Some(ref creds) = credentials {
            ApiClient::with_cookies(creds)
        } else {
            ApiClient::new()
        };

        // Load config and apply saved theme
        let config = crate::storage::load_config().unwrap_or_default();
        let keybindings = config.keybindings.clone();
        let theme_variant = config
            .theme
            .parse()
            .unwrap_or(ThemeVariant::CatppuccinMocha);
        let theme = Theme::from_variant(theme_variant);

        // Start on login page if no credentials, otherwise go to home
        let current_page = if credentials.is_some() {
            Page::Home(HomePage::new())
        } else {
            Page::Login(LoginPage::new())
        };

        Self {
            current_page,
            should_quit: false,
            api_client: Arc::new(api_client),
            credentials,
            sidebar: Sidebar::new(),
            show_sidebar: true,
            previous_page: None,
            theme,
            theme_variant,
            config,
            keybindings,
            cached_home: None,
        }
    }

    /// 记录当前页面以便返回导航
    fn save_previous_page(&mut self) {
        self.previous_page = match &self.current_page {
            Page::Home(_) => Some(PreviousPage::Home),
            Page::Search(_) => Some(PreviousPage::Search),
            Page::Dynamic(_) => Some(PreviousPage::Dynamic),
            Page::History(_) => Some(PreviousPage::History),
            Page::Live(_) => Some(PreviousPage::Live),
            _ => None,
        };
    }

    /// Main run loop
    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        // Initialize the first page
        self.init_current_page().await;

        // Store the last content area for mouse handling
        let mut last_content_area = Rect::default();

        // Scroll accumulator for high-resolution mouse wheel throttling
        // Many modern mice generate multiple scroll events per physical "click"
        const SCROLL_THRESHOLD: i32 = 15; // Accumulate 15 events before scrolling
        let mut scroll_accumulator: i32 = 0;

        while !self.should_quit {
            terminal.draw(|frame| {
                last_content_area = self.get_content_area(frame.area());
                self.draw(frame);
            })?;

            if event::poll(std::time::Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            self.handle_input(key.code, key.modifiers).await;
                        }
                    }
                    Event::Mouse(mouse) => {
                        use crossterm::event::MouseEventKind;
                        match mouse.kind {
                            MouseEventKind::ScrollDown => {
                                scroll_accumulator += 1;
                                if scroll_accumulator >= SCROLL_THRESHOLD {
                                    scroll_accumulator = 0;
                                    self.handle_mouse(mouse, last_content_area).await;
                                }
                            }
                            MouseEventKind::ScrollUp => {
                                scroll_accumulator -= 1;
                                if scroll_accumulator <= -SCROLL_THRESHOLD {
                                    scroll_accumulator = 0;
                                    self.handle_mouse(mouse, last_content_area).await;
                                }
                            }
                            _ => {
                                // Other mouse events (clicks) are handled immediately
                                self.handle_mouse(mouse, last_content_area).await;
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Handle background tasks (like QR code polling)
            self.tick().await;
        }
        Ok(())
    }

    /// Get the content area excluding sidebar
    fn get_content_area(&self, area: Rect) -> Rect {
        // Login page, VideoDetail, and DynamicDetail use full area
        if matches!(
            self.current_page,
            Page::Login(_) | Page::VideoDetail(_) | Page::DynamicDetail(_)
        ) {
            return area;
        }

        // Main layout with sidebar
        if self.show_sidebar {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(16), // Sidebar
                    Constraint::Min(40),    // Content
                ])
                .split(area)[1]
        } else {
            area
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Login page, VideoDetail, and DynamicDetail don't show sidebar
        if matches!(
            self.current_page,
            Page::Login(_) | Page::VideoDetail(_) | Page::DynamicDetail(_)
        ) {
            match &mut self.current_page {
                Page::Login(page) => page.draw(frame, area, &self.theme, &self.keybindings),
                Page::VideoDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
                Page::DynamicDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
                _ => {}
            }
            return;
        }

        // Main layout with sidebar
        let chunks = if self.show_sidebar {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(16), // Sidebar
                    Constraint::Min(40),    // Content
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(40)])
                .split(area)
        };

        if self.show_sidebar && chunks.len() > 1 {
            self.sidebar.draw(frame, chunks[0], &self.theme);
            self.draw_page(frame, chunks[1]);
        } else {
            self.draw_page(frame, chunks[0]);
        }
    }

    fn draw_page(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.current_page {
            Page::Login(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Home(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Search(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Dynamic(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::DynamicDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::VideoDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::History(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Live(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::LiveDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Settings(page) => page.draw(frame, area, &self.theme, &self.keybindings),
        }
    }

    async fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        let keys = &self.keybindings;
        let action = match &mut self.current_page {
            Page::Login(page) => page.handle_input(key, keys),
            Page::Home(page) => page.handle_input(key, keys),
            Page::Search(page) => page.handle_input(key, keys),
            Page::Dynamic(page) => page.handle_input_with_modifiers(key, modifiers, keys),
            Page::DynamicDetail(page) => page.handle_input(key, keys),
            Page::VideoDetail(page) => page.handle_input(key, keys),
            Page::History(page) => page.handle_input(key, keys),
            Page::Live(page) => page.handle_input(key, keys),
            Page::LiveDetail(page) => page.handle_input(key, keys),
            Page::Settings(page) => page.handle_input(key, keys),
        };

        if let Some(action) = action {
            self.handle_action(action).await;
        }
    }

    async fn handle_mouse(&mut self, event: MouseEvent, area: Rect) {
        let action = match &mut self.current_page {
            Page::Login(page) => page.handle_mouse(event, area),
            Page::Home(page) => page.handle_mouse(event, area),
            Page::Search(page) => page.handle_mouse(event, area),
            Page::Dynamic(page) => page.handle_mouse(event, area),
            Page::DynamicDetail(page) => page.handle_mouse(event, area),
            Page::VideoDetail(page) => page.handle_mouse(event, area),
            Page::History(page) => page.handle_mouse(event, area),
            Page::Live(page) => page.handle_mouse(event, area),
            Page::LiveDetail(page) => page.handle_mouse(event, area),
            Page::Settings(page) => page.handle_mouse(event, area),
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
                // Use cached home page if available
                if let Some(cached) = self.cached_home.take() {
                    self.current_page = Page::Home(cached);
                } else {
                    self.current_page = Page::Home(HomePage::new());
                    self.init_current_page().await;
                }
            }
            AppAction::RefreshHome => {
                self.sidebar.select(NavItem::Home);
                // Clear cache and create fresh home page
                self.cached_home = None;
                self.current_page = Page::Home(HomePage::new());
                self.init_current_page().await;
            }
            AppAction::SwitchToLogin => {
                self.current_page = Page::Login(LoginPage::new());
                self.init_current_page().await;
            }
            AppAction::LoginSuccess(creds) => {
                // Save credentials
                let _ = crate::storage::save_credentials(&creds);
                self.credentials = Some(creds.clone());
                // Update API client with new cookies
                {
                    let client = self.api_client.clone();
                    client.set_credentials(&creds);
                }
                // Switch to home
                self.current_page = Page::Home(HomePage::new());
                self.init_current_page().await;
            }
            AppAction::PlayVideo {
                bvid,
                aid,
                cid,
                duration,
            } => {
                let api_client = self.api_client.clone();
                let _ = crate::player::play_video(
                    api_client,
                    &bvid,
                    aid,
                    cid,
                    duration,
                    None,
                    self.credentials.as_ref(),
                )
                .await;
            }
            AppAction::PlayVideoWithPages {
                bvid,
                aid,
                pages,
                current_index,
            } => {
                // Play only the selected episode
                if current_index < pages.len() {
                    let page = &pages[current_index];
                    let api_client = self.api_client.clone();
                    let _ = crate::player::play_video(
                        api_client,
                        &bvid,
                        aid,
                        page.cid,
                        page.duration,
                        Some(page.page),
                        self.credentials.as_ref(),
                    )
                    .await;
                    // Update current page index in video detail page
                    if let Page::VideoDetail(detail_page) = &mut self.current_page {
                        if detail_page.bvid == bvid {
                            detail_page.current_page_index = current_index;
                        }
                    }
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
                    let client = self.api_client.clone();
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
                    let client = self.api_client.clone();
                    let feed_type = page.current_tab.get_feed_type();
                    let host_mid = page.get_selected_up_mid();
                    match client.get_dynamic_feed(None, feed_type, host_mid).await {
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
                self.save_previous_page();
                // Cache home page before navigating to video detail
                if let Page::Home(home_page) =
                    std::mem::replace(&mut self.current_page, Page::Home(HomePage::new()))
                {
                    self.cached_home = Some(home_page);
                }
                let mut detail_page = VideoDetailPage::new(bvid, aid);
                let client = &self.api_client;
                detail_page.load_data(client).await;
                self.current_page = Page::VideoDetail(Box::new(detail_page));
            }
            AppAction::OpenDynamicDetail(dynamic_id) => {
                self.save_previous_page();
                // Cache home page before navigating to dynamic detail
                if let Page::Home(home_page) =
                    std::mem::replace(&mut self.current_page, Page::Home(HomePage::new()))
                {
                    self.cached_home = Some(home_page);
                }
                use crate::ui::DynamicDetailPage;
                let mut detail_page = DynamicDetailPage::new(dynamic_id);
                let client = &self.api_client;
                detail_page.load_data(client).await;
                self.current_page = Page::DynamicDetail(Box::new(detail_page));
            }
            AppAction::BackToList => {
                match self.previous_page.take() {
                    Some(PreviousPage::Home) => {
                        self.sidebar.select(NavItem::Home);
                        // Use cached home page if available
                        if let Some(cached) = self.cached_home.take() {
                            self.current_page = Page::Home(cached);
                        } else {
                            self.current_page = Page::Home(HomePage::new());
                            self.init_current_page().await;
                        }
                    }
                    Some(PreviousPage::Search) => {
                        self.sidebar.select(NavItem::Search);
                        self.current_page = Page::Search(SearchPage::new());
                        self.init_current_page().await;
                    }
                    Some(PreviousPage::Dynamic) => {
                        self.sidebar.select(NavItem::Dynamic);
                        self.current_page = Page::Dynamic(DynamicPage::new());
                        self.init_current_page().await;
                    }
                    Some(PreviousPage::History) => {
                        self.sidebar.select(NavItem::History);
                        self.current_page = Page::History(HistoryPage::new());
                        self.init_current_page().await;
                    }
                    Some(PreviousPage::Live) => {
                        self.sidebar.select(NavItem::Live);
                        self.current_page = Page::Live(LivePage::new());
                        self.init_current_page().await;
                    }
                    None => {
                        // Default to home
                        self.sidebar.select(NavItem::Home);
                        if let Some(cached) = self.cached_home.take() {
                            self.current_page = Page::Home(cached);
                        } else {
                            self.current_page = Page::Home(HomePage::new());
                            self.init_current_page().await;
                        }
                    }
                }
            }
            AppAction::LoadMoreRecommendations => {
                if let Page::Home(page) = &mut self.current_page {
                    let client = self.api_client.clone();
                    page.load_more(&client).await;
                }
            }
            AppAction::LoadMoreSearch => {
                if let Page::Search(page) = &mut self.current_page {
                    let client = self.api_client.clone();
                    page.load_more(&client).await;
                }
            }
            AppAction::LoadMoreDynamic => {
                if let Page::Dynamic(page) = &mut self.current_page {
                    let client = self.api_client.clone();
                    page.load_more(&client).await;
                }
            }
            AppAction::LoadMoreHistory => {
                if let Page::History(page) = &mut self.current_page {
                    let client = self.api_client.clone();
                    page.load_more(&client).await;
                }
            }
            AppAction::SwitchToHistory => {
                self.sidebar.select(NavItem::History);
                self.current_page = Page::History(HistoryPage::new());
                self.init_current_page().await;
            }
            AppAction::LoadMoreComments => {
                if let Page::VideoDetail(page) = &mut self.current_page {
                    let client = self.api_client.clone();
                    page.load_more_comments(&client).await;
                } else if let Page::DynamicDetail(page) = &mut self.current_page {
                    let client = self.api_client.clone();
                    page.load_more_comments(&client).await;
                }
            }
            AppAction::ToggleCommentReplies => {
                if let Page::VideoDetail(page) = &mut self.current_page {
                    let client = self.api_client.clone();
                    page.toggle_comment_replies(&client).await;
                }
            }
            AppAction::SwitchDynamicTab(tab) => {
                if let Page::Dynamic(page) = &mut self.current_page {
                    page.switch_tab(tab);
                    let client = self.api_client.clone();
                    let feed_type = page.current_tab.get_feed_type();
                    let host_mid = page.get_selected_up_mid();
                    match client.get_dynamic_feed(None, feed_type, host_mid).await {
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
            AppAction::SelectUpMaster(index) => {
                if let Page::Dynamic(page) = &mut self.current_page {
                    page.select_up(index);
                    let client = self.api_client.clone();
                    let feed_type = page.current_tab.get_feed_type();
                    let host_mid = page.get_selected_up_mid();
                    match client.get_dynamic_feed(None, feed_type, host_mid).await {
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
            AppAction::NextTheme => {
                self.theme_variant = self.theme_variant.next();
                self.theme = Theme::from_variant(self.theme_variant);
                self.save_theme_to_config();
            }
            AppAction::SetTheme(variant) => {
                self.theme_variant = variant;
                self.theme = Theme::from_variant(variant);
                self.save_theme_to_config();
            }
            AppAction::SwitchToSettings => {
                self.sidebar.select(NavItem::Settings);
                let page = SettingsPage::new(self.keybindings.clone(), self.theme_variant);
                self.current_page = Page::Settings(Box::new(page));
            }
            AppAction::Logout => {
                let _ = crate::storage::delete_credentials();
                self.credentials = None;
                self.current_page = Page::Login(LoginPage::new());
                self.init_current_page().await;
            }
            AppAction::LikeComment {
                oid,
                rpid,
                comment_type,
            } => {
                let client = self.api_client.clone();
                // Toggle like - if already liked, unlike
                if let Page::VideoDetail(page) = &mut self.current_page {
                    let is_liked = page.liked_comments.contains(&rpid);
                    if let Ok(()) = client
                        .like_comment(oid, rpid, comment_type, !is_liked)
                        .await
                    {
                        if is_liked {
                            page.liked_comments.remove(&rpid);
                        } else {
                            page.liked_comments.insert(rpid);
                        }
                    }
                } else if let Page::DynamicDetail(page) = &mut self.current_page {
                    let is_liked = page.liked_comments.contains(&rpid);
                    if let Ok(()) = client
                        .like_comment(oid, rpid, comment_type, !is_liked)
                        .await
                    {
                        if is_liked {
                            page.liked_comments.remove(&rpid);
                        } else {
                            page.liked_comments.insert(rpid);
                        }
                    }
                }
            }
            AppAction::AddComment {
                oid,
                comment_type,
                message,
                root,
            } => {
                let client = self.api_client.clone();
                if let Ok(_response) = client
                    .add_comment(oid, comment_type, &message, root, root)
                    .await
                {
                    // Reload comments to show new comment
                    if let Page::VideoDetail(page) = &mut self.current_page {
                        page.load_data(&client).await;
                    } else if let Page::DynamicDetail(page) = &mut self.current_page {
                        page.load_data(&client).await;
                    }
                }
            }
            AppAction::SaveKeybindings(new_keybindings) => {
                self.keybindings = (*new_keybindings).clone();
                self.config.keybindings = *new_keybindings;
                let _ = crate::storage::save_config(&self.config);
            }
            AppAction::SwitchToLive => {
                self.sidebar.select(NavItem::Live);
                self.current_page = Page::Live(LivePage::new());
                self.init_current_page().await;
            }
            AppAction::OpenLiveDetail(room_id) => {
                self.save_previous_page();
                let mut detail_page = LiveDetailPage::new(room_id);
                let client = &self.api_client;
                detail_page.load_room_info(client).await;
                // Connect WebSocket for real-time messages
                let uid = self
                    .credentials
                    .as_ref()
                    .and_then(|c| c.dede_user_id.parse::<i64>().ok())
                    .unwrap_or(0);
                detail_page.connect_ws(client, uid).await;
                self.current_page = Page::LiveDetail(Box::new(detail_page));
            }
            AppAction::RefreshLive => {
                if let Page::Live(page) = &mut self.current_page {
                    let client = &self.api_client;
                    page.refresh(client).await;
                }
            }
            AppAction::LoadMoreLive => {
                if let Page::Live(page) = &mut self.current_page {
                    let client = &self.api_client;
                    page.load_more(client).await;
                }
            }
            AppAction::PlayLive { room_id, title: _ } => {
                let _ = crate::player::play_live(room_id).await;
            }
            AppAction::None => {}
        }
    }

    async fn switch_to_nav_page(&mut self) {
        // First, cache home page if we're leaving it
        if matches!(self.current_page, Page::Home(_)) && self.sidebar.selected != NavItem::Home {
            if let Page::Home(home_page) =
                std::mem::replace(&mut self.current_page, Page::Home(HomePage::new()))
            {
                self.cached_home = Some(home_page);
            }
        }

        match self.sidebar.selected {
            NavItem::Home => {
                if !matches!(self.current_page, Page::Home(_)) {
                    // Use cached home page if available
                    if let Some(cached) = self.cached_home.take() {
                        self.current_page = Page::Home(cached);
                    } else {
                        self.current_page = Page::Home(HomePage::new());
                        self.init_current_page().await;
                    }
                }
            }
            NavItem::Search => {
                if !matches!(self.current_page, Page::Search(_)) {
                    self.current_page = Page::Search(SearchPage::new());
                    self.init_current_page().await;
                }
            }
            NavItem::Dynamic => {
                if !matches!(self.current_page, Page::Dynamic(_)) {
                    self.current_page = Page::Dynamic(DynamicPage::new());
                    self.init_current_page().await;
                }
            }
            NavItem::History => {
                if !matches!(self.current_page, Page::History(_)) {
                    self.current_page = Page::History(HistoryPage::new());
                    self.init_current_page().await;
                }
            }
            NavItem::Settings => {
                if !matches!(self.current_page, Page::Settings(_)) {
                    let page = SettingsPage::new(self.keybindings.clone(), self.theme_variant);
                    self.current_page = Page::Settings(Box::new(page));
                }
            }
            NavItem::Live => {
                if !matches!(self.current_page, Page::Live(_)) {
                    self.current_page = Page::Live(LivePage::new());
                    self.init_current_page().await;
                }
            }
        }
    }

    async fn init_current_page(&mut self) {
        match &mut self.current_page {
            Page::Login(page) => {
                let client = self.api_client.clone();
                page.load_qrcode(&client).await;
            }
            Page::Home(page) => {
                let client = self.api_client.clone();
                page.load_recommendations(&client).await;
            }
            Page::Search(page) => {
                let client = self.api_client.clone();
                page.start_hotword_loading();

                match client.get_hot_search().await {
                    Ok(list) => page.set_hotwords(list),
                    Err(e) => page.set_hotword_error(format!("加载热搜失败: {}", e)),
                }
            }
            Page::Dynamic(page) => {
                let client = self.api_client.clone();

                // First load portal to get frequently watched UPs
                page.loading_up_list = true;
                match client.get_dynamic_portal().await {
                    Ok(portal) => {
                        if let Some(up_list) = portal.up_list {
                            page.set_up_list(up_list);
                        }
                    }
                    Err(_) => {
                        page.loading_up_list = false;
                    }
                }

                // Then load dynamic feed
                let feed_type = page.current_tab.get_feed_type();
                let host_mid = page.get_selected_up_mid();
                match client.get_dynamic_feed(None, feed_type, host_mid).await {
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
            Page::DynamicDetail(_) => {
                // DynamicDetail is initialized when created
            }
            Page::History(page) => {
                let client = self.api_client.clone();
                page.load_history(&client).await;
            }
            Page::Live(page) => {
                let client = self.api_client.clone();
                page.load_recommendations(&client).await;
            }
            Page::LiveDetail(page) => {
                let client = self.api_client.clone();
                page.load_room_info(&client).await;
            }
            Page::Settings(_) => {
                // Settings doesn't need async initialization
            }
        }
    }

    async fn tick(&mut self) {
        match &mut self.current_page {
            Page::Login(page) => {
                let client = &self.api_client;
                if let Some(action) = page.tick(client).await {
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
            Page::VideoDetail(page) => {
                page.poll_cover_results();
                page.start_cover_downloads();
            }
            Page::History(page) => {
                page.poll_cover_results();
                page.start_cover_downloads();
            }
            _ => {}
        }
    }

    fn save_theme_to_config(&mut self) {
        self.config.theme = self.theme_variant.to_string();
        if crate::storage::save_config(&self.config).is_err() {}
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
