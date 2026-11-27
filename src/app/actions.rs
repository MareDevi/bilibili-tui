use crate::app::{App, PreviousPage};
use crate::application::{AppAction, network};
use crate::infrastructure::{media, persistence};
use crate::presentation::tui::{
    ArticleDetailPage, BangumiDetailPage, BangumiPage, DynamicDetailPage, DynamicPage,
    FavoritesPage, HistoryPage, HomePage, LiveDetailPage, LivePage, LoginPage, NavItem, Page,
    SearchPage, SettingsPage, Theme, UpPage,
};
use std::sync::Arc;

impl App {
    fn login_required_message() -> String {
        "该功能需要登录，请前往设置页登录".to_string()
    }

    fn apply_login_required_hint(&mut self) {
        let msg = Self::login_required_message();
        match &mut self.current_page {
            Page::Dynamic(page) => {
                page.loading_up_list = false;
                page.set_error(msg);
            }
            Page::History(page) => page.apply_load_more_error(msg),
            Page::VideoDetail(page) => {
                page.error_message = Some(msg);
                page.loading = false;
            }
            Page::DynamicDetail(page) => {
                page.error_message = Some(msg);
                page.loading = false;
            }
            _ => {}
        }
    }

    /// 记录当前页面以便返回导航
    fn save_previous_page(&mut self) {
        self.previous_page = match &self.current_page {
            Page::Home(_) => Some(PreviousPage::Home),
            Page::Search(_) => Some(PreviousPage::Search),
            Page::Dynamic(_) => Some(PreviousPage::Dynamic),
            Page::History(_) => Some(PreviousPage::History),
            Page::Favorites(_) => Some(PreviousPage::Favorites),
            Page::Live(_) => Some(PreviousPage::Live),
            Page::Bangumi(_) => Some(PreviousPage::Bangumi),
            _ => None,
        };
    }

    pub(super) async fn handle_action(&mut self, action: AppAction) {
        if self.config.minimal_mode
            && matches!(
                action,
                AppAction::SwitchToHome
                    | AppAction::RefreshHome
                    | AppAction::SwitchHomeFeed(_)
                    | AppAction::LoadMoreRecommendations
                    | AppAction::SwitchToLive
                    | AppAction::RefreshLive
                    | AppAction::LoadMoreLive
                    | AppAction::SwitchToBangumi
                    | AppAction::RefreshBangumi
                    | AppAction::LoadMoreBangumi
            )
        {
            return;
        }
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
            AppAction::SwitchHomeFeed(feed) => {
                let req_id = self.next_request_id("home");
                // Invalidate any pending pagination result before switching.
                self.next_request_id("home_more");
                let use_guest_feed = self.credentials.is_none();
                if let Page::Home(page) = &mut self.current_page {
                    page.begin_feed_load(feed);
                    self.send_network_command(network::NetworkCommand::LoadHome {
                        req_id,
                        feed,
                        use_guest_feed,
                    });
                }
            }
            AppAction::SwitchToLogin => {
                self.current_page = Page::Login(LoginPage::new());
                self.init_current_page().await;
            }
            AppAction::LoginSuccess(creds) => {
                // Save credentials
                let _ = persistence::save_credentials(&creds);
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
                let session_id = self.allocate_playback_session();
                self.playback.session_id = None;
                self.playback.status = crate::domain::playback::PlaybackStatus::Starting;
                let api_client = self.api_client.clone();
                match media::play_video(
                    api_client,
                    &bvid,
                    aid,
                    cid,
                    duration,
                    None,
                    self.credentials.as_ref(),
                    self.config.danmaku.clone(),
                    self.config.video_quality,
                    self.playback_event_tx.clone(),
                    session_id,
                )
                .await
                {
                    Ok(()) => {
                        self.playback.begin_session(session_id);
                    }
                    Err(error) => {
                        self.playback.status = crate::domain::playback::PlaybackStatus::Failed;
                        self.playback.last_error = Some(format!("启动播放器失败: {error:#}"));
                    }
                }
            }
            AppAction::PlayVideoWithPages {
                bvid,
                aid,
                pages,
                current_index,
            } => {
                // Play only the selected episode
                if current_index < pages.len() {
                    let session_id = self.allocate_playback_session();
                    self.playback.session_id = None;
                    self.playback.status = crate::domain::playback::PlaybackStatus::Starting;
                    let page = &pages[current_index];
                    let api_client = self.api_client.clone();
                    match media::play_video(
                        api_client,
                        &bvid,
                        aid,
                        page.cid,
                        page.duration,
                        Some(page.page),
                        self.credentials.as_ref(),
                        self.config.danmaku.clone(),
                        self.config.video_quality,
                        self.playback_event_tx.clone(),
                        session_id,
                    )
                    .await
                    {
                        Ok(()) => {
                            self.playback.begin_session(session_id);
                        }
                        Err(error) => {
                            self.playback.status = crate::domain::playback::PlaybackStatus::Failed;
                            self.playback.last_error = Some(format!("启动播放器失败: {error:#}"));
                        }
                    }
                    // Update current page index in video detail page
                    if let Page::VideoDetail(detail_page) = &mut self.current_page
                        && detail_page.bvid == bvid
                    {
                        detail_page.current_page_index = current_index;
                    }
                }
            }
            AppAction::PlayPlaylist {
                items,
                source,
                start_index,
                order,
            } => self.start_playlist(items, source, start_index, order).await,
            AppAction::PlayUpAll {
                mid,
                name,
                video_order,
                play_order,
            } => {
                self.playback.status = crate::domain::playback::PlaybackStatus::Starting;
                let req_id = self.next_request_id("playlist_build");
                self.send_network_command(network::NetworkCommand::BuildUpPlaylist {
                    req_id,
                    mid,
                    name,
                    video_order,
                    play_order,
                });
            }
            AppAction::PlayFavoriteAll {
                media_id,
                title,
                favorite_order,
                play_order,
            } => {
                self.playback.status = crate::domain::playback::PlaybackStatus::Starting;
                let req_id = self.next_request_id("playlist_build");
                self.send_network_command(network::NetworkCommand::BuildFavoritePlaylist {
                    req_id,
                    media_id,
                    title,
                    favorite_order,
                    play_order,
                });
            }
            AppAction::NavNext => {
                self.send_network_command(network::NetworkCommand::CancelPending);
                // Don't navigate if on video detail page
                if !matches!(self.current_page, Page::VideoDetail(_)) {
                    self.sidebar.next();
                    self.switch_to_nav_page().await;
                }
            }
            AppAction::NavPrev => {
                self.send_network_command(network::NetworkCommand::CancelPending);
                if !matches!(self.current_page, Page::VideoDetail(_)) {
                    self.sidebar.prev();
                    self.switch_to_nav_page().await;
                }
            }
            AppAction::CancelPendingLoads => {
                self.send_network_command(network::NetworkCommand::CancelPending);
            }
            AppAction::Search(keyword) => {
                let mut start_search = false;
                match &mut self.current_page {
                    Page::Home(home) => {
                        let page = home.search_mut();
                        page.query = keyword.clone();
                        page.page = 1;
                        page.loading = true;
                        page.show_hot_list = false;
                        start_search = true;
                    }
                    Page::Search(page) => {
                        page.query = keyword.clone();
                        page.page = 1;
                        page.loading = true;
                        page.show_hot_list = false;
                        start_search = true;
                    }
                    _ => {}
                }
                if start_search {
                    let req_id = self.next_request_id("search");
                    self.send_network_command(network::NetworkCommand::Search {
                        req_id,
                        keyword,
                        page: 1,
                    });
                }
            }
            AppAction::RefreshDynamic => {
                if self.credentials.is_none() {
                    self.apply_login_required_hint();
                    return;
                }
                if let Page::Dynamic(page) = &mut self.current_page {
                    page.loading = true;
                    let tab = page.current_tab;
                    let host_mid = page.get_selected_up_mid();
                    let req_id = self.next_request_id("dynamic_refresh");
                    self.send_network_command(network::NetworkCommand::LoadDynamicRefresh {
                        req_id,
                        tab,
                        host_mid,
                    });
                }
            }
            AppAction::OpenVideoDetail(bvid, aid) => {
                let mut detail_page =
                    crate::presentation::tui::VideoDetailPage::new(bvid.clone(), aid);
                detail_page.recommendations_enabled = !self.config.minimal_mode;
                let previous = std::mem::replace(
                    &mut self.current_page,
                    Page::VideoDetail(Box::new(detail_page)),
                );
                self.navigation_stack.push(previous);
                let req_id = self.next_request_id("video_detail");
                self.send_network_command(network::NetworkCommand::LoadVideoDetail {
                    req_id,
                    bvid,
                    aid,
                    include_related: !self.config.minimal_mode,
                });
            }
            AppAction::OpenUpPage(mid) => {
                let page = UpPage::new(mid);
                let previous = std::mem::replace(&mut self.current_page, Page::Up(Box::new(page)));
                self.navigation_stack.push(previous);
                let req_id = self.next_request_id("up_page");
                self.send_network_command(network::NetworkCommand::LoadUpPage {
                    req_id,
                    mid,
                    order: crate::api::space::SpaceVideoOrder::Latest,
                });
            }
            AppAction::RefreshUpPage => {
                if let Page::Up(page) = &mut self.current_page {
                    page.loading = true;
                    let mid = page.mid;
                    let order = page.video_order;
                    let req_id = self.next_request_id("up_page");
                    self.send_network_command(network::NetworkCommand::LoadUpPage {
                        req_id,
                        mid,
                        order,
                    });
                }
            }
            AppAction::SwitchUpVideoOrder(order) => {
                if let Page::Up(page) = &mut self.current_page {
                    page.loading = true;
                    page.videos.clear();
                    page.video_page = 1;
                    let mid = page.mid;
                    let req_id = self.next_request_id("up_videos");
                    self.send_network_command(network::NetworkCommand::LoadUpVideos {
                        req_id,
                        mid,
                        page: 1,
                        order,
                    });
                }
            }
            AppAction::LoadMoreUpVideos => {
                let command = if let Page::Up(page) = &mut self.current_page {
                    let next_page = page.video_page + 1;
                    page.loading_more = true;
                    Some((page.mid, next_page, page.video_order))
                } else {
                    None
                };
                if let Some((mid, next_page, order)) = command {
                    let req_id = self.next_request_id("up_videos");
                    self.send_network_command(network::NetworkCommand::LoadUpVideos {
                        req_id,
                        mid,
                        page: next_page,
                        order,
                    });
                }
            }
            AppAction::OpenFavoriteFolder(media_id) => {
                let owner_mid = match &self.current_page {
                    Page::Up(page) => Some(page.mid),
                    _ => None,
                };
                if let Some(owner_mid) = owner_mid {
                    let req_id = self.next_request_id("favorite_resources");
                    self.send_network_command(network::NetworkCommand::LoadFavoriteResources {
                        req_id,
                        owner_mid,
                        media_id,
                        page: 1,
                        order: match &self.current_page {
                            Page::Up(page) => page.favorite_order,
                            _ => crate::api::favorite::FavoriteOrder::RecentlyFavorited,
                        },
                    });
                }
            }
            AppAction::SwitchFavoriteOrder(order) => {
                let command = if let Page::Up(page) = &mut self.current_page
                    && let Some(media_id) = page.active_folder
                {
                    page.favorite_videos.clear();
                    page.favorite_page = 1;
                    Some((page.mid, media_id))
                } else {
                    None
                };
                if let Some((owner_mid, media_id)) = command {
                    let req_id = self.next_request_id("favorite_resources");
                    self.send_network_command(network::NetworkCommand::LoadFavoriteResources {
                        req_id,
                        owner_mid,
                        media_id,
                        page: 1,
                        order,
                    });
                }
            }
            AppAction::LoadMoreFavoriteResources => {
                let command = if let Page::Up(page) = &mut self.current_page
                    && let Some(media_id) = page.active_folder
                {
                    let next_page = page.favorite_page + 1;
                    page.loading_more = true;
                    Some((page.mid, media_id, next_page))
                } else {
                    None
                };
                if let Some((owner_mid, media_id, next_page)) = command {
                    let req_id = self.next_request_id("favorite_resources");
                    self.send_network_command(network::NetworkCommand::LoadFavoriteResources {
                        req_id,
                        owner_mid,
                        media_id,
                        page: next_page,
                        order: match &self.current_page {
                            Page::Up(page) => page.favorite_order,
                            _ => crate::api::favorite::FavoriteOrder::RecentlyFavorited,
                        },
                    });
                }
            }
            AppAction::SelectFavoriteSource(source) => {
                if let Page::Favorites(page) = &mut self.current_page {
                    page.begin_source_load(source.clone());
                    let req_id = self.next_request_id("favorites_content");
                    self.send_network_command(network::NetworkCommand::LoadFavoritesContent {
                        req_id,
                        source,
                        page: 1,
                    });
                }
            }
            AppAction::LoadMoreFavorites => {
                if let Page::Favorites(page) = &self.current_page {
                    let source = page.active_source.clone();
                    let next_page = page.page + 1;
                    let req_id = self.next_request_id("favorites_content");
                    self.send_network_command(network::NetworkCommand::LoadFavoritesContent {
                        req_id,
                        source,
                        page: next_page,
                    });
                }
            }
            AppAction::OpenDynamicDetail(dynamic_id) => {
                self.save_previous_page();
                // Cache home page before navigating to dynamic detail
                if let Page::Home(home_page) =
                    std::mem::replace(&mut self.current_page, Page::Home(HomePage::new()))
                {
                    self.cached_home = Some(home_page);
                }
                let detail_page = DynamicDetailPage::new(dynamic_id.clone());
                self.current_page = Page::DynamicDetail(Box::new(detail_page));
                let req_id = self.next_request_id("dynamic_detail");
                self.send_network_command(network::NetworkCommand::LoadDynamicDetail {
                    req_id,
                    dynamic_id,
                });
            }
            AppAction::BackToList if !self.navigation_stack.is_empty() => {
                self.auto_return_after_playback = None;
                self.current_page = self
                    .navigation_stack
                    .pop()
                    .expect("navigation stack checked as non-empty");
            }
            AppAction::BackToList => {
                self.auto_return_after_playback = None;
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
                    Some(PreviousPage::Search) if self.config.minimal_mode => {
                        self.sidebar.select(NavItem::Search);
                        self.current_page = Page::Search(SearchPage::new());
                        self.init_current_page().await;
                    }
                    Some(PreviousPage::Search) => {
                        // Search now lives inside Home; return there with the search pane open.
                        self.sidebar.select(NavItem::Home);
                        let mut home = self.cached_home.take().unwrap_or_default();
                        home.begin_search();
                        self.current_page = Page::Home(home);
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
                    Some(PreviousPage::Favorites) => {
                        self.sidebar.select(NavItem::Favorites);
                        let mid = self
                            .credentials
                            .as_ref()
                            .and_then(|credentials| credentials.dede_user_id.parse::<i64>().ok());
                        if let Some(mid) = mid {
                            self.current_page = Page::Favorites(FavoritesPage::new(mid));
                            self.init_current_page().await;
                        }
                    }
                    Some(PreviousPage::Live) => {
                        self.sidebar.select(NavItem::Live);
                        self.current_page = Page::Live(LivePage::new());
                        self.init_current_page().await;
                    }
                    Some(PreviousPage::Bangumi) => {
                        self.sidebar.select(NavItem::Bangumi);
                        if let Some(cached) = self.cached_bangumi.take() {
                            self.current_page = Page::Bangumi(Box::new(cached));
                        } else {
                            self.current_page = Page::Bangumi(Box::<BangumiPage>::default());
                            self.init_current_page().await;
                        }
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
                let request = if let Page::Home(page) = &mut self.current_page {
                    page.begin_load_more()
                        .map(|fresh_idx| (fresh_idx, page.feed()))
                } else {
                    None
                };
                if let Some((fresh_idx, feed)) = request {
                    let req_id = self.next_request_id("home_more");
                    self.send_network_command(network::NetworkCommand::LoadHomeMore {
                        req_id,
                        fresh_idx,
                        feed,
                        use_guest_feed: self.credentials.is_none(),
                    });
                }
            }
            AppAction::LoadMoreSearch => {
                let mut command = None;
                match &mut self.current_page {
                    Page::Home(home) => {
                        let page = home.search_mut();
                        if page.loading_more || page.query.is_empty() || page.show_hot_list {
                            return;
                        }
                        if page.grid.cards.len() >= page.total_results as usize {
                            return;
                        }
                        page.loading_more = true;
                        command = Some((page.query.clone(), page.page + 1));
                    }
                    Page::Search(page) => {
                        if page.loading_more || page.query.is_empty() || page.show_hot_list {
                            return;
                        }
                        if page.grid.cards.len() >= page.total_results as usize {
                            return;
                        }
                        page.loading_more = true;
                        command = Some((page.query.clone(), page.page + 1));
                    }
                    _ => {}
                }
                if let Some((keyword, next_page)) = command {
                    let req_id = self.next_request_id("search");
                    self.send_network_command(network::NetworkCommand::Search {
                        req_id,
                        keyword,
                        page: next_page,
                    });
                }
            }
            AppAction::LoadMoreDynamic => {
                if self.credentials.is_none() {
                    self.apply_login_required_hint();
                    return;
                }
                let mut command = None;
                if let Page::Dynamic(page) = &mut self.current_page {
                    if page.loading_more || !page.has_more {
                        return;
                    }
                    let Some(offset) = page.offset.clone() else {
                        return;
                    };
                    page.loading_more = true;
                    command = Some((offset, page.current_tab, page.get_selected_up_mid()));
                }
                if let Some((offset, tab, host_mid)) = command {
                    let req_id = self.next_request_id("dynamic_more");
                    self.send_network_command(network::NetworkCommand::LoadDynamicMore {
                        req_id,
                        offset,
                        tab,
                        host_mid,
                    });
                }
            }
            AppAction::LoadMoreHistory => {
                if self.credentials.is_none() {
                    self.apply_login_required_hint();
                    return;
                }
                if let Page::History(page) = &mut self.current_page
                    && let Some(cursor) = page.start_load_more_request()
                {
                    let req_id = self.next_request_id("history_more");
                    self.send_network_command(network::NetworkCommand::LoadHistoryMore {
                        req_id,
                        cursor,
                    });
                }
            }
            AppAction::DeleteHistoryItems(keys) => {
                if self.credentials.is_none() {
                    self.apply_login_required_hint();
                    if let Page::History(page) = &mut self.current_page {
                        page.cancel_deletion();
                    }
                    return;
                }
                if keys.is_empty() {
                    return;
                }
                let req_id = self.next_request_id("history_delete");
                self.send_network_command(network::NetworkCommand::DeleteHistory { req_id, keys });
            }
            AppAction::OpenArticle(cvid) => {
                let page = ArticleDetailPage::new(cvid);
                let previous =
                    std::mem::replace(&mut self.current_page, Page::ArticleDetail(Box::new(page)));
                self.navigation_stack.push(previous);
                let req_id = self.next_request_id("article_detail");
                self.send_network_command(network::NetworkCommand::LoadArticle { req_id, cvid });
            }
            AppAction::OpenHistoryBangumi { season_id, ep_id } => {
                let page =
                    BangumiDetailPage::new_for_episode(season_id, ep_id, self.config.auto_play);
                let previous =
                    std::mem::replace(&mut self.current_page, Page::BangumiDetail(Box::new(page)));
                self.navigation_stack.push(previous);
                let req_id = self.next_request_id("bangumi_detail");
                self.send_network_command(network::NetworkCommand::LoadBangumiDetail {
                    req_id,
                    season_id,
                });
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
                if self.credentials.is_none() {
                    self.apply_login_required_hint();
                    return;
                }
                let mut command = None;
                if let Page::Dynamic(page) = &mut self.current_page {
                    page.switch_tab(tab);
                    let host_mid = page.get_selected_up_mid();
                    command = Some((page.current_tab, host_mid));
                }
                if let Some((tab, host_mid)) = command {
                    let req_id = self.next_request_id("dynamic_refresh");
                    self.send_network_command(network::NetworkCommand::LoadDynamicRefresh {
                        req_id,
                        tab,
                        host_mid,
                    });
                }
            }
            AppAction::SelectUpMaster(index) => {
                if self.credentials.is_none() {
                    self.apply_login_required_hint();
                    return;
                }
                let mut command = None;
                if let Page::Dynamic(page) = &mut self.current_page {
                    page.select_up(index);
                    let host_mid = page.get_selected_up_mid();
                    command = Some((page.current_tab, host_mid));
                }
                if let Some((tab, host_mid)) = command {
                    let req_id = self.next_request_id("dynamic_refresh");
                    self.send_network_command(network::NetworkCommand::LoadDynamicRefresh {
                        req_id,
                        tab,
                        host_mid,
                    });
                }
            }
            AppAction::NextTheme => {
                self.theme_id = Theme::next_theme_id(&self.theme_id);
                self.theme = Theme::load_or_default(&self.theme_id).0;
                self.save_theme_to_config();
            }
            AppAction::SetTheme(theme_id) => {
                self.theme_id = theme_id;
                self.theme = Theme::load_or_default(&self.theme_id).0;
                self.save_theme_to_config();
            }
            AppAction::SwitchToSettings => {
                self.sidebar.select(NavItem::Settings);
                let page = SettingsPage::new(
                    self.keybindings.clone(),
                    self.theme_id.clone(),
                    self.credentials.is_some(),
                    self.config.danmaku.clone(),
                    self.config.auto_play,
                    self.config.video_quality,
                )
                .with_minimal_mode(self.config.minimal_mode);
                self.current_page = Page::Settings(Box::new(page));
            }
            AppAction::Logout => {
                let _ = persistence::delete_credentials();
                self.credentials = None;
                self.api_client.clear_credentials();
                self.cached_home = None;
                self.current_page = Page::Home(HomePage::new());
                self.init_current_page().await;
            }
            AppAction::LikeComment {
                oid,
                rpid,
                comment_type,
            } => {
                if self.credentials.is_none() {
                    self.apply_login_required_hint();
                    return;
                }
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
                if self.credentials.is_none() {
                    self.apply_login_required_hint();
                    return;
                }
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
                let _ = persistence::save_config(&self.config);
            }
            AppAction::SaveDanmakuConfig(danmaku) => {
                self.config.danmaku = *danmaku;
                self.danmaku_config_tx
                    .send_replace(self.config.danmaku.clone());
                let _ = persistence::save_config(&self.config);
            }
            AppAction::SaveMinimalMode(enabled) => {
                self.apply_minimal_mode(enabled);
                let save_failed = persistence::save_config(&self.config).is_err();
                if let Page::Settings(page) = &mut self.current_page {
                    page.minimal_mode = enabled;
                    page.minimal_mode_error = save_failed
                        .then(|| "极简模式已生效，但保存失败；请检查配置目录权限".to_string());
                }
                if !matches!(self.current_page, Page::Settings(_)) {
                    self.current_page = if enabled {
                        Page::Search(SearchPage::new())
                    } else {
                        Page::Home(HomePage::new())
                    };
                    self.init_current_page().await;
                }
            }
            AppAction::SaveAutoPlay(enabled) => {
                self.config.auto_play = enabled;
                let _ = persistence::save_config(&self.config);
            }
            AppAction::SaveVideoQuality(quality) => {
                self.config.video_quality = quality;
                let _ = persistence::save_config(&self.config);
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
                detail_page.load_history_danmaku(client).await;
                let uid = self
                    .credentials
                    .as_ref()
                    .and_then(|c| c.dede_user_id.parse::<i64>().ok())
                    .unwrap_or(0);
                let existing_hub = self
                    .live_danmaku_hub
                    .as_ref()
                    .filter(|hub| hub.room_id() == room_id)
                    .cloned();
                let hub_result = match existing_hub {
                    Some(hub) => Ok(hub),
                    None => crate::api::LiveDanmakuHub::connect(client, room_id, uid).await,
                };
                match hub_result {
                    Ok(hub) => {
                        detail_page.attach_danmaku_hub(Arc::clone(&hub));
                        self.live_danmaku_hub = Some(hub);
                    }
                    Err(error) => {
                        detail_page.set_ws_error(format!("WS连接失败: {error}"));
                    }
                }
                self.current_page = Page::LiveDetail(Box::new(detail_page));
            }
            AppAction::RefreshLive => {
                if let Page::Live(page) = &mut self.current_page {
                    page.begin_loading();
                    let req_id = self.next_request_id("live_init");
                    self.send_network_command(network::NetworkCommand::LoadLiveInit { req_id });
                }
            }
            AppAction::LoadMoreLive => {
                if let Page::Live(page) = &mut self.current_page
                    && page.begin_load_more()
                {
                    let req_id = self.next_request_id("live_more");
                    self.send_network_command(network::NetworkCommand::LoadLiveMore { req_id });
                }
            }
            AppAction::SendLiveDanmaku { room_id, message } => {
                let req_id = self.next_request_id("live_danmaku_send");
                if let Page::LiveDetail(page) = &mut self.current_page {
                    page.begin_danmaku_send(req_id);
                    if let Err(std::sync::mpsc::SendError(
                        network::NetworkCommand::SendLiveDanmaku { message, .. },
                    )) = self
                        .network_command_tx
                        .send(network::NetworkCommand::SendLiveDanmaku {
                            req_id,
                            room_id,
                            message,
                        })
                    {
                        page.finish_danmaku_send(req_id, Err((message, "网络任务已停止".into())));
                    }
                }
            }
            AppAction::PlayLive { room_id, title: _ } => {
                let existing_hub = self
                    .live_danmaku_hub
                    .as_ref()
                    .filter(|hub| hub.room_id() == room_id)
                    .cloned();
                let danmaku_hub = if existing_hub.is_some() {
                    existing_hub
                } else {
                    let uid = self
                        .credentials
                        .as_ref()
                        .and_then(|credentials| credentials.dede_user_id.parse::<i64>().ok())
                        .unwrap_or(0);
                    match crate::api::LiveDanmakuHub::connect(&self.api_client, room_id, uid).await
                    {
                        Ok(hub) => {
                            self.live_danmaku_hub = Some(Arc::clone(&hub));
                            Some(hub)
                        }
                        Err(error) => {
                            if let Page::LiveDetail(page) = &mut self.current_page {
                                page.set_ws_error(format!("WS连接失败: {error}"));
                            }
                            None
                        }
                    }
                };
                match media::play_live(
                    self.api_client.clone(),
                    room_id,
                    danmaku_hub,
                    self.danmaku_config_tx.subscribe(),
                )
                .await
                {
                    Ok(()) => {
                        self.playback.status = crate::domain::playback::PlaybackStatus::Playing;
                        self.playback.last_error = None;
                    }
                    Err(error) => {
                        self.playback.status = crate::domain::playback::PlaybackStatus::Failed;
                        self.playback.last_error = Some(format!("启动直播失败: {error:#}"));
                    }
                }
            }
            AppAction::SwitchToBangumi => {
                self.sidebar.select(NavItem::Bangumi);
                if let Some(cached) = self.cached_bangumi.take() {
                    self.current_page = Page::Bangumi(Box::new(cached));
                } else {
                    self.current_page = Page::Bangumi(Box::<BangumiPage>::default());
                    self.init_current_page().await;
                }
            }
            AppAction::RefreshBangumi => {
                self.sidebar.select(NavItem::Bangumi);
                self.cached_bangumi = None;
                self.current_page = Page::Bangumi(Box::<BangumiPage>::default());
                self.init_current_page().await;
            }
            AppAction::SwitchBangumiTab(_tab) => {
                // Single tab mode, no-op
            }
            AppAction::OpenBangumiDetail(season_id) => {
                // Invalidate comments that belong to the season being left.
                self.next_request_id("bangumi_comments");
                if matches!(self.current_page, Page::Bangumi(_)) {
                    self.save_previous_page();
                    if let Page::Bangumi(bangumi_page) = std::mem::replace(
                        &mut self.current_page,
                        Page::Bangumi(Box::<BangumiPage>::default()),
                    ) {
                        self.cached_bangumi = Some(*bangumi_page);
                    }
                } else if matches!(self.current_page, Page::BangumiDetail(_)) {
                    // Nested season jump: keep the previous season reachable via Esc.
                    if let Page::BangumiDetail(previous_page) = std::mem::replace(
                        &mut self.current_page,
                        Page::Bangumi(Box::<BangumiPage>::default()),
                    ) {
                        self.navigation_stack
                            .push(Page::BangumiDetail(previous_page));
                    }
                } else {
                    self.save_previous_page();
                }
                let detail_page = BangumiDetailPage::new(season_id);
                self.current_page = Page::BangumiDetail(Box::new(detail_page));
                let req_id = self.next_request_id("bangumi_detail");
                self.send_network_command(network::NetworkCommand::LoadBangumiDetail {
                    req_id,
                    season_id,
                });
            }
            AppAction::LoadBangumiComments { aid, page } => {
                if aid <= 0 || !matches!(self.current_page, Page::BangumiDetail(_)) {
                    return;
                }
                if let Page::BangumiDetail(detail) = &mut self.current_page {
                    detail.begin_comments_load(aid, page);
                }
                let req_id = self.next_request_id("bangumi_comments");
                self.send_network_command(network::NetworkCommand::LoadBangumiComments {
                    req_id,
                    aid,
                    page,
                });
            }
            AppAction::LoadMoreBangumi => {
                // Rank API has no pagination
            }
            AppAction::PlayBangumiEpisode {
                ep_id,
                season_id: _,
                title: _,
            } => {
                let session_id = self.allocate_playback_session();
                self.playback.session_id = None;
                self.playback.status = crate::domain::playback::PlaybackStatus::Starting;
                match media::play_bangumi_episode(
                    self.api_client.clone(),
                    ep_id,
                    self.credentials.as_ref(),
                    self.config.video_quality,
                    self.playback_event_tx.clone(),
                    session_id,
                )
                .await
                {
                    Ok(()) => self.playback.begin_session(session_id),
                    Err(error) => {
                        self.playback.status = crate::domain::playback::PlaybackStatus::Failed;
                        self.playback.last_error = Some(format!("启动番剧播放器失败: {error:#}"));
                    }
                }
            }
            AppAction::None => {}
        }
    }

    pub(super) async fn start_playlist(
        &mut self,
        items: Vec<crate::domain::playback::PlaylistItem>,
        source: crate::domain::playback::PlaylistSource,
        start_index: usize,
        order: crate::domain::playback::PlayOrder,
    ) {
        self.playback.replace_queue(source, items.clone());
        self.playback.order = order;
        let _ = self.playback.play_from(start_index);
        let session_id = self.allocate_playback_session();
        match media::play_playlist(
            self.api_client.clone(),
            items,
            order,
            start_index,
            self.credentials.as_ref(),
            self.config.video_quality,
            self.playback_event_tx.clone(),
            session_id,
        )
        .await
        {
            Ok(()) => {
                self.playback.begin_session(session_id);
            }
            Err(error) => {
                self.playback.status = crate::domain::playback::PlaybackStatus::Failed;
                self.playback.last_error = Some(format!("启动播放列表失败: {error:#}"));
            }
        }
    }

    async fn switch_to_nav_page(&mut self) {
        // First, cache home page if we're leaving it
        if matches!(self.current_page, Page::Home(_))
            && self.sidebar.selected != NavItem::Home
            && let Page::Home(home_page) =
                std::mem::replace(&mut self.current_page, Page::Home(HomePage::new()))
        {
            self.cached_home = Some(home_page);
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
            NavItem::MySpace => {
                if let Some(mid) = self
                    .credentials
                    .as_ref()
                    .and_then(|credentials| credentials.dede_user_id.parse::<i64>().ok())
                {
                    let page = UpPage::new_sidebar(mid);
                    self.current_page = Page::Up(Box::new(page));
                    let req_id = self.next_request_id("up_page");
                    self.send_network_command(network::NetworkCommand::LoadUpPage {
                        req_id,
                        mid,
                        order: crate::api::space::SpaceVideoOrder::Latest,
                    });
                } else {
                    self.sidebar.select(NavItem::Settings);
                    self.current_page = Page::Settings(Box::new(
                        SettingsPage::new(
                            self.keybindings.clone(),
                            self.theme_id.clone(),
                            false,
                            self.config.danmaku.clone(),
                            self.config.auto_play,
                            self.config.video_quality,
                        )
                        .with_minimal_mode(self.config.minimal_mode),
                    ));
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
            NavItem::Favorites => {
                let mid = self
                    .credentials
                    .as_ref()
                    .and_then(|credentials| credentials.dede_user_id.parse::<i64>().ok());
                if let Some(mid) = mid {
                    if !matches!(self.current_page, Page::Favorites(_)) {
                        self.current_page = Page::Favorites(FavoritesPage::new(mid));
                        self.init_current_page().await;
                    }
                } else {
                    self.current_page = Page::Settings(Box::new(
                        SettingsPage::new(
                            self.keybindings.clone(),
                            self.theme_id.clone(),
                            false,
                            self.config.danmaku.clone(),
                            self.config.auto_play,
                            self.config.video_quality,
                        )
                        .with_minimal_mode(self.config.minimal_mode),
                    ));
                }
            }
            NavItem::Settings => {
                if !matches!(self.current_page, Page::Settings(_)) {
                    let page = SettingsPage::new(
                        self.keybindings.clone(),
                        self.theme_id.clone(),
                        self.credentials.is_some(),
                        self.config.danmaku.clone(),
                        self.config.auto_play,
                        self.config.video_quality,
                    )
                    .with_minimal_mode(self.config.minimal_mode);
                    self.current_page = Page::Settings(Box::new(page));
                }
            }
            NavItem::Live => {
                if !matches!(self.current_page, Page::Live(_)) {
                    self.current_page = Page::Live(LivePage::new());
                    self.init_current_page().await;
                }
            }
            NavItem::Bangumi => {
                if !matches!(self.current_page, Page::Bangumi(_)) {
                    if let Some(cached) = self.cached_bangumi.take() {
                        self.current_page = Page::Bangumi(Box::new(cached));
                    } else {
                        self.current_page = Page::Bangumi(Box::<BangumiPage>::default());
                        self.init_current_page().await;
                    }
                }
            }
        }
    }

    pub(super) async fn init_current_page(&mut self) {
        if self.config.minimal_mode
            && matches!(
                self.current_page,
                Page::Home(_) | Page::Live(_) | Page::Bangumi(_)
            )
        {
            self.sidebar.select(NavItem::Search);
            self.current_page = Page::Search(SearchPage::new());
        }
        match &mut self.current_page {
            Page::Login(page) => {
                let client = self.api_client.clone();
                page.load_qrcode(&client).await;
            }
            Page::Home(page) => {
                let feed = {
                    page.begin_loading();
                    page.search_mut().start_hotword_loading();
                    page.feed()
                };
                let home_req_id = self.next_request_id("home");
                self.send_network_command(network::NetworkCommand::LoadHome {
                    req_id: home_req_id,
                    feed,
                    use_guest_feed: self.credentials.is_none(),
                });
                let hotword_req_id = self.next_request_id("hotwords");
                self.send_network_command(network::NetworkCommand::LoadHotwords {
                    req_id: hotword_req_id,
                });
            }
            Page::Favorites(page) => {
                page.loading = true;
                let mid = page.mid;
                let req_id = self.next_request_id("favorites_init");
                self.send_network_command(network::NetworkCommand::LoadFavoritesInit {
                    req_id,
                    mid,
                });
            }
            Page::Search(page) => {
                if self.config.minimal_mode {
                    page.disable_hotwords();
                    return;
                }
                page.start_hotword_loading();
                let req_id = self.next_request_id("hotwords");
                self.send_network_command(network::NetworkCommand::LoadHotwords { req_id });
            }
            Page::Dynamic(page) => {
                if self.credentials.is_none() {
                    page.loading_up_list = false;
                    page.set_error(Self::login_required_message());
                    return;
                }
                page.loading_up_list = true;
                let tab = page.current_tab;
                let host_mid = page.get_selected_up_mid();
                let req_id = self.next_request_id("dynamic_init");
                self.send_network_command(network::NetworkCommand::LoadDynamicInit {
                    req_id,
                    tab,
                    host_mid,
                });
            }
            Page::VideoDetail(_) => {
                // VideoDetail is initialized when created
            }
            Page::DynamicDetail(_) => {
                // DynamicDetail is initialized when created
            }
            Page::ArticleDetail(_) => {
                // ArticleDetail is initialized when opened from history.
            }
            Page::History(page) => {
                if self.credentials.is_none() {
                    page.apply_load_more_error(Self::login_required_message());
                    return;
                }
                page.begin_loading();
                let req_id = self.next_request_id("history_init");
                self.send_network_command(network::NetworkCommand::LoadHistoryInit { req_id });
            }
            Page::Live(page) => {
                page.begin_loading();
                let req_id = self.next_request_id("live_init");
                self.send_network_command(network::NetworkCommand::LoadLiveInit { req_id });
            }
            Page::LiveDetail(page) => {
                let client = self.api_client.clone();
                page.load_room_info(&client).await;
            }
            Page::Settings(_) => {
                // Settings doesn't need async initialization
            }
            Page::Bangumi(page) => {
                page.loading = true;
                let req_id = self.next_request_id("bangumi_index");
                self.send_network_command(network::NetworkCommand::LoadBangumiIndex { req_id });
            }
            Page::BangumiDetail(_) => {
                // BangumiDetail is initialized when created
            }
            Page::Up(_) => {
                // UpPage is initialized by OpenUpPage with an identity-bound request.
            }
        }
    }

    pub(super) fn apply_minimal_mode(&mut self, enabled: bool) {
        self.config.minimal_mode = enabled;
        self.sidebar.set_minimal_mode(enabled);
        self.cached_home = None;
        self.cached_bangumi = None;
        self.navigation_stack.clear();
        self.previous_page = None;
        for key in [
            "home",
            "home_more",
            "hotwords",
            "live_init",
            "live_more",
            "bangumi_index",
        ] {
            self.next_request_id(key);
        }
        self.send_network_command(network::NetworkCommand::CancelPending);
    }

    fn save_theme_to_config(&mut self) {
        self.config.theme = self.theme_id.clone();
        if persistence::save_config(&self.config).is_err() {}
    }
}
