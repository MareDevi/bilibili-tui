//! Search API types and functions

use serde::Deserialize;

/// Search category supported by `/x/web-interface/wbi/search/type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchKind {
    #[default]
    Video,
    MediaBangumi,
    MediaFt,
}

impl SearchKind {
    pub const ALL: [Self; 3] = [Self::Video, Self::MediaBangumi, Self::MediaFt];

    pub fn api_value(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::MediaBangumi => "media_bangumi",
            Self::MediaFt => "media_ft",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Video => "视频",
            Self::MediaBangumi => "番剧",
            Self::MediaFt => "影视",
        }
    }

    /// Cycle to the previous / next kind, wrapping around.
    pub fn cycle(self, forward: bool) -> Self {
        let all = Self::ALL;
        let index = all.iter().position(|kind| *kind == self).unwrap_or(0);
        let next = if forward {
            (index + 1) % all.len()
        } else {
            (index + all.len() - 1) % all.len()
        };
        all[next]
    }
}

/// Remove highlight tags added by the search API to matched keywords.
pub fn strip_highlight_tags(raw: &str) -> String {
    raw.replace("<em class=\"keyword\">", "")
        .replace("</em>", "")
}

/// Search result for video type
#[derive(Debug, Deserialize)]
pub struct SearchData {
    pub result: Option<Vec<SearchVideoItem>>,
    #[serde(rename = "numResults")]
    pub num_results: Option<i32>,
    pub page: Option<i32>,
    pub pagesize: Option<i32>,
}

/// Individual video search result
#[derive(Debug, Clone, Deserialize)]
pub struct SearchVideoItem {
    pub aid: Option<i64>,
    pub bvid: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub pic: Option<String>,
    pub play: Option<i64>,
    pub duration: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "danmaku")]
    pub danmaku: Option<i64>,
    pub mid: Option<i64>,
}

impl SearchVideoItem {
    pub fn display_title(&self) -> String {
        strip_highlight_tags(self.title.as_deref().unwrap_or("无标题"))
    }

    pub fn author_name(&self) -> &str {
        self.author.as_deref().unwrap_or("未知")
    }

    pub fn format_play(&self) -> String {
        match self.play {
            Some(n) if n >= 10000 => format!("{:.1}万", n as f64 / 10000.0),
            Some(n) => format!("{}", n),
            None => "-".to_string(),
        }
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

/// Hot search item from web endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct HotwordItem {
    pub keyword: Option<String>,
    pub show_name: Option<String>,
    pub icon: Option<String>,
    pub pos: Option<i32>,
    pub word_type: Option<i32>,
}

impl HotwordItem {
    /// Display name prefers show_name over keyword
    pub fn display_text(&self) -> String {
        self.show_name
            .as_ref()
            .or(self.keyword.as_ref())
            .cloned()
            .unwrap_or_else(|| "-".to_string())
    }

    /// Keyword to trigger search
    pub fn keyword_text(&self) -> Option<String> {
        self.keyword
            .clone()
            .or_else(|| self.show_name.clone())
            .filter(|s| !s.is_empty())
    }

    /// Optional badge based on word_type
    pub fn badge(&self) -> Option<&'static str> {
        match self.word_type.unwrap_or_default() {
            4 => Some("新"),
            5 => Some("热"),
            7 => Some("直播"),
            9 => Some("梗"),
            11 => Some("话题"),
            12 => Some("独家"),
            _ => None,
        }
    }
}

/// Response for hot search list (web)
#[derive(Debug, Deserialize)]
pub struct HotwordResponse {
    pub code: Option<i32>,
    pub message: Option<String>,
    pub list: Option<Vec<HotwordItem>>, // Top 10 hot words
}

/// Search result payload for `media_bangumi` / `media_ft`.
#[derive(Debug, Deserialize)]
pub struct MediaSearchData {
    pub result: Option<Vec<MediaSearchItem>>,
    #[serde(rename = "numResults")]
    pub num_results: Option<i32>,
    pub page: Option<i32>,
    pub pagesize: Option<i32>,
}

/// Score block of a media search item.
#[derive(Debug, Clone, Deserialize)]
pub struct MediaScore {
    pub score: Option<f32>,
    pub user_count: Option<i64>,
}

/// A single bangumi / movie / TV search result.
#[derive(Debug, Clone, Deserialize)]
pub struct MediaSearchItem {
    pub media_id: Option<i64>,
    pub season_id: Option<i64>,
    pub pgc_season_id: Option<i64>,
    pub title: Option<String>,
    pub org_title: Option<String>,
    pub cover: Option<String>,
    pub areas: Option<String>,
    pub styles: Option<String>,
    pub cv: Option<String>,
    pub staff: Option<String>,
    pub desc: Option<String>,
    pub pubtime: Option<i64>,
    pub fix_pubtime_str: Option<String>,
    pub media_score: Option<MediaScore>,
    pub season_type: Option<i32>,
    pub season_type_name: Option<String>,
    pub ep_size: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub eps: Vec<MediaSearchEpisode>,
    pub url: Option<String>,
    pub goto_url: Option<String>,
    pub angle_title: Option<String>,
    pub is_follow: Option<i32>,
}

impl MediaSearchItem {
    pub fn display_title(&self) -> String {
        strip_highlight_tags(self.title.as_deref().unwrap_or("无标题"))
    }

    /// Season identifier accepted by the bangumi detail page.
    pub fn resolved_season_id(&self) -> Option<i64> {
        self.season_id
            .filter(|&id| id > 0)
            .or_else(|| self.pgc_season_id.filter(|&id| id > 0))
    }

    pub fn cover_url(&self) -> Option<String> {
        self.cover.as_ref().map(|url| {
            if url.starts_with("//") {
                format!("https:{}", url)
            } else {
                url.clone()
            }
        })
    }

    pub fn display_subtitle(&self) -> String {
        join_non_empty([
            self.season_type_name.as_deref(),
            self.areas.as_deref(),
            self.styles.as_deref(),
        ])
    }

    pub fn score_text(&self) -> String {
        match self.media_score.as_ref().and_then(|score| score.score) {
            Some(score) if score > 0.0 => format!("{:.1}分", score),
            _ => "-".to_string(),
        }
    }

    pub fn meta_text(&self) -> String {
        let ep_size = self
            .ep_size
            .filter(|&count| count > 0)
            .map(|count| format!("共{}话", count));
        let followed = (self.is_follow == Some(1)).then_some("[已追番]");
        join_non_empty([ep_size.as_deref(), self.angle_title.as_deref(), followed])
    }
}

/// One episode entry of a media search result.
#[derive(Debug, Clone, Deserialize)]
pub struct MediaSearchEpisode {
    pub id: Option<i64>,
    pub cover: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub index_title: Option<String>,
    pub long_title: Option<String>,
}

/// Join the non-empty values with ` · `.
fn join_non_empty<const N: usize>(values: [Option<&str>; N]) -> String {
    values
        .into_iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Deserialize `null` (and missing values) as an empty vector.
fn deserialize_vec_or_default<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn media_item() -> MediaSearchItem {
        serde_json::from_value(json!({
            "media_id": 28223075,
            "season_id": 44219,
            "pgc_season_id": 44219,
            "title": "鬼灭之刃<em class=\"keyword\">番剧</em>",
            "org_title": "Kimetsu no Yaiba",
            "cover": "//i0.hdslb.com/bfs/bangumi/cover.jpg",
            "areas": "日本",
            "styles": "热血/战斗",
            "cv": "花江夏树",
            "staff": "导演：外崎春雄",
            "desc": "少年与鬼的故事",
            "pubtime": 1567267200_i64,
            "fix_pubtime_str": "2019",
            "media_score": {"score": 9.8, "user_count": 12345},
            "season_type": 1,
            "season_type_name": "番剧",
            "ep_size": 26,
            "eps": [{
                "id": 1,
                "cover": "//i0.hdslb.com/bfs/bangumi/ep.jpg",
                "title": "1",
                "url": "https://www.bilibili.com/bangumi/play/ep1",
                "index_title": "1",
                "long_title": "残酷"
            }],
            "url": "https://www.bilibili.com/bangumi/play/ss44219",
            "goto_url": "bilibili://bangumi/season/44219",
            "angle_title": "",
            "is_follow": 1
        }))
        .expect("full media fixture")
    }

    #[test]
    fn media_search_data_deserializes_full_response() {
        let data: MediaSearchData = serde_json::from_value(json!({
            "result": [{
                "media_id": 28223075,
                "season_id": 44219,
                "title": "鬼灭之刃<em class=\"keyword\">番剧</em>",
                "cover": "//i0.hdslb.com/bfs/bangumi/cover.jpg",
                "areas": "日本",
                "styles": "热血/战斗",
                "media_score": {"score": 9.8, "user_count": 12345},
                "season_type_name": "番剧",
                "ep_size": 26,
                "is_follow": 1
            }],
            "numResults": 42,
            "page": 1,
            "pagesize": 20
        }))
        .expect("media search response");

        assert_eq!(data.num_results, Some(42));
        assert_eq!(data.page, Some(1));
        assert_eq!(data.pagesize, Some(20));
        let item = data.result.expect("result list").pop().expect("item");
        assert_eq!(item.display_title(), "鬼灭之刃番剧");
        assert_eq!(item.resolved_season_id(), Some(44219));
        assert_eq!(
            item.cover_url().as_deref(),
            Some("https://i0.hdslb.com/bfs/bangumi/cover.jpg")
        );
        assert_eq!(item.display_subtitle(), "番剧 · 日本 · 热血/战斗");
        assert_eq!(item.score_text(), "9.8分");
        assert_eq!(item.meta_text(), "共26话 · [已追番]");
    }

    #[test]
    fn media_search_item_helpers_cover_full_fixture() {
        let item = media_item();
        assert_eq!(item.display_title(), "鬼灭之刃番剧");
        assert_eq!(item.eps.len(), 1);
        assert_eq!(item.eps[0].long_title.as_deref(), Some("残酷"));
        assert_eq!(item.eps[0].index_title.as_deref(), Some("1"));
        assert_eq!(item.meta_text(), "共26话 · [已追番]");
    }

    #[test]
    fn media_search_item_tolerates_null_score_eps_and_missing_fields() {
        let data: MediaSearchData = serde_json::from_value(json!({
            "result": [{
                "season_id": 7,
                "title": "无评分",
                "media_score": null,
                "eps": null
            }],
            "numResults": 1,
            "page": 1,
            "pagesize": 20
        }))
        .expect("null-safe media response");
        let item = data.result.expect("result list").pop().expect("item");
        assert!(item.eps.is_empty());
        assert_eq!(item.score_text(), "-");
        assert_eq!(item.meta_text(), "");
        assert_eq!(item.display_subtitle(), "");
        assert_eq!(item.cover_url(), None);
    }

    #[test]
    fn media_search_item_falls_back_to_pgc_season_id() {
        let item: MediaSearchItem = serde_json::from_value(json!({
            "season_id": 0,
            "pgc_season_id": 99,
            "title": "fallback"
        }))
        .expect("pgc fixture");
        assert_eq!(item.resolved_season_id(), Some(99));
    }

    #[test]
    fn highlight_tags_are_stripped_from_titles() {
        assert_eq!(
            strip_highlight_tags("a<em class=\"keyword\">b</em>c"),
            "abc"
        );
        assert_eq!(SearchKind::Video.api_value(), "video");
        assert_eq!(SearchKind::MediaBangumi.api_value(), "media_bangumi");
        assert_eq!(SearchKind::MediaFt.api_value(), "media_ft");
        assert_eq!(SearchKind::MediaBangumi.label(), "番剧");
        assert_eq!(SearchKind::Video.cycle(false), SearchKind::MediaFt);
        assert_eq!(SearchKind::MediaFt.cycle(true), SearchKind::Video);
    }
}
