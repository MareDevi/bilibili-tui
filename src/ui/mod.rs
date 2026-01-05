mod dynamic;
mod home;
mod login;
mod search;
mod settings;
mod sidebar;
pub mod theme;
mod video_card;
mod video_detail;

pub use dynamic::{DynamicPage, DynamicTab};
pub use home::HomePage;
pub use login::LoginPage;
pub use search::SearchPage;
pub use settings::SettingsPage;
pub use sidebar::{NavItem, Sidebar};
pub use theme::{Theme, ThemeVariant};
pub use video_card::{VideoCard, VideoCardGrid};
pub use video_detail::VideoDetailPage;

use crate::app::AppAction;
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::Rect,
    Frame,
};

/// UI Component trait
pub trait Component {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction> {
        self.handle_input_with_modifiers(key, KeyModifiers::empty())
    }
    fn handle_input_with_modifiers(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<AppAction> {
        let _ = modifiers;
        self.handle_input(key)
    }
}

/// Application pages
pub enum Page {
    Login(LoginPage),
    Home(HomePage),
    Search(SearchPage),
    Dynamic(DynamicPage),
    VideoDetail(Box<VideoDetailPage>),
    Settings(SettingsPage),
}
