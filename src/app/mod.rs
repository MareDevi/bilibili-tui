mod actions;
mod network_events;
mod runtime;

use crate::application::network;
use crate::domain::playback::{PlaybackEvent, PlaybackState};
use crate::infrastructure::{
    bilibili::{ApiClient, LiveDanmakuHub},
    persistence::{self, AppConfig, Credentials, Keybindings},
};
use crate::presentation::tui::{BangumiPage, DEFAULT_THEME_ID, HomePage, Page, Sidebar, Theme};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc;
use tokio::sync::watch;

#[derive(Default)]
struct RequestTracker {
    sequence: u64,
    pending: HashMap<&'static str, u64>,
}

impl RequestTracker {
    fn next(&mut self, key: &'static str) -> u64 {
        self.sequence = self.sequence.saturating_add(1);
        self.pending.insert(key, self.sequence);
        self.sequence
    }

    fn is_latest(&self, key: &'static str, request_id: u64) -> bool {
        self.pending
            .get(key)
            .is_some_and(|latest| *latest == request_id)
    }
}

/// Previous page for back navigation
#[derive(Clone)]
pub enum PreviousPage {
    Home,
    Search,
    Dynamic,
    History,
    Favorites,
    Live,
    Bangumi,
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
    /// Full page instances for nested detail navigation (list -> video -> UP).
    pub navigation_stack: Vec<Page>,
    pub theme: Theme,
    pub theme_id: String,
    pub config: AppConfig,
    pub keybindings: Keybindings,
    pub pending_home_notice: Option<String>,
    pub playback: PlaybackState,
    pub live_danmaku_hub: Option<Arc<LiveDanmakuHub>>,
    danmaku_config_tx: watch::Sender<crate::storage::DanmakuConfig>,
    playback_event_tx: mpsc::Sender<PlaybackEvent>,
    playback_event_rx: mpsc::Receiver<PlaybackEvent>,
    auto_return_after_playback: Option<(u64, String)>,
    next_playback_session_id: u64,
    pending_playlist: Option<(
        Vec<crate::domain::playback::PlaylistItem>,
        crate::domain::playback::PlaylistSource,
        usize,
        crate::domain::playback::PlayOrder,
    )>,

    /// Cached home page to avoid refresh when switching tabs
    pub cached_home: Option<HomePage>,
    /// Cached bangumi page to avoid refresh when switching tabs
    pub cached_bangumi: Option<BangumiPage>,
    network_command_tx: mpsc::Sender<network::NetworkCommand>,
    network_event_rx: mpsc::Receiver<network::NetworkEvent>,
    request_tracker: RequestTracker,
}

impl App {
    pub fn new() -> Self {
        let credentials = persistence::load_credentials().ok();
        let api_client = if let Some(ref creds) = credentials {
            ApiClient::with_cookies(creds)
        } else {
            ApiClient::new()
        };
        let api_client = Arc::new(api_client);
        let bridge = network::start_network_worker(api_client.clone());
        let (playback_event_tx, playback_event_rx) = mpsc::channel();

        // Load config and apply saved theme
        let config = persistence::load_config().unwrap_or_default();
        let (danmaku_config_tx, _) = watch::channel(config.danmaku.clone());
        let keybindings = config.keybindings.clone();
        let configured_theme_id = config.theme.clone();
        let (theme, used_fallback) = Theme::load_or_default(&configured_theme_id);
        let theme_id = if used_fallback {
            DEFAULT_THEME_ID.to_string()
        } else {
            configured_theme_id
        };

        let current_page = if config.minimal_mode {
            Page::Search(crate::presentation::tui::SearchPage::new())
        } else {
            Page::Home(HomePage::new())
        };
        let mut sidebar = Sidebar::new();
        sidebar.set_minimal_mode(config.minimal_mode);

        Self {
            current_page,
            should_quit: false,
            api_client,
            credentials,
            sidebar,
            show_sidebar: true,
            previous_page: None,
            navigation_stack: Vec::new(),
            theme,
            theme_id,
            config,
            keybindings,
            pending_home_notice: used_fallback
                .then_some("⚠ 旧主题配置无效，请前往设置页重新选择主题".to_string()),
            playback: PlaybackState::default(),
            live_danmaku_hub: None,
            danmaku_config_tx,
            playback_event_tx,
            playback_event_rx,
            auto_return_after_playback: None,
            next_playback_session_id: 1,
            pending_playlist: None,
            cached_home: None,
            cached_bangumi: None,
            network_command_tx: bridge.command_tx,
            network_event_rx: bridge.event_rx,
            request_tracker: RequestTracker::default(),
        }
    }

    fn next_request_id(&mut self, key: &'static str) -> u64 {
        self.request_tracker.next(key)
    }

    fn is_latest_request(&self, key: &'static str, req_id: u64) -> bool {
        self.request_tracker.is_latest(key, req_id)
    }

    fn send_network_command(&self, command: network::NetworkCommand) {
        let _ = self.network_command_tx.send(command);
    }

    fn allocate_playback_session(&mut self) -> u64 {
        let id = self.next_playback_session_id;
        self.next_playback_session_id = self.next_playback_session_id.saturating_add(1);
        id
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{App, RequestTracker};
    use crate::application::AppAction;
    use crate::domain::playback::PlaybackEvent;
    use crate::presentation::tui::{FavoritesPage, HomePage, NavItem, Page, VideoDetailPage};

    pub(super) fn minimal_app() -> (
        App,
        std::sync::mpsc::Receiver<crate::application::network::NetworkCommand>,
    ) {
        let mut app = App::new();
        app.config.minimal_mode = true;
        app.sidebar.set_minimal_mode(true);
        let (tx, rx) = std::sync::mpsc::channel();
        app.network_command_tx = tx;
        (app, rx)
    }

    #[tokio::test]
    async fn minimal_startup_and_back_navigation_do_not_request_recommendations() {
        let (mut app, rx) = minimal_app();
        app.current_page = Page::Home(HomePage::new());
        app.init_current_page().await;
        assert!(
            matches!(&app.current_page, Page::Search(page) if !page.show_hot_list && !page.hotword_loading)
        );
        app.handle_action(AppAction::BackToList).await;
        assert!(matches!(app.current_page, Page::Search(_)));
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn minimal_mode_rejects_feed_actions_before_sending_requests() {
        let (mut app, rx) = minimal_app();
        app.current_page = Page::Search(crate::presentation::tui::SearchPage::new());
        for action in [
            AppAction::SwitchToHome,
            AppAction::RefreshHome,
            AppAction::SwitchHomeFeed(crate::api::recommend::HomeFeed::Popular),
            AppAction::LoadMoreRecommendations,
            AppAction::SwitchToLive,
            AppAction::RefreshLive,
            AppAction::LoadMoreLive,
            AppAction::SwitchToBangumi,
            AppAction::RefreshBangumi,
            AppAction::LoadMoreBangumi,
        ] {
            app.handle_action(action).await;
            assert!(matches!(app.current_page, Page::Search(_)));
            assert!(rx.try_recv().is_err());
        }
    }

    #[tokio::test]
    async fn minimal_video_details_disable_related_requests_and_keep_search_navigation() {
        use crate::application::network::NetworkCommand;
        let (mut app, rx) = minimal_app();
        app.current_page = Page::Search(crate::presentation::tui::SearchPage::new());
        app.handle_action(AppAction::OpenVideoDetail("BV1test".into(), 1))
            .await;
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetworkCommand::LoadVideoDetail {
                include_related: false,
                ..
            }
        ));
        assert!(
            matches!(&app.current_page, Page::VideoDetail(page) if !page.recommendations_enabled)
        );
        app.handle_action(AppAction::BackToList).await;
        assert!(matches!(app.current_page, Page::Search(_)));
    }

    #[tokio::test]
    async fn video_detail_loaded_adopts_authoritative_aid_for_follow_up_requests() {
        use crate::application::network::NetworkEvent;
        let (mut app, _rx) = minimal_app();
        let req_id = app.next_request_id("video_detail");
        app.current_page =
            Page::VideoDetail(Box::new(VideoDetailPage::new("BV1test".to_string(), 0)));
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        app.network_event_rx = event_rx;
        event_tx
            .send(NetworkEvent::VideoDetailLoaded {
                req_id,
                bvid: "BV1test".to_string(),
                video_info: serde_json::from_str(
                    r#"{"bvid":"BV1test","aid":42,"cid":7,"title":"t","desc":null,"pic":null,
                        "duration":null,"pubdate":null,"owner":{"mid":1,"name":"u","face":""},
                        "stat":{"view":0,"danmaku":0,"like":0,"coin":0,"favorite":0,"share":0},
                        "pages":null}"#,
                )
                .expect("video info"),
                comments: Vec::new(),
                has_more_comments: false,
                related_videos: Vec::new(),
            })
            .expect("send event");

        app.drain_network_events();

        assert!(
            matches!(&app.current_page, Page::VideoDetail(page) if page.aid == 42),
            "loaded video info should replace the placeholder aid"
        );
    }

    #[tokio::test]
    async fn minimal_mode_keeps_explicit_search_requests() {
        let (mut app, rx) = minimal_app();
        app.current_page = Page::Search(crate::presentation::tui::SearchPage::new());
        app.init_current_page().await;
        app.handle_action(AppAction::Search("rust".into())).await;
        assert!(
            matches!(rx.try_recv().unwrap(), crate::application::network::NetworkCommand::Search { keyword, page: 1, .. } if keyword == "rust")
        );
    }

    #[tokio::test]
    async fn minimal_mode_keeps_followed_dynamics_and_own_space_accessible() {
        let (mut app, rx) = minimal_app();
        app.credentials = Some(crate::infrastructure::persistence::Credentials {
            sessdata: String::new(),
            bili_jct: String::new(),
            dede_user_id: "1".into(),
            dede_user_id_ckmd5: None,
            refresh_token: None,
        });
        app.sidebar.select(NavItem::Search);
        app.current_page = Page::Search(crate::presentation::tui::SearchPage::new());
        app.handle_action(AppAction::NavNext).await;
        assert!(matches!(app.current_page, Page::Dynamic(_)));
        assert!(matches!(
            rx.try_recv().unwrap(),
            crate::application::network::NetworkCommand::CancelPending
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            crate::application::network::NetworkCommand::LoadDynamicInit { .. }
        ));
        app.sidebar.select(NavItem::Favorites);
        app.handle_action(AppAction::NavNext).await;
        assert!(matches!(&app.current_page, Page::Up(page) if page.mid == 1));
        assert!(matches!(
            rx.try_recv().unwrap(),
            crate::application::network::NetworkCommand::CancelPending
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            crate::application::network::NetworkCommand::LoadUpPage { mid: 1, .. }
        ));
    }

    #[tokio::test]
    async fn disabling_minimal_mode_restores_home_and_hotword_requests() {
        let (mut app, rx) = minimal_app();
        app.current_page = Page::Search(crate::presentation::tui::SearchPage::new());
        app.apply_minimal_mode(false);
        assert!(matches!(
            rx.try_recv().unwrap(),
            crate::application::network::NetworkCommand::CancelPending
        ));
        app.handle_action(AppAction::SwitchToHome).await;
        assert!(matches!(app.current_page, Page::Home(_)));
        assert!(matches!(
            rx.try_recv().unwrap(),
            crate::application::network::NetworkCommand::LoadHome { .. }
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            crate::application::network::NetworkCommand::LoadHotwords { .. }
        ));
    }

    #[tokio::test]
    async fn enabling_minimal_mode_discards_cached_feeds_and_invalidates_pending_results() {
        let (mut app, rx) = minimal_app();
        let old_req_id = app.next_request_id("hotwords");
        app.cached_home = Some(HomePage::new());
        app.cached_bangumi = Some(crate::presentation::tui::BangumiPage::new());
        app.navigation_stack.push(Page::Home(HomePage::new()));
        app.previous_page = Some(super::PreviousPage::Home);
        app.apply_minimal_mode(true);
        assert!(app.cached_home.is_none() && app.cached_bangumi.is_none());
        assert!(app.navigation_stack.is_empty() && app.previous_page.is_none());
        assert!(!app.is_latest_request("hotwords", old_req_id));
        assert!(matches!(
            rx.try_recv().unwrap(),
            crate::application::network::NetworkCommand::CancelPending
        ));
        app.apply_minimal_mode(false);
        assert_eq!(app.sidebar.items(), NavItem::all());
    }

    #[test]
    fn request_tracking_latest_wins_per_key() {
        let mut tracker = RequestTracker::default();
        let first = tracker.next("search");
        let second = tracker.next("search");

        assert!(!tracker.is_latest("search", first));
        assert!(tracker.is_latest("search", second));
    }

    #[test]
    fn request_tracking_isolated_by_key() {
        let mut tracker = RequestTracker::default();
        let search_id = tracker.next("search");
        let home_id = tracker.next("home");

        assert!(tracker.is_latest("search", search_id));
        assert!(tracker.is_latest("home", home_id));
        assert!(!tracker.is_latest("home", search_id));
    }

    #[tokio::test]
    async fn tab_continues_from_favorites_to_live() {
        let mut app = App::new();
        app.sidebar.select(NavItem::Favorites);
        app.current_page = Page::Favorites(FavoritesPage::new(1));
        app.handle_action(AppAction::NavNext).await;
        assert_eq!(app.sidebar.selected, NavItem::Live);
        assert!(matches!(app.current_page, Page::Live(_)));
    }

    #[tokio::test]
    async fn completed_auto_play_returns_to_the_previous_page() {
        let mut app = App::new();
        app.navigation_stack.push(Page::Home(HomePage::new()));
        app.current_page =
            Page::VideoDetail(Box::new(VideoDetailPage::new("BV1test".to_string(), 1)));
        app.playback.begin_session(7);
        app.auto_return_after_playback = Some((7, "BV1test".to_string()));
        app.playback_event_tx
            .send(PlaybackEvent::Finished {
                session_id: 7,
                bvid: Some("BV1test".to_string()),
            })
            .unwrap();

        app.tick().await;

        assert!(matches!(app.current_page, Page::Home(_)));
        assert!(app.navigation_stack.is_empty());
        assert!(app.auto_return_after_playback.is_none());
    }

    #[tokio::test]
    async fn stale_playback_session_does_not_return() {
        let mut app = App::new();
        app.navigation_stack.push(Page::Home(HomePage::new()));
        app.current_page =
            Page::VideoDetail(Box::new(VideoDetailPage::new("BV1test".to_string(), 1)));
        app.playback.begin_session(8);
        app.auto_return_after_playback = Some((8, "BV1test".to_string()));
        app.playback_event_tx
            .send(PlaybackEvent::Finished {
                session_id: 7,
                bvid: Some("BV1test".to_string()),
            })
            .unwrap();
        app.tick().await;
        assert!(matches!(app.current_page, Page::VideoDetail(_)));
        assert_eq!(
            app.playback.status,
            crate::domain::playback::PlaybackStatus::Playing
        );
    }
}
