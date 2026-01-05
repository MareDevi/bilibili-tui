use crate::storage::Credentials;
use crate::ui::ThemeVariant;

/// Actions that can be triggered from UI components
#[derive(Debug, Clone)]
pub enum AppAction {
    /// Quit the application
    Quit,
    /// Switch to home page
    SwitchToHome,
    /// Switch to login page
    SwitchToLogin,
    /// Switch to settings page
    SwitchToSettings,
    /// Login was successful with credentials
    LoginSuccess(Credentials),
    /// Play a video by bvid
    PlayVideo(String),
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
    /// Go back to previous page
    BackToList,
    /// Load more recommendations
    LoadMoreRecommendations,
    /// Load more search results
    LoadMoreSearch,
    /// Load more dynamic items
    LoadMoreDynamic,
    /// Load more comments in video detail page
    LoadMoreComments,
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
    /// No action
    None,
}
