use crate::app::App;
use crate::presentation::tui::{Component, Page};
use crossterm::event::MouseEventKind;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEvent},
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::io;

impl App {
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
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_input(key.code, key.modifiers).await;
                    }
                    Event::Mouse(mouse) => match mouse.kind {
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
                    },
                    _ => {}
                }
            }

            // Handle background tasks (like QR code polling)
            self.tick().await;
        }
        Ok(())
    }

    fn current_page_uses_full_area(&self) -> bool {
        match &self.current_page {
            Page::Login(_)
            | Page::VideoDetail(_)
            | Page::DynamicDetail(_)
            | Page::ArticleDetail(_)
            | Page::BangumiDetail(_) => true,
            Page::Up(page) => !page.is_sidebar_page,
            _ => false,
        }
    }

    /// Get the content area excluding sidebar
    fn get_content_area(&self, area: Rect) -> Rect {
        if self.current_page_uses_full_area() {
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

        if self.current_page_uses_full_area() {
            match &mut self.current_page {
                Page::Login(page) => page.draw(frame, area, &self.theme, &self.keybindings),
                Page::VideoDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
                Page::DynamicDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
                Page::ArticleDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
                Page::BangumiDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
                Page::Up(page) => page.draw(frame, area, &self.theme, &self.keybindings),
                _ => {}
            }
            self.draw_playback_error(frame, area);
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
        self.draw_playback_error(frame, area);
    }

    fn draw_playback_error(&self, frame: &mut Frame, area: Rect) {
        let Some(error) = self.playback.last_error.as_deref() else {
            return;
        };
        let popup = Rect {
            x: area.x,
            y: area.bottom().saturating_sub(3),
            width: area.width,
            height: 3.min(area.height),
        };
        let message = Paragraph::new(error)
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title(" 播放错误 "));
        frame.render_widget(message, popup);
    }

    fn draw_page(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.current_page {
            Page::Login(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Home(page) => {
                if let Some(notice) = self.pending_home_notice.take() {
                    page.set_footer_notice(notice);
                }
                page.draw(frame, area, &self.theme, &self.keybindings);
            }
            Page::Search(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Dynamic(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::DynamicDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::ArticleDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::VideoDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::History(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Favorites(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Live(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::LiveDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Settings(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Bangumi(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::BangumiDetail(page) => page.draw(frame, area, &self.theme, &self.keybindings),
            Page::Up(page) => page.draw(frame, area, &self.theme, &self.keybindings),
        }
    }

    async fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        // Playback errors remain visible until the user acknowledges them with
        // the next key press; the key still performs its normal action.
        self.playback.last_error = None;
        let keys = &self.keybindings;
        let action = match &mut self.current_page {
            Page::Login(page) => page.handle_input(key, keys),
            Page::Home(page) => page.handle_input(key, keys),
            Page::Search(page) => page.handle_input(key, keys),
            Page::Dynamic(page) => page.handle_input_with_modifiers(key, modifiers, keys),
            Page::DynamicDetail(page) => page.handle_input(key, keys),
            Page::ArticleDetail(page) => page.handle_input(key, keys),
            Page::VideoDetail(page) => page.handle_input(key, keys),
            Page::History(page) => page.handle_input_with_modifiers(key, modifiers, keys),
            Page::Favorites(page) => page.handle_input(key, keys),
            Page::Live(page) => page.handle_input(key, keys),
            Page::LiveDetail(page) => page.handle_input(key, keys),
            Page::Settings(page) => page.handle_input(key, keys),
            Page::Bangumi(page) => page.handle_input(key, keys),
            Page::BangumiDetail(page) => page.handle_input(key, keys),
            Page::Up(page) => page.handle_input(key, keys),
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
            Page::ArticleDetail(page) => page.handle_mouse(event, area),
            Page::VideoDetail(page) => page.handle_mouse(event, area),
            Page::History(page) => page.handle_mouse(event, area),
            Page::Favorites(page) => page.handle_mouse(event, area),
            Page::Live(page) => page.handle_mouse(event, area),
            Page::LiveDetail(page) => page.handle_mouse(event, area),
            Page::Settings(page) => page.handle_mouse(event, area),
            Page::Bangumi(page) => page.handle_mouse(event, area),
            Page::BangumiDetail(page) => page.handle_mouse(event, area),
            Page::Up(page) => page.handle_mouse(event, area),
        };

        if let Some(action) = action {
            self.handle_action(action).await;
        }
    }

    pub(super) async fn tick(&mut self) {
        self.drain_network_events();
        if let Some((items, source, start_index, order)) = self.pending_playlist.take() {
            self.start_playlist(items, source, start_index, order).await;
        }
        while let Ok(event) = self.playback_event_rx.try_recv() {
            let accepted = self.playback.apply_event(&event);
            match event {
                crate::domain::playback::PlaybackEvent::Finished {
                    session_id,
                    bvid: Some(bvid),
                } => {
                    if accepted
                        && self.auto_return_after_playback.as_ref()
                            == Some(&(session_id, bvid.clone()))
                        && matches!(&self.current_page, Page::VideoDetail(page) if page.bvid == bvid)
                    {
                        self.auto_return_after_playback = None;
                        self.handle_action(crate::application::AppAction::BackToList)
                            .await;
                    } else if self
                        .auto_return_after_playback
                        .as_ref()
                        .is_some_and(|(id, _)| *id == session_id)
                    {
                        self.auto_return_after_playback = None;
                    }
                }
                crate::domain::playback::PlaybackEvent::Failed { session_id, .. }
                    if accepted
                        && self
                            .auto_return_after_playback
                            .as_ref()
                            .is_some_and(|(id, _)| *id == session_id) =>
                {
                    self.auto_return_after_playback = None;
                }
                _ => {}
            }
        }

        let auto_play = if self.config.auto_play {
            match &mut self.current_page {
                Page::VideoDetail(page)
                    if page.auto_play_pending && !page.loading && page.video_info.is_some() =>
                {
                    page.auto_play_pending = false;
                    Some((Some(page.bvid.clone()), page.play_action()))
                }
                Page::BangumiDetail(page)
                    if page.auto_play_pending && !page.loading && page.season.is_some() =>
                {
                    page.auto_play_pending = false;
                    page.play_action().map(|action| (None, action))
                }
                _ => None,
            }
        } else {
            // Auto-play disabled: clear the pending flag so it doesn't fire
            // once the user re-enables the setting mid-session.
            if let Page::VideoDetail(page) = &mut self.current_page {
                page.auto_play_pending = false;
            }
            if let Page::BangumiDetail(page) = &mut self.current_page {
                page.auto_play_pending = false;
            }
            None
        };
        if let Some((return_bvid, action)) = auto_play {
            self.handle_action(action).await;
            if let (Some(bvid), Some(session_id)) = (return_bvid, self.playback.session_id) {
                self.auto_return_after_playback = Some((session_id, bvid));
            }
        }
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
            Page::ArticleDetail(page) => {
                page.poll_image_results();
                page.start_image_downloads();
            }
            Page::History(page) => {
                page.poll_cover_results();
                page.start_cover_downloads();
            }
            Page::Favorites(page) => {
                page.videos.poll_cover_results();
                page.videos.start_cover_downloads();
            }
            Page::Bangumi(page) => {
                page.rank_grid.poll_cover_results();
                page.rank_grid.start_cover_downloads();
                page.index_grid.poll_cover_results();
                page.index_grid.start_cover_downloads();
            }
            Page::Up(page) => {
                page.videos.poll_cover_results();
                page.videos.start_cover_downloads();
                page.favorite_videos.poll_cover_results();
                page.favorite_videos.start_cover_downloads();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::minimal_app;
    use crate::application::AppAction;
    use crate::presentation::tui::{NavItem, SettingsPage, UpPage};

    fn logged_in_minimal_app() -> App {
        let (mut app, _commands) = minimal_app();
        app.credentials = Some(crate::infrastructure::persistence::Credentials {
            sessdata: String::new(),
            bili_jct: String::new(),
            dede_user_id: "1".into(),
            dede_user_id_ckmd5: None,
            refresh_token: None,
        });
        app
    }

    fn select_own_space(app: &mut App) {
        app.sidebar.select(NavItem::MySpace);
        app.current_page = Page::Up(Box::new(UpPage::new_sidebar(1)));
    }

    #[tokio::test]
    async fn minimal_sidebar_tab_cycle_reaches_settings_after_own_space() {
        let mut app = logged_in_minimal_app();
        app.sidebar.select(NavItem::Settings);
        app.current_page =
            Page::Settings(Box::new(SettingsPage::default().with_minimal_mode(true)));
        for expected in [
            NavItem::Search,
            NavItem::Dynamic,
            NavItem::History,
            NavItem::Favorites,
            NavItem::MySpace,
            NavItem::Settings,
        ] {
            app.handle_input(KeyCode::Tab, KeyModifiers::NONE).await;
            assert_eq!(app.sidebar.selected, expected);
        }
        assert!(matches!(app.current_page, Page::Settings(_)));
    }

    #[tokio::test]
    async fn minimal_sidebar_reverse_tab_crosses_own_space_while_loading() {
        let mut app = logged_in_minimal_app();
        app.sidebar.select(NavItem::Settings);
        app.current_page = Page::Settings(Box::default());
        app.handle_input(KeyCode::BackTab, KeyModifiers::SHIFT)
            .await;
        assert!(
            matches!(&app.current_page, Page::Up(page) if page.is_sidebar_page && page.loading)
        );
        app.handle_input(KeyCode::BackTab, KeyModifiers::SHIFT)
            .await;
        assert_eq!(app.sidebar.selected, NavItem::Favorites);
        assert!(matches!(app.current_page, Page::Favorites(_)));
    }

    #[tokio::test]
    async fn minimal_own_space_escape_keeps_sidebar_navigation_reachable() {
        let mut app = logged_in_minimal_app();
        select_own_space(&mut app);
        app.handle_input(KeyCode::Esc, KeyModifiers::NONE).await;
        assert_eq!(app.sidebar.selected, NavItem::Favorites);
        app.handle_input(KeyCode::Tab, KeyModifiers::NONE).await;
        app.handle_input(KeyCode::Tab, KeyModifiers::NONE).await;
        assert_eq!(app.sidebar.selected, NavItem::Settings);
        assert!(matches!(app.current_page, Page::Settings(_)));
    }

    #[tokio::test]
    async fn minimal_video_back_restores_own_space_before_tab_to_settings() {
        let mut app = logged_in_minimal_app();
        select_own_space(&mut app);
        app.handle_action(AppAction::OpenVideoDetail("BV1test".into(), 1))
            .await;
        app.handle_input(KeyCode::Esc, KeyModifiers::NONE).await;
        assert!(matches!(&app.current_page, Page::Up(page) if page.is_sidebar_page));
        assert_eq!(app.sidebar.selected, NavItem::MySpace);
        app.handle_input(KeyCode::Tab, KeyModifiers::NONE).await;
        assert!(matches!(app.current_page, Page::Settings(_)));
    }

    #[tokio::test]
    async fn minimal_own_space_error_keeps_configured_navigation_keys_working() {
        let mut app = logged_in_minimal_app();
        select_own_space(&mut app);
        if let Page::Up(page) = &mut app.current_page {
            page.set_error("加载失败".into());
        }
        app.keybindings.nav_next_page = "n".into();
        app.handle_input(KeyCode::Char('n'), KeyModifiers::NONE)
            .await;
        assert_eq!(app.sidebar.selected, NavItem::Settings);
        assert!(matches!(app.current_page, Page::Settings(_)));
    }

    #[tokio::test]
    async fn minimal_own_space_draw_keeps_sidebar_and_content_area_consistent() {
        let mut app = logged_in_minimal_app();
        select_own_space(&mut app);
        let area = Rect::new(0, 0, 100, 30);
        assert_eq!(app.get_content_area(area), Rect::new(16, 0, 84, 30));
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(content.replace(' ', "").contains("我的主页"));
        assert!(content.replace(' ', "").contains("设置"));
        app.current_page = Page::Up(Box::new(UpPage::new(1)));
        assert_eq!(app.get_content_area(area), area);
    }
}
