//! Bilibili Live Streaming API types and utilities

use serde::Deserialize;

/// Live room recommendation item from getMoreRecList API
#[derive(Debug, Clone, Deserialize)]
pub struct LiveRoom {
    pub roomid: i64,
    pub uid: i64,
    pub title: String,
    pub uname: String,
    pub face: String,
    pub cover: String,
    #[serde(default)]
    pub keyframe: String,
    pub online: i64,
    #[serde(default)]
    pub area_v2_name: String,
    #[serde(default)]
    pub area_v2_parent_name: String,
    #[serde(default)]
    pub watched_show: Option<WatchedShow>,
}

/// Watched count display info
#[derive(Debug, Clone, Deserialize)]
pub struct WatchedShow {
    pub num: i64,
    #[serde(default)]
    pub text_small: String,
}

/// Live recommendations response data
#[derive(Debug, Deserialize)]
pub struct LiveRecommendData {
    #[serde(default)]
    pub recommend_room_list: Vec<LiveRoom>,
}

/// Live room detailed info from get_info API
#[derive(Debug, Clone, Deserialize)]
pub struct LiveRoomInfo {
    pub uid: i64,
    pub room_id: i64,
    #[serde(default)]
    pub short_id: i64,
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// 0=未开播, 1=直播中, 2=轮播中
    pub live_status: i32,
    #[serde(default)]
    pub area_id: i64,
    #[serde(default)]
    pub area_name: String,
    #[serde(default)]
    pub parent_area_name: String,
    /// 关注数
    #[serde(default)]
    pub attention: i64,
    /// 在线人数
    #[serde(default)]
    pub online: i64,
    #[serde(default)]
    pub user_cover: String,
    #[serde(default)]
    pub keyframe: String,
    #[serde(default)]
    pub live_time: String,
    #[serde(default)]
    pub tags: String,
}

impl LiveRoomInfo {
    /// Get display cover URL
    pub fn cover_url(&self) -> &str {
        if !self.user_cover.is_empty() {
            &self.user_cover
        } else if !self.keyframe.is_empty() {
            &self.keyframe
        } else {
            ""
        }
    }

    /// Get live status text
    pub fn status_text(&self) -> &'static str {
        match self.live_status {
            0 => "未开播",
            1 => "直播中",
            2 => "轮播中",
            _ => "未知",
        }
    }
}
