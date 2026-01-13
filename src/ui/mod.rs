mod dynamic;
mod dynamic_detail;
mod history;
mod home;
mod login;
mod search;
mod settings;
mod sidebar;
pub mod theme;
mod video_card;
mod video_detail;

pub use dynamic::{DynamicPage, DynamicTab};
pub use dynamic_detail::DynamicDetailPage;
pub use history::HistoryPage;
pub use home::HomePage;
pub use login::LoginPage;
pub use search::SearchPage;
pub use settings::SettingsPage;
pub use sidebar::{NavItem, Sidebar};
pub use theme::{Theme, ThemeVariant};
pub use video_card::{VideoCard, VideoCardGrid};
pub use video_detail::VideoDetailPage;

use crate::app::AppAction;
use crate::storage::Keybindings;
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers, MouseEvent},
    prelude::Rect,
    Frame,
};

/// UI Component trait
pub trait Component {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings);
    fn handle_input(&mut self, key: KeyCode, keys: &Keybindings) -> Option<AppAction> {
        let _ = (key, keys);
        None
    }
    fn handle_input_with_modifiers(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
        keys: &Keybindings,
    ) -> Option<AppAction> {
        let _ = modifiers;
        self.handle_input(key, keys)
    }
    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        let _ = (event, area);
        None
    }
}

/// Application pages
pub enum Page {
    Login(LoginPage),
    Home(HomePage),
    Search(SearchPage),
    Dynamic(DynamicPage),
    DynamicDetail(Box<DynamicDetailPage>),
    VideoDetail(Box<VideoDetailPage>),
    History(HistoryPage),
    Settings(Box<SettingsPage>),
}
