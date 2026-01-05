//! Video info API types

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct VideoInfo {
    pub bvid: String,
    pub aid: i64,
    pub cid: i64,
    pub title: String,
    pub desc: Option<String>,
    pub pic: Option<String>,
    pub duration: Option<i64>,
    pub pubdate: Option<i64>,
    pub owner: VideoOwner,
    pub stat: VideoStat,
    pub pages: Option<Vec<VideoPage>>,
}

#[derive(Debug, Deserialize)]
pub struct VideoOwner {
    pub mid: i64,
    pub name: String,
    pub face: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideoStat {
    pub view: i64,
    pub danmaku: i64,
    pub like: i64,
    pub coin: i64,
    pub favorite: i64,
    pub share: i64,
    pub reply: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct VideoPage {
    pub cid: i64,
    pub page: i32,
    pub part: String,
    pub duration: i64,
}

/// Related video item from /x/web-interface/archive/related
#[derive(Debug, Clone, Deserialize)]
pub struct RelatedVideoItem {
    pub aid: Option<i64>,
    pub bvid: Option<String>,
    pub cid: Option<i64>,
    pub title: Option<String>,
    pub pic: Option<String>,
    pub duration: Option<i64>,
    pub owner: Option<RelatedVideoOwner>,
    pub stat: Option<RelatedVideoStat>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelatedVideoOwner {
    pub mid: Option<i64>,
    pub name: Option<String>,
    pub face: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelatedVideoStat {
    pub view: Option<i64>,
    pub danmaku: Option<i64>,
}

impl RelatedVideoItem {
    pub fn author_name(&self) -> &str {
        self.owner
            .as_ref()
            .and_then(|o| o.name.as_deref())
            .unwrap_or("-")
    }

    pub fn format_duration(&self) -> String {
        if let Some(duration) = self.duration {
            let minutes = duration / 60;
            let seconds = duration % 60;
            format!("{:02}:{:02}", minutes, seconds)
        } else {
            "--:--".to_string()
        }
    }

    pub fn format_views(&self) -> String {
        self.stat
            .as_ref()
            .and_then(|s| s.view)
            .map(|view| {
                if view >= 10000 {
                    format!("{:.1}万", view as f64 / 10000.0)
                } else {
                    view.to_string()
                }
            })
            .unwrap_or("-".to_string())
    }

    pub fn cover_url(&self) -> Option<String> {
        self.pic.as_ref().map(|url| {
            if url.starts_with("//") {
                format!("https:{}", url)
            } else {
                url.clone()
            }
        })
    }
}

impl VideoStat {
    pub fn format_views(&self) -> String {
        if self.view >= 10000 {
            format!("{:.1}万", self.view as f64 / 10000.0)
        } else {
            self.view.to_string()
        }
    }

    pub fn format_danmaku(&self) -> String {
        if self.danmaku >= 10000 {
            format!("{:.1}万", self.danmaku as f64 / 10000.0)
        } else {
            self.danmaku.to_string()
        }
    }

    pub fn format_like(&self) -> String {
        if self.like >= 10000 {
            format!("{:.1}万", self.like as f64 / 10000.0)
        } else {
            self.like.to_string()
        }
    }

    pub fn format_coin(&self) -> String {
        if self.coin >= 10000 {
            format!("{:.1}万", self.coin as f64 / 10000.0)
        } else {
            self.coin.to_string()
        }
    }

    pub fn format_favorite(&self) -> String {
        if self.favorite >= 10000 {
            format!("{:.1}万", self.favorite as f64 / 10000.0)
        } else {
            self.favorite.to_string()
        }
    }

    pub fn format_reply(&self) -> String {
        match self.reply {
            Some(n) if n >= 10000 => format!("{:.1}万", n as f64 / 10000.0),
            Some(n) => n.to_string(),
            None => "-".to_string(),
        }
    }
}

