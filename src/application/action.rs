use crate::api::bangumi::{BangumiRankDay, BangumiSeasonType, SeasonIndexQuery, TimelineType};
use crate::api::favorite::{FavoriteOrder, FavoriteSource};
use crate::api::history::HistoryKey;
use crate::api::recommend::HomeFeed;
use crate::api::search::SearchKind;
use crate::api::space::SpaceVideoOrder;
use crate::api::video::VideoPage;
use crate::domain::playback::{PlayOrder, PlaylistItem, PlaylistSource};
use crate::infrastructure::persistence::{Credentials, DanmakuConfig, Keybindings, VideoQuality};
use crate::presentation::tui::DynamicTab;

/// Actions that can be triggered from UI components
#[derive(Debug, Clone)]
pub enum AppAction {
    /// Quit the application
    Quit,
    /// Switch to home page
    SwitchToHome,
    /// Refresh home page recommendations (force reload)
    RefreshHome,
    SwitchHomeFeed(HomeFeed),
    /// Switch to login page
    SwitchToLogin,
    /// Switch to settings page
    SwitchToSettings,
    /// Switch to history page
    SwitchToHistory,
    /// Login was successful with credentials
    LoginSuccess(Credentials),
    /// Play a video with metadata (bvid, aid, cid, duration)
    PlayVideo {
        bvid: String,
        aid: i64,
        cid: i64,
        duration: i64,
    },
    /// Play a video with page info for auto-play next episode
    PlayVideoWithPages {
        bvid: String,
        aid: i64,
        pages: Vec<VideoPage>,
        current_index: usize,
    },
    PlayPlaylist {
        items: Vec<PlaylistItem>,
        source: PlaylistSource,
        start_index: usize,
        order: PlayOrder,
    },
    PlayUpAll {
        mid: i64,
        name: String,
        video_order: SpaceVideoOrder,
        play_order: PlayOrder,
    },
    PlayFavoriteAll {
        media_id: i64,
        title: String,
        favorite_order: FavoriteOrder,
        play_order: PlayOrder,
    },
    /// Navigate to next sidebar item
    NavNext,
    /// Navigate to previous sidebar item
    NavPrev,
    CancelPendingLoads,
    /// Search for videos
    Search(String),
    /// Switch the search category (video / bangumi / movie-TV)
    SwitchSearchKind(SearchKind),
    /// Refresh dynamic feed
    RefreshDynamic,
    /// Open video detail page (bvid, aid)
    OpenVideoDetail(String, i64),
    /// Open an uploader's public space by member ID.
    OpenUpPage(i64),
    RefreshUpPage,
    SwitchUpVideoOrder(SpaceVideoOrder),
    LoadMoreUpVideos,
    OpenFavoriteFolder(i64),
    SwitchFavoriteOrder(FavoriteOrder),
    LoadMoreFavoriteResources,
    SelectFavoriteSource(FavoriteSource),
    LoadMoreFavorites,
    /// Open dynamic detail page for image/text dynamics (dynamic_id)
    OpenDynamicDetail(String),
    /// Go back to previous page
    BackToList,
    /// Load more recommendations
    LoadMoreRecommendations,
    /// Load more search results
    LoadMoreSearch,
    /// Load more dynamic items
    LoadMoreDynamic,
    /// Load more history items
    LoadMoreHistory,
    DeleteHistoryItems(Vec<HistoryKey>),
    OpenArticle(i64),
    OpenHistoryBangumi {
        season_id: i64,
        ep_id: i64,
    },
    /// Load more comments in video detail page
    LoadMoreComments,
    /// Toggle comment replies expansion
    ToggleCommentReplies,
    /// Switch dynamic tab
    SwitchDynamicTab(DynamicTab),
    /// Select UP master (0 = all, 1+ = specific UP)
    SelectUpMaster(usize),
    /// Switch to next theme variant
    NextTheme,
    /// Set a specific theme by Opaline theme ID
    SetTheme(String),
    /// Save keybindings to config
    SaveKeybindings(Box<Keybindings>),
    /// Save live/video danmaku rendering settings.
    SaveDanmakuConfig(Box<DanmakuConfig>),
    /// Save the auto-play-on-video-open preference.
    SaveAutoPlay(bool),
    SaveMinimalMode(bool),
    SaveVideoQuality(VideoQuality),
    /// Logout and return to login page
    Logout,
    /// Like or unlike a comment (oid, rpid, comment_type)
    LikeComment {
        oid: i64,
        rpid: i64,
        comment_type: i32,
    },
    /// Add a comment (oid, comment_type, message, optional root rpid for replies)
    AddComment {
        oid: i64,
        comment_type: i32,
        message: String,
        root: Option<i64>,
    },
    /// Switch to live page
    SwitchToLive,
    /// Open live room detail
    OpenLiveDetail(i64),
    /// Refresh live recommendations
    RefreshLive,
    /// Load more live rooms
    LoadMoreLive,
    SendLiveDanmaku {
        room_id: i64,
        message: String,
    },
    /// Play live stream
    PlayLive {
        room_id: i64,
        title: String,
    },
    /// Switch to bangumi page
    SwitchToBangumi,
    /// Refresh bangumi timeline
    RefreshBangumi,
    /// Switch bangumi tab
    SwitchBangumiTab(BangumiTab),
    /// Load the ranked bangumi list for a window and season type
    LoadBangumiRank {
        day: BangumiRankDay,
        season_type: BangumiSeasonType,
    },
    /// Load the bangumi timeline
    LoadBangumiTimeline {
        types: TimelineType,
        before: u8,
        after: u8,
    },
    /// Load a season index page, appending when `append` is set
    LoadSeasonIndex {
        query: SeasonIndexQuery,
        append: bool,
    },
    /// Open bangumi detail page
    OpenBangumiDetail(i64),
    /// Open a bangumi episode directly from the timeline (no auto-play)
    OpenBangumiEpisode {
        season_id: i64,
        ep_id: i64,
    },
    /// Load more bangumi index items
    LoadMoreBangumi,
    /// Play a bangumi episode
    PlayBangumiEpisode {
        ep_id: i64,
        season_id: i64,
        title: String,
    },
    /// Load a page of bangumi episode comments (aid, page)
    LoadBangumiComments {
        aid: i64,
        page: i32,
    },
    /// No action
    None,
}

/// Bangumi page tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BangumiTab {
    Rank,
    Timeline,
    Index,
}
