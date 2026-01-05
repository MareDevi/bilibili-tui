mod dynamic;
mod home;
mod login;
mod search;
mod sidebar;
mod video_card;
mod video_detail;

pub use dynamic::DynamicPage;
pub use home::HomePage;
pub use login::LoginPage;
pub use search::SearchPage;
pub use sidebar::{NavItem, Sidebar};
pub use video_card::{VideoCard, VideoCardGrid};
pub use video_detail::VideoDetailPage;

use crate::app::AppAction;
use ratatui::{crossterm::event::KeyCode, Frame, prelude::Rect};

/// UI Component trait
pub trait Component {
    fn draw(&mut self, frame: &mut Frame, area: Rect);
    fn handle_input(&mut self, key: KeyCode) -> Option<AppAction>;
}

/// Application pages
pub enum Page {
    Login(LoginPage),
    Home(HomePage),
    Search(SearchPage),
    Dynamic(DynamicPage),
    VideoDetail(VideoDetailPage),
}

