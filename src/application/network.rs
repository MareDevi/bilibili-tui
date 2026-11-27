use crate::api::{
    ApiClient,
    article::ArticleData,
    bangumi::{SeasonQuery, SeasonRankItem, SeasonResult},
    comment::{CommentItem, CommentType},
    dynamic::DynamicItem,
    dynamic::UpListItem,
    favorite::{
        CollectedFolder, FavoriteFolder, FavoriteOrder, FavoriteResourceData, FavoriteSource,
        SeasonArchivesData, WatchLaterData,
    },
    history::{HistoryCursor, HistoryData, HistoryKey},
    live::LiveRoom,
    recommend::{HomeFeed, VideoItem},
    search::HotwordItem,
    search::SearchVideoItem,
    space::{RelationStat, SpaceInfo, SpaceVideoData, SpaceVideoOrder},
    video::RelatedVideoItem,
    video::VideoInfo,
};
use crate::domain::playback::{PlayOrder, PlaylistItem, PlaylistSource};
use crate::presentation::tui::DynamicTab;
use futures_util::{StreamExt, stream};
use std::collections::HashMap;
use std::sync::{Arc, mpsc};

#[derive(Debug)]
pub enum NetworkCommand {
    CancelPending,
    LoadHome {
        req_id: u64,
        feed: HomeFeed,
        use_guest_feed: bool,
    },
    LoadHomeMore {
        req_id: u64,
        fresh_idx: i32,
        feed: HomeFeed,
        use_guest_feed: bool,
    },
    LoadHotwords {
        req_id: u64,
    },
    Search {
        req_id: u64,
        keyword: String,
        page: i32,
    },
    LoadDynamicInit {
        req_id: u64,
        tab: DynamicTab,
        host_mid: Option<i64>,
    },
    LoadDynamicRefresh {
        req_id: u64,
        tab: DynamicTab,
        host_mid: Option<i64>,
    },
    LoadDynamicMore {
        req_id: u64,
        offset: String,
        tab: DynamicTab,
        host_mid: Option<i64>,
    },
    LoadHistoryInit {
        req_id: u64,
    },
    LoadHistoryMore {
        req_id: u64,
        cursor: HistoryCursor,
    },
    DeleteHistory {
        req_id: u64,
        keys: Vec<HistoryKey>,
    },
    LoadArticle {
        req_id: u64,
        cvid: i64,
    },
    LoadLiveInit {
        req_id: u64,
    },
    LoadLiveMore {
        req_id: u64,
    },
    LoadVideoDetail {
        req_id: u64,
        bvid: String,
        aid: i64,
        include_related: bool,
    },
    LoadUpPage {
        req_id: u64,
        mid: i64,
        order: SpaceVideoOrder,
    },
    LoadUpVideos {
        req_id: u64,
        mid: i64,
        page: i32,
        order: SpaceVideoOrder,
    },
    LoadFavoriteResources {
        req_id: u64,
        owner_mid: i64,
        media_id: i64,
        page: i32,
        order: FavoriteOrder,
    },
    BuildUpPlaylist {
        req_id: u64,
        mid: i64,
        name: String,
        video_order: SpaceVideoOrder,
        play_order: PlayOrder,
    },
    BuildFavoritePlaylist {
        req_id: u64,
        media_id: i64,
        title: String,
        favorite_order: FavoriteOrder,
        play_order: PlayOrder,
    },
    LoadFavoritesInit {
        req_id: u64,
        mid: i64,
    },
    LoadFavoritesContent {
        req_id: u64,
        source: FavoriteSource,
        page: i32,
    },
    LoadDynamicDetail {
        req_id: u64,
        dynamic_id: String,
    },
    LoadBangumiIndex {
        req_id: u64,
    },
    LoadBangumiDetail {
        req_id: u64,
        season_id: i64,
    },
    SendLiveDanmaku {
        req_id: u64,
        room_id: i64,
        message: String,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum NetworkEvent {
    HomeLoaded {
        req_id: u64,
        feed: HomeFeed,
        videos: Vec<VideoItem>,
    },
    HomeMoreLoaded {
        req_id: u64,
        feed: HomeFeed,
        videos: Vec<VideoItem>,
    },
    HotwordsLoaded {
        req_id: u64,
        hotwords: Vec<HotwordItem>,
    },
    SearchLoaded {
        req_id: u64,
        keyword: String,
        page: i32,
        results: Vec<SearchVideoItem>,
        total: i32,
    },
    DynamicLoaded {
        req_id: u64,
        append: bool,
        up_list: Option<Vec<UpListItem>>,
        items: Vec<crate::api::dynamic::DynamicItem>,
        offset: Option<String>,
        has_more: bool,
    },
    HistoryLoaded {
        req_id: u64,
        append: bool,
        data: HistoryData,
    },
    HistoryDeleted {
        req_id: u64,
        successful: Vec<HistoryKey>,
        failed: Vec<(HistoryKey, String)>,
    },
    ArticleLoaded {
        req_id: u64,
        cvid: i64,
        article: ArticleData,
        comments: Vec<CommentItem>,
    },
    LiveLoaded {
        req_id: u64,
        append: bool,
        rooms: Vec<LiveRoom>,
    },
    LiveDanmakuSent {
        req_id: u64,
        result: Result<(), (String, String)>,
    },
    VideoDetailLoaded {
        req_id: u64,
        bvid: String,
        video_info: VideoInfo,
        comments: Vec<CommentItem>,
        has_more_comments: bool,
        related_videos: Vec<RelatedVideoItem>,
    },
    UpPageLoaded {
        req_id: u64,
        mid: i64,
        order: SpaceVideoOrder,
        profile: SpaceInfo,
        relation: Option<RelationStat>,
        videos: SpaceVideoData,
        folders: Vec<FavoriteFolder>,
    },
    UpVideosLoaded {
        req_id: u64,
        mid: i64,
        page: i32,
        order: SpaceVideoOrder,
        videos: SpaceVideoData,
    },
    FavoriteResourcesLoaded {
        req_id: u64,
        owner_mid: i64,
        media_id: i64,
        page: i32,
        order: FavoriteOrder,
        resources: FavoriteResourceData,
    },
    PlaylistLoaded {
        req_id: u64,
        items: Vec<PlaylistItem>,
        source: PlaylistSource,
        start_index: usize,
        order: PlayOrder,
    },
    FavoritesInitLoaded {
        req_id: u64,
        mid: i64,
        watch_later: WatchLaterData,
        created: Vec<FavoriteFolder>,
        collected: Vec<CollectedFolder>,
    },
    FavoritesWatchLaterLoaded {
        req_id: u64,
        page: i32,
        data: WatchLaterData,
    },
    FavoritesCreatedLoaded {
        req_id: u64,
        media_id: i64,
        page: i32,
        data: FavoriteResourceData,
    },
    FavoritesCollectedLoaded {
        req_id: u64,
        season_id: i64,
        page: i32,
        data: SeasonArchivesData,
    },
    DynamicDetailLoaded {
        req_id: u64,
        dynamic_id: String,
        dynamic_item: DynamicItem,
        comments: Vec<CommentItem>,
        has_more_comments: bool,
        image_urls: Vec<String>,
    },
    BangumiIndexLoaded {
        req_id: u64,
        items: Vec<SeasonRankItem>,
    },
    BangumiDetailLoaded {
        req_id: u64,
        season_id: i64,
        season: SeasonResult,
    },
    RequestFailed {
        req_id: u64,
        target: &'static str,
        error: String,
    },
}

pub struct NetworkBridge {
    pub command_tx: mpsc::Sender<NetworkCommand>,
    pub event_rx: mpsc::Receiver<NetworkEvent>,
}

pub fn start_network_worker(api_client: Arc<ApiClient>) -> NetworkBridge {
    let (command_tx, command_rx) = mpsc::channel::<NetworkCommand>();
    let (event_tx, event_rx) = mpsc::channel::<NetworkEvent>();

    std::thread::Builder::new()
        .name("bilibili-network-worker".to_string())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(2)
                .build()
            {
                Ok(rt) => rt,
                Err(_) => return,
            };

            let mut cancellations =
                HashMap::<&'static str, tokio_util::sync::CancellationToken>::new();
            while let Ok(command) = command_rx.recv() {
                if matches!(command, NetworkCommand::CancelPending) {
                    for (_, token) in cancellations.drain() {
                        token.cancel();
                    }
                    continue;
                }
                let key = command.cancel_key();
                let token = tokio_util::sync::CancellationToken::new();
                if let Some(previous) = cancellations.insert(key, token.clone()) {
                    previous.cancel();
                }
                let api_client = api_client.clone();
                let event_tx = event_tx.clone();
                runtime.spawn(async move {
                    tokio::select! {
                        _ = token.cancelled() => {}
                        event = handle_command(api_client, command) => {
                            let _ = event_tx.send(event);
                        }
                    }
                });
            }
        })
        .expect("failed to spawn network worker");

    NetworkBridge {
        command_tx,
        event_rx,
    }
}

impl NetworkCommand {
    fn cancel_key(&self) -> &'static str {
        match self {
            Self::LoadHome { .. } | Self::LoadHomeMore { .. } => "home",
            Self::LoadFavoriteResources { .. }
            | Self::LoadFavoritesInit { .. }
            | Self::LoadFavoritesContent { .. } => "favorites",
            Self::BuildUpPlaylist { .. } | Self::BuildFavoritePlaylist { .. } => "playlist",
            Self::CancelPending => "cancel",
            Self::LoadHotwords { .. } | Self::Search { .. } => "search",
            Self::LoadDynamicInit { .. }
            | Self::LoadDynamicRefresh { .. }
            | Self::LoadDynamicMore { .. } => "dynamic",
            Self::LoadHistoryInit { .. } | Self::LoadHistoryMore { .. } => "history",
            Self::DeleteHistory { .. } => "history_delete",
            Self::LoadArticle { .. } => "article_detail",
            Self::LoadLiveInit { .. } | Self::LoadLiveMore { .. } => "live",
            Self::SendLiveDanmaku { .. } => "live_danmaku_send",
            Self::LoadVideoDetail { .. } => "video_detail",
            Self::LoadUpPage { .. } | Self::LoadUpVideos { .. } => "up",
            Self::LoadDynamicDetail { .. } => "dynamic_detail",
            Self::LoadBangumiIndex { .. } | Self::LoadBangumiDetail { .. } => "bangumi",
        }
    }
}

async fn handle_command(api_client: Arc<ApiClient>, command: NetworkCommand) -> NetworkEvent {
    match command {
        NetworkCommand::CancelPending => unreachable!("handled by worker"),
        NetworkCommand::LoadHome {
            req_id,
            feed,
            use_guest_feed,
        } => {
            let result = match (feed, use_guest_feed) {
                (HomeFeed::Recommended, false) => api_client.get_recommendations().await,
                (HomeFeed::Recommended, true) | (HomeFeed::Popular, _) => {
                    api_client.get_popular_videos(1, 20).await
                }
                _ => api_client.get_home_feed(feed, 1, 20).await,
            };
            match result {
                Ok(mut videos) => {
                    enrich_followers(&api_client, &mut videos).await;
                    NetworkEvent::HomeLoaded {
                        req_id,
                        feed,
                        videos,
                    }
                }
                Err(e) => failed(req_id, "home", e),
            }
        }
        NetworkCommand::LoadHomeMore {
            req_id,
            fresh_idx,
            feed,
            use_guest_feed,
        } => {
            let result = match (feed, use_guest_feed) {
                (HomeFeed::Recommended, false) => {
                    api_client.get_recommendations_paged(fresh_idx).await
                }
                (HomeFeed::Recommended, true) | (HomeFeed::Popular, _) => {
                    api_client.get_popular_videos(fresh_idx, 20).await
                }
                _ => api_client.get_home_feed(feed, fresh_idx, 20).await,
            };
            match result {
                Ok(mut videos) => {
                    enrich_followers(&api_client, &mut videos).await;
                    NetworkEvent::HomeMoreLoaded {
                        req_id,
                        feed,
                        videos,
                    }
                }
                Err(e) => failed(req_id, "home_more", e),
            }
        }
        NetworkCommand::LoadHotwords { req_id } => match api_client.get_hot_search().await {
            Ok(hotwords) => NetworkEvent::HotwordsLoaded { req_id, hotwords },
            Err(e) => failed(req_id, "hotwords", e),
        },
        NetworkCommand::Search {
            req_id,
            keyword,
            page,
        } => match api_client.search_videos(&keyword, page).await {
            Ok(data) => NetworkEvent::SearchLoaded {
                req_id,
                keyword,
                page,
                results: data.result.unwrap_or_default(),
                total: data.num_results.unwrap_or(0),
            },
            Err(e) => failed(req_id, "search", e),
        },
        NetworkCommand::LoadUpPage { req_id, mid, order } => {
            match api_client.get_space_info(mid).await {
                Ok(profile) => {
                    let relation = api_client.get_relation_stat(mid).await.ok();
                    let folders = api_client
                        .get_favorite_folders(mid)
                        .await
                        .unwrap_or_default();
                    match api_client.get_space_videos(mid, 1, 40, order).await {
                        Ok(videos) => NetworkEvent::UpPageLoaded {
                            req_id,
                            mid,
                            order,
                            profile,
                            relation,
                            videos,
                            folders,
                        },
                        Err(error) => failed(req_id, "up_page", error),
                    }
                }
                Err(error) => failed(req_id, "up_page", error),
            }
        }
        NetworkCommand::LoadUpVideos {
            req_id,
            mid,
            page,
            order,
        } => match api_client.get_space_videos(mid, page, 40, order).await {
            Ok(videos) => NetworkEvent::UpVideosLoaded {
                req_id,
                mid,
                page,
                order,
                videos,
            },
            Err(error) => failed(req_id, "up_videos", error),
        },
        NetworkCommand::LoadFavoriteResources {
            req_id,
            owner_mid,
            media_id,
            page,
            order,
        } => match api_client
            .get_favorite_resources(media_id, page, 40, order)
            .await
        {
            Ok(resources) => NetworkEvent::FavoriteResourcesLoaded {
                req_id,
                owner_mid,
                media_id,
                page,
                order,
                resources,
            },
            Err(error) => failed(req_id, "favorite_resources", error),
        },
        NetworkCommand::BuildUpPlaylist {
            req_id,
            mid,
            name,
            video_order,
            play_order,
        } => {
            let mut items = Vec::new();
            let mut page = 1;
            loop {
                let data = match api_client
                    .get_space_videos(mid, page, 40, video_order)
                    .await
                {
                    Ok(data) => data,
                    Err(error) => return failed(req_id, "playlist_build", error),
                };
                let total = data.page.count as usize;
                let empty = data.list.vlist.is_empty();
                items.extend(data.list.vlist.into_iter().map(|video| PlaylistItem {
                    bvid: video.bvid,
                    aid: video.aid,
                    cid: None,
                    title: video.title,
                    uploader_mid: Some(video.mid.unwrap_or(mid)),
                    duration: video.duration,
                    page: None,
                }));
                if empty || items.len() >= total {
                    break;
                }
                page += 1;
            }
            let start_index = if play_order == PlayOrder::Reverse {
                items.len().saturating_sub(1)
            } else {
                0
            };
            NetworkEvent::PlaylistLoaded {
                req_id,
                items,
                source: PlaylistSource::Uploader { mid, name },
                start_index,
                order: play_order,
            }
        }
        NetworkCommand::BuildFavoritePlaylist {
            req_id,
            media_id,
            title,
            favorite_order,
            play_order,
        } => {
            let mut items = Vec::new();
            let mut page = 1;
            loop {
                let data = match api_client
                    .get_favorite_resources(media_id, page, 40, favorite_order)
                    .await
                {
                    Ok(data) => data,
                    Err(error) => return failed(req_id, "playlist_build", error),
                };
                let has_more = data.has_more.unwrap_or(false);
                items.extend(data.medias.into_iter().filter_map(|media| {
                    Some(PlaylistItem {
                        bvid: media.bvid?,
                        aid: media.id,
                        cid: None,
                        title: media.title,
                        uploader_mid: media.upper.as_ref().map(|upper| upper.mid),
                        duration: media.duration,
                        page: None,
                    })
                }));
                if !has_more {
                    break;
                }
                page += 1;
            }
            let start_index = if play_order == PlayOrder::Reverse {
                items.len().saturating_sub(1)
            } else {
                0
            };
            NetworkEvent::PlaylistLoaded {
                req_id,
                items,
                source: PlaylistSource::Favorites { media_id, title },
                start_index,
                order: play_order,
            }
        }
        NetworkCommand::LoadFavoritesInit { req_id, mid } => {
            match api_client.get_watch_later(1, 20).await {
                Ok(watch_later) => {
                    let created = api_client
                        .get_favorite_folders(mid)
                        .await
                        .unwrap_or_default();
                    match api_client.get_collected_folders(mid, 1, 50).await {
                        Ok(collected) => NetworkEvent::FavoritesInitLoaded {
                            req_id,
                            mid,
                            watch_later,
                            created,
                            collected: collected.list,
                        },
                        Err(error) => failed(req_id, "favorites_init", error),
                    }
                }
                Err(error) => failed(req_id, "favorites_init", error),
            }
        }
        NetworkCommand::LoadFavoritesContent {
            req_id,
            source,
            page,
        } => match source {
            FavoriteSource::WatchLater => match api_client.get_watch_later(page, 20).await {
                Ok(data) => NetworkEvent::FavoritesWatchLaterLoaded { req_id, page, data },
                Err(error) => failed(req_id, "favorites_content", error),
            },
            FavoriteSource::Created { media_id, .. } => match api_client
                .get_favorite_resources(media_id, page, 40, FavoriteOrder::RecentlyFavorited)
                .await
            {
                Ok(data) => NetworkEvent::FavoritesCreatedLoaded {
                    req_id,
                    media_id,
                    page,
                    data,
                },
                Err(error) => failed(req_id, "favorites_content", error),
            },
            FavoriteSource::Collected { season_id, mid, .. } => match api_client
                .get_collected_season_videos(mid, season_id, page, 30)
                .await
            {
                Ok(data) => NetworkEvent::FavoritesCollectedLoaded {
                    req_id,
                    season_id,
                    page,
                    data,
                },
                Err(error) => failed(req_id, "favorites_content", error),
            },
        },
        NetworkCommand::LoadDynamicInit {
            req_id,
            tab,
            host_mid,
        } => {
            let up_list = match api_client.get_dynamic_portal().await {
                Ok(portal) => portal.up_list,
                Err(_) => None,
            };
            let feed_type = tab.get_feed_type();
            match api_client.get_dynamic_feed(None, feed_type, host_mid).await {
                Ok(data) => NetworkEvent::DynamicLoaded {
                    req_id,
                    append: false,
                    up_list,
                    items: data.items.unwrap_or_default(),
                    offset: data.offset,
                    has_more: data.has_more.unwrap_or(false),
                },
                Err(e) => failed(req_id, "dynamic_init", e),
            }
        }
        NetworkCommand::LoadDynamicRefresh {
            req_id,
            tab,
            host_mid,
        } => {
            let feed_type = tab.get_feed_type();
            match api_client.get_dynamic_feed(None, feed_type, host_mid).await {
                Ok(data) => NetworkEvent::DynamicLoaded {
                    req_id,
                    append: false,
                    up_list: None,
                    items: data.items.unwrap_or_default(),
                    offset: data.offset,
                    has_more: data.has_more.unwrap_or(false),
                },
                Err(e) => failed(req_id, "dynamic_refresh", e),
            }
        }
        NetworkCommand::LoadDynamicMore {
            req_id,
            offset,
            tab,
            host_mid,
        } => {
            let feed_type = tab.get_feed_type();
            match api_client
                .get_dynamic_feed(Some(&offset), feed_type, host_mid)
                .await
            {
                Ok(data) => NetworkEvent::DynamicLoaded {
                    req_id,
                    append: true,
                    up_list: None,
                    items: data.items.unwrap_or_default(),
                    offset: data.offset,
                    has_more: data.has_more.unwrap_or(false),
                },
                Err(e) => failed(req_id, "dynamic_more", e),
            }
        }
        NetworkCommand::LoadHistoryInit { req_id } => {
            match api_client.get_history(None, None, None).await {
                Ok(data) => NetworkEvent::HistoryLoaded {
                    req_id,
                    append: false,
                    data,
                },
                Err(e) => failed(req_id, "history_init", e),
            }
        }
        NetworkCommand::LoadHistoryMore { req_id, cursor } => match api_client
            .get_history(
                Some(cursor.max),
                Some(cursor.view_at),
                Some(cursor.business.as_str()),
            )
            .await
        {
            Ok(data) => NetworkEvent::HistoryLoaded {
                req_id,
                append: true,
                data,
            },
            Err(e) => failed(req_id, "history_more", e),
        },
        NetworkCommand::DeleteHistory { req_id, keys } => {
            let mut successful = Vec::new();
            let mut failed_keys = Vec::new();
            for key in keys {
                match api_client.delete_history_item(&key).await {
                    Ok(()) => successful.push(key),
                    Err(error) => failed_keys.push((key, error.to_string())),
                }
            }
            NetworkEvent::HistoryDeleted {
                req_id,
                successful,
                failed: failed_keys,
            }
        }
        NetworkCommand::LoadArticle { req_id, cvid } => match api_client.get_article(cvid).await {
            Ok(article) => {
                let comment_oid = if article.id > 0 { article.id } else { cvid };
                let comments = api_client
                    .get_dynamic_comments(comment_oid, CommentType::Article.as_i32(), 1)
                    .await
                    .ok()
                    .map(|data| {
                        let mut comments = data.hots.unwrap_or_default();
                        for comment in data.replies.unwrap_or_default() {
                            if !comments
                                .iter()
                                .any(|existing| existing.rpid == comment.rpid)
                            {
                                comments.push(comment);
                            }
                        }
                        comments
                    })
                    .unwrap_or_default();
                NetworkEvent::ArticleLoaded {
                    req_id,
                    cvid,
                    article,
                    comments,
                }
            }
            Err(error) => failed(req_id, "article_detail", error),
        },
        NetworkCommand::LoadLiveInit { req_id } => {
            let rooms = match api_client.get_live_home_rooms().await {
                Ok(rooms) => Ok(rooms),
                Err(_) => api_client.get_live_recommendations().await,
            };
            match rooms {
                Ok(rooms) => NetworkEvent::LiveLoaded {
                    req_id,
                    append: false,
                    rooms,
                },
                Err(e) => failed(req_id, "live_init", e),
            }
        }
        NetworkCommand::LoadLiveMore { req_id } => {
            match api_client.get_live_recommendations().await {
                Ok(rooms) => NetworkEvent::LiveLoaded {
                    req_id,
                    append: true,
                    rooms,
                },
                Err(e) => failed(req_id, "live_more", e),
            }
        }
        NetworkCommand::SendLiveDanmaku {
            req_id,
            room_id,
            message,
        } => {
            let result = api_client
                .send_live_danmaku(room_id, message.trim())
                .await
                .map_err(|error| (message, error.to_string()));
            NetworkEvent::LiveDanmakuSent { req_id, result }
        }
        NetworkCommand::LoadVideoDetail {
            req_id,
            bvid,
            aid,
            include_related,
        } => {
            let video_info = match api_client.get_video_info(&bvid).await {
                Ok(info) => info,
                Err(e) => return failed(req_id, "video_detail", e),
            };
            let effective_aid = resolve_video_aid(video_info.aid, aid);
            let (comments, has_more_comments) =
                match api_client.get_comments(effective_aid, 1).await {
                    Ok(data) => {
                        let comments = data.replies.unwrap_or_default();
                        let has_more = data
                            .page
                            .map(|p| p.count.unwrap_or(0) > comments.len() as i32)
                            .unwrap_or(false);
                        (comments, has_more)
                    }
                    Err(_) => (Vec::new(), false),
                };
            let related_videos = if include_related {
                api_client
                    .get_related_videos(&bvid)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            NetworkEvent::VideoDetailLoaded {
                req_id,
                bvid,
                video_info,
                comments,
                has_more_comments,
                related_videos,
            }
        }
        NetworkCommand::LoadDynamicDetail { req_id, dynamic_id } => {
            let dynamic_item = match api_client.get_dynamic_detail(&dynamic_id).await {
                Ok(item) => item,
                Err(e) => return failed(req_id, "dynamic_detail", e),
            };
            let comment_type = dynamic_item.comment_type();
            let comment_oid = dynamic_item.comment_oid(&dynamic_id);
            let (comments, has_more_comments) = if let Some(oid) = comment_oid {
                match api_client.get_dynamic_comments(oid, comment_type, 1).await {
                    Ok(data) => {
                        let comments = data.replies.unwrap_or_default();
                        let has_more = data
                            .page
                            .map(|p| p.count.unwrap_or(0) > comments.len() as i32)
                            .unwrap_or(false);
                        (comments, has_more)
                    }
                    Err(_) => (Vec::new(), false),
                }
            } else {
                (Vec::new(), false)
            };
            let mut image_urls = Vec::new();
            if dynamic_item.is_draw() {
                image_urls.extend(
                    dynamic_item
                        .draw_images()
                        .into_iter()
                        .map(|s| s.to_string()),
                );
            }
            if dynamic_item.is_opus() {
                image_urls.extend(
                    dynamic_item
                        .opus_images()
                        .into_iter()
                        .map(|s| s.to_string()),
                );
            }
            NetworkEvent::DynamicDetailLoaded {
                req_id,
                dynamic_id,
                dynamic_item,
                comments,
                has_more_comments,
                image_urls,
            }
        }
        NetworkCommand::LoadBangumiIndex { req_id } => match api_client.get_bangumi_rank().await {
            Ok(items) => NetworkEvent::BangumiIndexLoaded { req_id, items },
            Err(e) => failed(req_id, "bangumi_index", e),
        },
        NetworkCommand::LoadBangumiDetail { req_id, season_id } => {
            match api_client
                .get_bangumi_season(SeasonQuery::SeasonId(season_id))
                .await
            {
                Ok(mut season) => {
                    let effective_season_id = if season.season_id > 0 {
                        season.season_id
                    } else {
                        season_id
                    };
                    if effective_season_id > 0
                        && let Ok(sections) = api_client
                            .get_bangumi_season_section(effective_season_id)
                            .await
                    {
                        season.apply_section_result(sections);
                    }
                    if let Some(media_id) = season.media_id.filter(|id| *id > 0)
                        && let Ok(review) = api_client.get_bangumi_review_user(media_id).await
                    {
                        season.review_user = Some(review);
                    }
                    NetworkEvent::BangumiDetailLoaded {
                        req_id,
                        season_id,
                        season,
                    }
                }
                Err(e) => failed(req_id, "bangumi_detail", e),
            }
        }
    }
}

async fn enrich_followers(api_client: &Arc<ApiClient>, videos: &mut [VideoItem]) {
    let mids = videos
        .iter()
        .filter_map(|video| video.owner.as_ref().map(|owner| owner.mid))
        .filter(|mid| *mid > 0)
        .collect::<std::collections::BTreeSet<_>>();
    let followers = stream::iter(mids)
        .map(|mid| {
            let client = Arc::clone(api_client);
            async move {
                let follower = client
                    .get_relation_stat(mid)
                    .await
                    .ok()
                    .and_then(|stat| stat.follower);
                (mid, follower)
            }
        })
        .buffer_unordered(6)
        .collect::<HashMap<_, _>>()
        .await;
    for video in videos {
        if let Some(owner) = video.owner.as_mut() {
            owner.follower = followers.get(&owner.mid).copied().flatten();
        }
    }
}

/// Prefer the authoritative aid from `/x/web-interface/view` over the caller's hint.
fn resolve_video_aid(video_info_aid: i64, fallback_aid: i64) -> i64 {
    if video_info_aid > 0 {
        video_info_aid
    } else {
        fallback_aid
    }
}

fn failed(req_id: u64, target: &'static str, error: anyhow::Error) -> NetworkEvent {
    let safe_error = redact_url_queries(&error.to_string());
    if let Some(mut dir) = dirs::config_dir() {
        dir.push("bilibili-tui");
        let log_path = dir.join("debug.log");
        if std::fs::create_dir_all(&dir).is_ok()
            && let Ok(mut log) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
        {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&log_path, std::fs::Permissions::from_mode(0o600));
            }
            use std::io::Write;
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let safe_log_error = redact_url_queries(&format!("{error:#}"));
            let _ = writeln!(
                log,
                "[{timestamp}] Network request failed\nTarget: {target}\nRequest ID: {req_id}\nError: {safe_log_error}\n"
            );
        }
    }

    NetworkEvent::RequestFailed {
        req_id,
        target,
        error: safe_error,
    }
}

pub(crate) fn redact_url_queries(message: &str) -> String {
    let mut output = String::with_capacity(message.len());
    let mut chars = message.chars().peekable();
    while let Some(ch) = chars.next() {
        output.push(ch);
        if ch == '?' {
            output.push_str("<redacted>");
            while chars
                .peek()
                .is_some_and(|next| !next.is_whitespace() && !matches!(next, '"' | '\''))
            {
                chars.next();
            }
        }
    }
    output
}

#[cfg(test)]
mod security_tests {
    use super::redact_url_queries;
    use super::resolve_video_aid;

    #[test]
    fn network_log_redacts_url_query_parameters() {
        let value =
            redact_url_queries("GET https://cdn.example/video?token=secret&expires=1 failed");
        assert_eq!(value, "GET https://cdn.example/video?<redacted> failed");
        assert!(!value.contains("secret"));
    }

    #[test]
    fn video_aid_prefers_authoritative_video_info() {
        assert_eq!(resolve_video_aid(42, 0), 42);
        assert_eq!(resolve_video_aid(42, 7), 42);
        assert_eq!(resolve_video_aid(0, 7), 7);
        assert_eq!(resolve_video_aid(0, 0), 0);
    }
}
