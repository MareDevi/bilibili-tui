use crate::storage::Credentials;
use crate::ui::ThemeVariant;

/// Actions that can be triggered from UI components
#[derive(Debug, Clone)]
pub enum AppAction {
    /// Quit the application
    Quit,
    /// Switch to home page
    SwitchToHome,
    /// Refresh home page recommendations (force reload)
    RefreshHome,
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
    /// Navigate to next sidebar item
    NavNext,
    /// Navigate to previous sidebar item
    NavPrev,
    /// Search for videos
    Search(String),
    /// Refresh dynamic feed
    RefreshDynamic,
    /// Open video detail page (bvid, aid)
    OpenVideoDetail(String, i64),
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
    /// Load more comments in video detail page
    LoadMoreComments,
    /// Toggle comment replies expansion
    ToggleCommentReplies,
    /// Switch dynamic tab
    SwitchDynamicTab(crate::ui::DynamicTab),
    /// Select UP master (0 = all, 1+ = specific UP)
    SelectUpMaster(usize),
    /// Switch to next theme variant
    NextTheme,
    /// Set a specific theme
    SetTheme(ThemeVariant),
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
    /// No action
    None,
}
