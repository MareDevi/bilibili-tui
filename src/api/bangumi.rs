//! Bangumi (番剧/影视) API types

use serde::Deserialize;

// ─── Shared PGC enums ───────────────────────────────────────────────────────

/// Season type (`season_type`) shared by the season index, rank list and
/// timeline endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BangumiSeasonType {
    #[default]
    Anime,
    Movie,
    Documentary,
    Guochuang,
    Tv,
    Variety,
}

impl BangumiSeasonType {
    pub const ALL: [Self; 6] = [
        Self::Anime,
        Self::Movie,
        Self::Documentary,
        Self::Guochuang,
        Self::Tv,
        Self::Variety,
    ];

    pub fn as_i32(self) -> i32 {
        match self {
            Self::Anime => 1,
            Self::Movie => 2,
            Self::Documentary => 3,
            Self::Guochuang => 4,
            Self::Tv => 5,
            Self::Variety => 7,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Anime => "番剧",
            Self::Movie => "电影",
            Self::Documentary => "纪录片",
            Self::Guochuang => "国创",
            Self::Tv => "电视剧",
            Self::Variety => "综艺",
        }
    }

    /// Movie / documentary / TV rows filter by `release_date`; the rest by
    /// `year`.
    pub fn uses_release_date(self) -> bool {
        matches!(self, Self::Movie | Self::Documentary | Self::Tv)
    }
}

/// Ranking window (`day`) for `/pgc/web/rank/list`.
///
/// The endpoint is not documented in `bilibili-API-collect`; it was observed
/// from the Bilibili web rank page. Live probing confirmed only `day=3` and
/// `day=7` succeed (`code=0`), whereas `day=1` and `day=30` return `code=-400`.
/// `season_type` accepts 1/2/3/4/5/7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BangumiRankDay {
    #[default]
    ThreeDays,
    Week,
}

impl BangumiRankDay {
    pub const ALL: [Self; 2] = [Self::ThreeDays, Self::Week];

    pub fn as_i32(self) -> i32 {
        match self {
            Self::ThreeDays => 3,
            Self::Week => 7,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ThreeDays => "三日榜",
            Self::Week => "周榜",
        }
    }
}

/// Timeline season type (`types`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TimelineType {
    #[default]
    Anime,
    Movie,
    Guochuang,
}

impl TimelineType {
    pub const ALL: [Self; 3] = [Self::Anime, Self::Movie, Self::Guochuang];

    pub fn as_i32(self) -> i32 {
        match self {
            Self::Anime => 1,
            Self::Movie => 3,
            Self::Guochuang => 4,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Anime => "番剧",
            Self::Movie => "电影",
            Self::Guochuang => "国创",
        }
    }
}

/// Sort field (`order`) for `/pgc/season/index/result`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SeasonIndexOrder {
    UpdateTime,
    Danmaku,
    Play,
    #[default]
    Follow,
    Score,
    PubDate,
    ReleaseDate,
}

impl SeasonIndexOrder {
    pub const ALL: [Self; 7] = [
        Self::UpdateTime,
        Self::Danmaku,
        Self::Play,
        Self::Follow,
        Self::Score,
        Self::PubDate,
        Self::ReleaseDate,
    ];

    pub fn as_i32(self) -> i32 {
        match self {
            Self::UpdateTime => 0,
            Self::Danmaku => 1,
            Self::Play => 2,
            Self::Follow => 3,
            Self::Score => 4,
            Self::PubDate => 5,
            Self::ReleaseDate => 6,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::UpdateTime => "更新时间",
            Self::Danmaku => "弹幕数量",
            Self::Play => "播放数量",
            Self::Follow => "追番人数",
            Self::Score => "最高评分",
            Self::PubDate => "开播时间",
            Self::ReleaseDate => "上映时间",
        }
    }
}

/// Sort direction (`sort`) for `/pgc/season/index/result`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SeasonIndexSort {
    #[default]
    Desc,
    Asc,
}

impl SeasonIndexSort {
    pub fn as_i32(self) -> i32 {
        match self {
            Self::Desc => 0,
            Self::Asc => 1,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Desc => "降序",
            Self::Asc => "升序",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Desc => Self::Asc,
            Self::Asc => Self::Desc,
        }
    }
}

// ─── Timeline ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TimelineDay {
    #[serde(default)]
    pub date: String,
    #[serde(rename = "date_ts", default)]
    pub date_ts: i64,
    #[serde(rename = "day_of_week", default)]
    pub day_of_week: i32,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub episodes: Vec<TimelineEpisode>,
    #[serde(rename = "is_today", default)]
    pub is_today: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimelineEpisode {
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub delay: i32,
    #[serde(rename = "delay_id", default)]
    pub delay_id: i64,
    #[serde(rename = "delay_index", default)]
    pub delay_index: String,
    #[serde(rename = "delay_reason", default)]
    pub delay_reason: String,
    #[serde(rename = "ep_cover", default)]
    pub ep_cover: String,
    #[serde(rename = "episode_id", default)]
    pub episode_id: i64,
    #[serde(default)]
    pub follows: String,
    #[serde(default)]
    pub plays: String,
    #[serde(rename = "pub_index", default)]
    pub pub_index: String,
    #[serde(rename = "pub_time", default)]
    pub pub_time: String,
    #[serde(rename = "pub_ts", default)]
    pub pub_ts: i64,
    #[serde(default)]
    pub published: i32,
    #[serde(rename = "season_id", default)]
    pub season_id: i64,
    #[serde(rename = "square_cover", default)]
    pub square_cover: String,
    #[serde(default)]
    pub title: String,
}

impl TimelineEpisode {
    pub fn cover_url(&self) -> String {
        let url = if self.ep_cover.is_empty() {
            &self.cover
        } else {
            &self.ep_cover
        };
        if url.starts_with("//") {
            format!("https:{}", url)
        } else {
            url.clone()
        }
    }
}

// ─── Season Rank (replaces index) ───────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SeasonRankItem {
    pub badge: String,
    pub cover: String,
    #[serde(rename = "new_ep")]
    pub new_ep: Option<NewEp>,
    #[serde(rename = "season_id")]
    pub season_id: i64,
    pub title: String,
    pub rating: Option<String>,
    pub stat: Option<RankStat>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewEp {
    #[serde(rename = "index_show")]
    pub index_show: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RankStat {
    pub view: Option<i64>,
}

impl SeasonRankItem {
    pub fn cover_url(&self) -> String {
        if self.cover.starts_with("//") {
            format!("https:{}", self.cover)
        } else {
            self.cover.clone()
        }
    }

    pub fn display_title(&self) -> String {
        self.title.clone()
    }

    pub fn display_subtitle(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref ep) = self.new_ep
            && !ep.index_show.is_empty()
        {
            parts.push(ep.index_show.clone());
        }
        if let Some(ref stat) = self.stat
            && let Some(view) = stat.view
        {
            parts.push(format_views(view));
        }
        if parts.is_empty() {
            "-".to_string()
        } else {
            parts.join(" · ")
        }
    }

    pub fn badge_text(&self) -> Option<&str> {
        if self.badge.is_empty() {
            None
        } else {
            Some(&self.badge)
        }
    }

    pub fn score_text(&self) -> String {
        self.rating.clone().unwrap_or_default()
    }
}

fn format_views(view: i64) -> String {
    if view >= 100000000 {
        format!("{:.1}亿", view as f64 / 100000000.0)
    } else if view >= 10000 {
        format!("{:.1}万", view as f64 / 10000.0)
    } else {
        view.to_string()
    }
}

// ─── Season Index ───────────────────────────────────────────────────────────

/// Query for `GET /pgc/season/index/result`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeasonIndexQuery {
    pub season_type: BangumiSeasonType,
    pub page: i32,
    pub pagesize: i32,
    pub season_version: i32,
    pub spoken_language_type: i32,
    pub area: String,
    pub year: String,
    pub is_finish: i32,
    pub copyright: String,
    pub season_status: String,
    pub season_month: i32,
    pub order: SeasonIndexOrder,
    pub sort: SeasonIndexSort,
    pub style_id: i32,
    pub producer_id: i32,
}

impl Default for SeasonIndexQuery {
    fn default() -> Self {
        Self {
            season_type: BangumiSeasonType::Anime,
            page: 1,
            pagesize: 20,
            season_version: -1,
            spoken_language_type: -1,
            area: "-1".to_string(),
            year: "-1".to_string(),
            is_finish: -1,
            copyright: "-1".to_string(),
            season_status: "-1".to_string(),
            season_month: -1,
            order: SeasonIndexOrder::Follow,
            sort: SeasonIndexSort::Desc,
            style_id: -1,
            producer_id: -1,
        }
    }
}

impl SeasonIndexQuery {
    /// Builds the full query string pairs. `year` becomes `release_date` for
    /// season types whose catalogue filters by release date.
    pub fn to_query_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![
            ("type", "1".to_string()),
            ("season_type", self.season_type.as_i32().to_string()),
            ("st", self.season_type.as_i32().to_string()),
            ("page", self.page.to_string()),
            ("pagesize", self.pagesize.to_string()),
            ("season_version", self.season_version.to_string()),
            (
                "spoken_language_type",
                self.spoken_language_type.to_string(),
            ),
            ("area", self.area.clone()),
            ("is_finish", self.is_finish.to_string()),
            ("copyright", self.copyright.clone()),
            ("season_status", self.season_status.clone()),
            ("season_month", self.season_month.to_string()),
            ("order", self.order.as_i32().to_string()),
            ("sort", self.sort.as_i32().to_string()),
            ("style_id", self.style_id.to_string()),
            ("producer_id", self.producer_id.to_string()),
        ];
        if self.season_type.uses_release_date() {
            params.push(("release_date", format_release_date(&self.year)));
        } else {
            params.push(("year", self.year.clone()));
        }
        params
    }
}

/// Converts a `[2025,2026)` style interval into the `release_date` format the
/// endpoint expects: `[2025-01-01 00:00:00,2026-01-01 00:00:00)`.
fn format_release_date(year: &str) -> String {
    let trimmed = year.trim();
    if trimmed.is_empty() {
        return "-1".to_string();
    }
    if trimmed == "-1" || trimmed.contains(' ') {
        return trimmed.to_string();
    }
    let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(')'))
    else {
        return trimmed.to_string();
    };
    let Some((left, right)) = inner.split_once(',') else {
        return trimmed.to_string();
    };
    format!("[{},{})", expand_year_bound(left), expand_year_bound(right))
}

fn expand_year_bound(bound: &str) -> String {
    let bound = bound.trim();
    if bound.is_empty() {
        return String::new();
    }
    if bound.len() == 4 && bound.chars().all(|ch| ch.is_ascii_digit()) {
        format!("{bound}-01-01 00:00:00")
    } else {
        bound.to_string()
    }
}

fn deserialize_string_or_number_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(deserialize_string_or_number(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexFirstEp {
    #[serde(rename = "ep_id", default)]
    pub ep_id: i64,
    #[serde(default)]
    pub cover: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeasonIndexItem {
    #[serde(rename = "season_id", default)]
    pub season_id: i64,
    #[serde(rename = "media_id", default)]
    pub media_id: i64,
    #[serde(default)]
    pub title: String,
    #[serde(rename = "subTitle", default)]
    pub sub_title: String,
    #[serde(default)]
    pub cover: String,
    #[serde(rename = "index_show", default)]
    pub index_show: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number_default")]
    pub order: String,
    #[serde(
        rename = "order_type",
        default,
        deserialize_with = "deserialize_string_or_number_default"
    )]
    pub order_type: String,
    #[serde(default, deserialize_with = "deserialize_string_or_number_default")]
    pub score: String,
    #[serde(default)]
    pub badge: String,
    #[serde(rename = "badge_info", default)]
    pub badge_info: Option<BadgeInfo>,
    #[serde(rename = "season_status", default)]
    pub season_status: Option<i32>,
    #[serde(rename = "season_type", default)]
    pub season_type: Option<i32>,
    #[serde(rename = "is_finish", default)]
    pub is_finish: Option<i32>,
    #[serde(default)]
    pub link: String,
    #[serde(rename = "first_ep", default)]
    pub first_ep: Option<IndexFirstEp>,
}

impl SeasonIndexItem {
    pub fn cover_url(&self) -> String {
        if self.cover.starts_with("//") {
            format!("https:{}", self.cover)
        } else {
            self.cover.clone()
        }
    }

    pub fn display_subtitle(&self) -> String {
        if !self.sub_title.is_empty() {
            self.sub_title.clone()
        } else if !self.index_show.is_empty() {
            self.index_show.clone()
        } else {
            "-".to_string()
        }
    }

    pub fn badge_text(&self) -> Option<&str> {
        if !self.badge.is_empty() {
            return Some(&self.badge);
        }
        self.badge_info
            .as_ref()
            .and_then(|info| info.text.as_deref())
            .filter(|text| !text.is_empty())
    }

    pub fn score_text(&self) -> String {
        if self.score.is_empty() {
            "-".to_string()
        } else {
            self.score.clone()
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeasonIndexPage {
    #[serde(rename = "has_next", default)]
    pub has_next: i32,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub list: Vec<SeasonIndexItem>,
    #[serde(default)]
    pub num: i32,
    #[serde(default)]
    pub size: i32,
    #[serde(default)]
    pub total: i64,
}

impl SeasonIndexPage {
    pub fn has_next_page(&self) -> bool {
        self.has_next != 0
    }
}

// ─── Season Detail ──────────────────────────────────────────────────────────

/// Query used to look up a season: by season id or by episode id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeasonQuery {
    SeasonId(i64),
    EpId(i64),
}

impl SeasonQuery {
    /// Query-string key and value, e.g. `("season_id", 123)`.
    pub fn param(&self) -> (&'static str, i64) {
        match self {
            SeasonQuery::SeasonId(id) => ("season_id", *id),
            SeasonQuery::EpId(id) => ("ep_id", *id),
        }
    }
}

/// Deserialize `null` (and missing values) as an empty vector.
fn deserialize_vec_or_default<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

/// Deserialize a string, accepting a number as a fallback representation.
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    match Option::<serde_json::Value>::deserialize(deserializer)? {
        Some(serde_json::Value::String(text)) => Ok(Some(text)),
        Some(serde_json::Value::Number(number)) => Ok(Some(number.to_string())),
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(other) => Err(D::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeasonResult {
    #[serde(default)]
    pub title: String,
    #[serde(rename = "season_id", default)]
    pub season_id: i64,
    #[serde(default)]
    pub cover: String,
    #[serde(rename = "square_cover", default)]
    pub square_cover: String,
    pub evaluate: Option<String>,
    pub link: Option<String>,
    #[serde(default)]
    pub episodes: Option<Vec<BangumiEpisode>>,
    #[serde(default)]
    pub section: Option<Vec<EpisodeSection>>,
    pub rating: Option<SeasonRating>,
    pub stat: Option<SeasonStat>,
    pub badge: Option<String>,
    #[serde(rename = "is_finish")]
    pub is_finish: Option<i32>,
    #[serde(rename = "index_show")]
    pub index_show: Option<String>,
    #[serde(rename = "media_id")]
    pub media_id: Option<i64>,
    #[serde(rename = "type")]
    pub season_type: Option<i32>,
    pub type_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub areas: Vec<SeasonArea>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub styles: Vec<String>,
    pub staff: Option<String>,
    pub actors: Option<String>,
    pub alias: Option<String>,
    pub subtitle: Option<String>,
    pub publish: Option<SeasonPublish>,
    pub new_ep: Option<SeasonNewEp>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub seasons: Vec<RelatedSeason>,
    pub series: Option<SeasonSeries>,
    pub total: Option<i32>,
    pub share_url: Option<String>,
    pub bkg_cover: Option<String>,
    pub up_info: Option<SeasonUpInfo>,
    pub payment: Option<SeasonPayment>,
    pub rights: Option<SeasonRights>,
    pub user_status: Option<SeasonUserStatus>,
    pub status: Option<i32>,
    pub show: Option<SeasonShow>,
    /// Real sections fetched from `/pgc/web/season/section`.
    #[serde(skip)]
    pub section_result: Option<SeasonSectionResult>,
    /// Optional `/pgc/review/user` metadata for the media.
    #[serde(skip)]
    pub review_user: Option<ReviewUser>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonArea {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeasonRating {
    #[serde(default)]
    pub score: f32,
    #[serde(default)]
    pub count: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonStat {
    pub views: Option<i64>,
    pub danmakus: Option<i64>,
    pub favorite: Option<i64>,
    pub favorites: Option<i64>,
    pub coins: Option<i64>,
    pub share: Option<i64>,
    pub likes: Option<i64>,
    pub reply: Option<i64>,
    pub follow_text: Option<String>,
}

impl SeasonStat {
    pub fn display_favorites(&self) -> Option<i64> {
        self.favorites.or(self.favorite)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonPublish {
    pub is_finish: Option<i32>,
    pub is_started: Option<i32>,
    pub pub_time: Option<String>,
    pub pub_time_show: Option<String>,
    pub unknow_pub_date: Option<i32>,
    pub weekday: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonNewEp {
    pub desc: Option<String>,
    pub id: Option<i64>,
    pub is_new: Option<i32>,
    pub title: Option<String>,
    /// `/pgc/review/user` sends a JSON string; some payloads send a number.
    #[serde(default, deserialize_with = "deserialize_string_or_number")]
    pub index: Option<String>,
    pub index_show: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelatedSeason {
    #[serde(rename = "season_id", default)]
    pub season_id: i64,
    pub season_title: Option<String>,
    pub media_id: Option<i64>,
    pub badge: Option<String>,
    pub badge_type: Option<i32>,
    pub cover: Option<String>,
    #[serde(alias = "type")]
    pub season_type: Option<i32>,
    pub new_ep: Option<SeasonNewEp>,
    pub stat: Option<SeasonStat>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonSeries {
    pub display_type: Option<i32>,
    pub series_id: Option<i64>,
    pub series_title: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonUpInfo {
    pub mid: Option<i64>,
    pub uname: Option<String>,
    pub avatar: Option<String>,
    pub follower: Option<i64>,
    pub is_follow: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonPayment {
    pub discount: Option<i32>,
    pub price: Option<String>,
    pub promotion: Option<String>,
    pub tip: Option<String>,
    pub vip_discount: Option<i32>,
    pub vip_price: Option<String>,
    pub vip_promotion: Option<String>,
    pub pay_type: Option<SeasonPayType>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonPayType {
    pub allow_discount: Option<i32>,
    pub allow_pack: Option<i32>,
    pub allow_ticket: Option<i32>,
    pub allow_time_limit: Option<i32>,
    pub allow_vip_discount: Option<i32>,
    pub forbid_bb: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonRights {
    pub allow_bp: Option<i32>,
    pub allow_download: Option<i32>,
    pub allow_review: Option<i32>,
    pub area_limit: Option<i32>,
    pub ban_area_show: Option<i32>,
    pub can_watch: Option<i32>,
    pub copyright: Option<String>,
    pub is_preview: Option<i32>,
    pub only_vip_download: Option<i32>,
    pub watch_platform: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonUserStatus {
    pub area_limit: Option<i32>,
    pub ban_area_show: Option<i32>,
    pub follow: Option<i32>,
    pub follow_status: Option<i32>,
    pub login: Option<i32>,
    pub pay: Option<i32>,
    pub pay_pack_paid: Option<i32>,
    pub sponsor: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonShow {
    pub wide_screen: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EpisodeSection {
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub episodes: Vec<BangumiEpisode>,
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub title: String,
    #[serde(rename = "type", default)]
    pub section_type: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeasonSectionResult {
    pub main_section: Option<EpisodeSection>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub section: Vec<EpisodeSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewUser {
    pub media: Option<ReviewMedia>,
    pub review: Option<ReviewUserAction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewMedia {
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub areas: Vec<SeasonArea>,
    pub cover: Option<String>,
    pub horizontal_picture: Option<String>,
    pub media_id: Option<i64>,
    pub new_ep: Option<SeasonNewEp>,
    pub rating: Option<SeasonRating>,
    pub season_id: Option<i64>,
    pub share_url: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub season_type: Option<i32>,
    pub type_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewUserAction {
    pub is_coin: Option<i32>,
    pub is_open: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BangumiEpisode {
    #[serde(default)]
    pub aid: i64,
    #[serde(default)]
    pub badge: String,
    #[serde(default)]
    pub cid: i64,
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub id: i64,
    #[serde(rename = "is_premiere", default)]
    pub is_premiere: i32,
    #[serde(rename = "long_title", default)]
    pub long_title: String,
    #[serde(rename = "share_url", default)]
    pub share_url: String,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub vid: String,
    pub bvid: Option<String>,
    pub pub_time: Option<i64>,
    pub duration: Option<i64>,
    pub subtitle: Option<String>,
    pub dimension: Option<EpisodeDimension>,
    pub rights: Option<EpisodeRights>,
    pub skip: Option<EpisodeSkip>,
    pub badge_info: Option<BadgeInfo>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub multi_view_eps: Vec<MultiViewEp>,
    pub interaction: Option<EpisodeInteraction>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EpisodeDimension {
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub rotate: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EpisodeRights {
    pub allow_demand: Option<i32>,
    pub allow_dm: Option<i32>,
    pub allow_download: Option<i32>,
    pub area_limit: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EpisodeSkip {
    pub op: Option<SkipRange>,
    pub ed: Option<SkipRange>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SkipRange {
    #[serde(default)]
    pub start: i64,
    #[serde(default)]
    pub end: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BadgeInfo {
    pub bg_color: Option<String>,
    pub bg_color_night: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MultiViewEp {
    #[serde(default)]
    pub ep_id: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EpisodeInteraction {
    pub graph_version: Option<i64>,
    pub interaction: Option<bool>,
}

impl BangumiEpisode {
    pub fn cover_url(&self) -> String {
        if self.cover.starts_with("//") {
            format!("https:{}", self.cover)
        } else {
            self.cover.clone()
        }
    }

    pub fn display_title(&self) -> String {
        if self.long_title.is_empty() {
            format!("第{}集", self.title)
        } else {
            format!("{} {}", self.title, self.long_title)
        }
    }

    pub fn badge_text(&self) -> Option<&str> {
        if self.badge.is_empty() {
            None
        } else {
            Some(&self.badge)
        }
    }

    /// Fill detail fields that `/pgc/web/season/section` omits from a richer
    /// episode object (`/pgc/view/web/season`).
    fn enrich_from(&mut self, source: &BangumiEpisode) {
        if self.aid == 0 && source.aid > 0 {
            self.aid = source.aid;
        }
        if self.cid == 0 && source.cid > 0 {
            self.cid = source.cid;
        }
        if self.bvid.is_none() {
            self.bvid.clone_from(&source.bvid);
        }
        if self.pub_time.is_none() {
            self.pub_time = source.pub_time;
        }
        if self.duration.is_none() {
            self.duration = source.duration;
        }
        if self.subtitle.is_none() {
            self.subtitle.clone_from(&source.subtitle);
        }
        if self.dimension.is_none() {
            self.dimension.clone_from(&source.dimension);
        }
        if self.rights.is_none() {
            self.rights.clone_from(&source.rights);
        }
        if self.skip.is_none() {
            self.skip.clone_from(&source.skip);
        }
        if self.badge_info.is_none() {
            self.badge_info.clone_from(&source.badge_info);
        }
        if self.multi_view_eps.is_empty() {
            self.multi_view_eps.clone_from(&source.multi_view_eps);
        }
        if self.interaction.is_none() {
            self.interaction.clone_from(&source.interaction);
        }
    }
}

impl SeasonResult {
    pub fn cover_url(&self) -> String {
        let url = if self.cover.is_empty() {
            self.square_cover.as_str()
        } else {
            &self.cover
        };
        if url.starts_with("//") {
            format!("https:{}", url)
        } else {
            url.to_string()
        }
    }

    pub fn resolved_type_name(&self) -> Option<&str> {
        self.type_name
            .as_deref()
            .filter(|name| !name.is_empty())
            .or_else(|| {
                self.review_user
                    .as_ref()
                    .and_then(|review| review.media.as_ref())
                    .and_then(|media| media.type_name.as_deref())
                    .filter(|name| !name.is_empty())
            })
            .or_else(|| self.season_type.and_then(season_type_name))
    }

    pub fn is_finished(&self) -> Option<bool> {
        self.publish
            .as_ref()
            .and_then(|publish| publish.is_finish)
            .or(self.is_finish)
            .map(|finished| finished != 0)
    }

    pub fn display_index_show(&self) -> Option<&str> {
        self.index_show
            .as_deref()
            .filter(|text| !text.is_empty())
            .or_else(|| {
                self.new_ep
                    .as_ref()
                    .and_then(|ep| ep.desc.as_deref())
                    .filter(|text| !text.is_empty())
            })
            .or_else(|| {
                self.new_ep
                    .as_ref()
                    .and_then(|ep| ep.index_show.as_deref())
                    .filter(|text| !text.is_empty())
            })
            .or_else(|| {
                self.review_user
                    .as_ref()
                    .and_then(|review| review.media.as_ref())
                    .and_then(|media| media.new_ep.as_ref())
                    .and_then(|ep| ep.index_show.as_deref())
                    .filter(|text| !text.is_empty())
            })
    }

    /// `rights.area_limit` is a region bitmask (e.g. `328`) even for watchable
    /// seasons, so only `user_status.area_limit == 1` or `rights.can_watch == 0`
    /// signal a real restriction.
    pub fn is_area_limited(&self) -> bool {
        self.user_status
            .as_ref()
            .and_then(|status| status.area_limit)
            == Some(1)
            || self.rights.as_ref().and_then(|rights| rights.can_watch) == Some(0)
    }

    pub fn is_followed(&self) -> bool {
        self.user_status.as_ref().is_some_and(|status| {
            status.follow_status.is_some_and(|value| value > 0) || status.follow == Some(1)
        })
    }

    /// Store real section data from `/pgc/web/season/section`, enriching its
    /// lean episodes with detail fields keyed by `ep.id`.
    pub fn apply_section_result(&mut self, mut section_result: SeasonSectionResult) {
        let mut rich_episodes = std::collections::HashMap::new();
        for episode in self.episodes.iter().flatten() {
            rich_episodes.insert(episode.id, episode.clone());
        }
        for section in self.section.iter().flatten() {
            for episode in &section.episodes {
                rich_episodes
                    .entry(episode.id)
                    .or_insert_with(|| episode.clone());
            }
        }

        let enrich = |episodes: &mut Vec<BangumiEpisode>| {
            for episode in episodes.iter_mut() {
                if let Some(source) = rich_episodes.get(&episode.id) {
                    episode.enrich_from(source);
                }
            }
        };
        if let Some(main_section) = &mut section_result.main_section {
            enrich(&mut main_section.episodes);
        }
        for section in &mut section_result.section {
            enrich(&mut section.episodes);
        }

        self.section_result = Some(section_result);
    }

    pub fn all_sections(&self) -> Vec<EpisodeSection> {
        let mut sections = Vec::new();
        let mut seen_episodes = std::collections::HashSet::new();
        let mut seen_sections = std::collections::HashSet::new();

        if let Some(section_result) = &self.section_result {
            for section in section_result
                .main_section
                .iter()
                .chain(section_result.section.iter())
            {
                push_unique_section(
                    section,
                    &mut sections,
                    &mut seen_episodes,
                    &mut seen_sections,
                );
            }
            if !sections.is_empty() {
                return sections;
            }
        }

        // Fallback: modern API puts main episodes directly in `result.episodes`.
        if let Some(episodes) = &self.episodes
            && !episodes.is_empty()
        {
            let main_section = EpisodeSection {
                episodes: episodes.clone(),
                id: 0,
                title: "正片".to_string(),
                section_type: 0,
            };
            push_unique_section(
                &main_section,
                &mut sections,
                &mut seen_episodes,
                &mut seen_sections,
            );
        }
        if let Some(extras) = &self.section {
            for section in extras {
                push_unique_section(
                    section,
                    &mut sections,
                    &mut seen_episodes,
                    &mut seen_sections,
                );
            }
        }
        sections
    }

    pub fn total_episodes(&self) -> usize {
        self.all_sections().iter().map(|s| s.episodes.len()).sum()
    }
}

fn push_unique_section(
    section: &EpisodeSection,
    sections: &mut Vec<EpisodeSection>,
    seen_episodes: &mut std::collections::HashSet<i64>,
    seen_sections: &mut std::collections::HashSet<(i64, String)>,
) {
    if section.id != 0 && !seen_sections.insert((section.id, section.title.clone())) {
        return;
    }
    let episodes: Vec<BangumiEpisode> = section
        .episodes
        .iter()
        .filter(|episode| seen_episodes.insert(episode.id))
        .cloned()
        .collect();
    if episodes.is_empty() {
        return;
    }
    sections.push(EpisodeSection {
        episodes,
        id: section.id,
        title: section.title.clone(),
        section_type: section.section_type,
    });
}

fn season_type_name(season_type: i32) -> Option<&'static str> {
    match season_type {
        1 => Some("番剧"),
        2 => Some("电影"),
        3 => Some("纪录片"),
        4 => Some("国创"),
        5 => Some("电视剧"),
        7 => Some("综艺"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn modern_season_payload_deserializes_all_detail_fields() {
        let season: SeasonResult = serde_json::from_str(
            r##"{
            "title": "示例番剧",
            "season_id": 30000,
            "cover": "//i0.hdslb.com/cover.jpg",
            "square_cover": "//i0.hdslb.com/square.jpg",
            "evaluate": "简介",
            "link": "https://www.bilibili.com/bangumi/play/ss30000",
            "episodes": [{
                "aid": 100,
                "badge": "会员",
                "bvid": "BV1xx411c7mD",
                "cid": 200,
                "cover": "//i0.hdslb.com/ep.jpg",
                "from": "bangumi",
                "id": 500,
                "is_premiere": 1,
                "long_title": "第一话",
                "share_url": "https://www.bilibili.com/bangumi/play/ep500",
                "status": 2,
                "title": "1",
                "vid": "ep500",
                "pub_time": 1700000000,
                "duration": 1430,
                "subtitle": "字幕",
                "dimension": {"width": 1920, "height": 1080, "rotate": 0},
                "rights": {"allow_demand": 0, "allow_dm": 1, "allow_download": 1, "area_limit": 0},
                "skip": {"op": {"start": 10, "end": 100}, "ed": {"start": 1300, "end": 1400}},
                "badge_info": {"bg_color": "#fff", "bg_color_night": "#000", "text": "会员"},
                "multi_view_eps": [{"ep_id": 501}],
                "interaction": {"graph_version": 7, "interaction": true}
            }],
            "section": [{"episodes": [], "id": 9000, "title": "花絮", "type": 1}],
            "rating": {"score": 9.1, "count": 1234},
            "stat": {
                "views": 1000,
                "danmakus": 20,
                "favorite": 10,
                "favorites": 11,
                "coins": 2,
                "share": 3,
                "likes": 4,
                "reply": 5,
                "follow_text": "追番"
            },
            "badge": "独家",
            "is_finish": 1,
            "index_show": "全12话",
            "media_id": 777,
            "type": 1,
            "type_name": "番剧",
            "areas": [{"id": 1, "name": "日本"}],
            "styles": ["奇幻", "冒险"],
            "staff": "staff",
            "actors": "actors",
            "alias": "别名",
            "subtitle": "sub",
            "publish": {
                "is_finish": 1,
                "is_started": 1,
                "pub_time": "2024-01-01",
                "pub_time_show": "1月1日",
                "unknow_pub_date": 0,
                "weekday": 1
            },
            "new_ep": {
                "desc": "更新至第12话",
                "id": 500,
                "is_new": 1,
                "title": "12",
                "index": "14",
                "index_show": "全12话"
            },
            "seasons": [{
                "season_id": 30001,
                "season_title": "第二季",
                "media_id": 778,
                "badge": "独家",
                "badge_type": 1,
                "cover": "//cover",
                "season_type": 1,
                "new_ep": {"index_show": "全12话"},
                "stat": {"views": 1}
            }],
            "series": {"display_type": 1, "series_id": 42, "series_title": "系列"},
            "total": 12,
            "share_url": "https://share",
            "bkg_cover": "//bkg",
            "up_info": {"mid": 1, "uname": "up", "avatar": "//a", "follower": 100, "is_follow": 1},
            "payment": {
                "discount": 0,
                "price": "100",
                "promotion": "",
                "tip": "",
                "vip_discount": 0,
                "vip_price": "0",
                "vip_promotion": "",
                "pay_type": {
                    "allow_discount": 0,
                    "allow_pack": 0,
                    "allow_ticket": 0,
                    "allow_time_limit": 0,
                    "allow_vip_discount": 0,
                    "forbid_bb": 0
                }
            },
            "rights": {
                "allow_bp": 0,
                "allow_download": 1,
                "allow_review": 1,
                "area_limit": 328,
                "ban_area_show": 0,
                "can_watch": 1,
                "copyright": "bilibili",
                "is_preview": 0,
                "only_vip_download": 0,
                "watch_platform": 1
            },
            "user_status": {
                "area_limit": 0,
                "ban_area_show": 0,
                "follow": 1,
                "follow_status": 2,
                "login": 1,
                "pay": 0,
                "pay_pack_paid": 0,
                "sponsor": 0
            },
            "status": 13,
            "show": {"wide_screen": 1}
        }"##,
        )
        .unwrap();
        assert_eq!(season.title, "示例番剧");
        assert_eq!(season.season_id, 30000);
        assert_eq!(season.media_id, Some(777));
        assert_eq!(season.season_type, Some(1));
        assert_eq!(season.areas[0].name, "日本");
        assert_eq!(season.styles, vec!["奇幻", "冒险"]);
        assert_eq!(season.total, Some(12));
        assert_eq!(season.up_info.as_ref().unwrap().follower, Some(100));
        assert_eq!(
            season.payment.as_ref().unwrap().price.as_deref(),
            Some("100")
        );
        assert_eq!(season.rights.as_ref().unwrap().area_limit, Some(328));
        assert_eq!(
            season.rights.as_ref().unwrap().copyright.as_deref(),
            Some("bilibili")
        );
        assert_eq!(
            season
                .payment
                .as_ref()
                .unwrap()
                .pay_type
                .as_ref()
                .unwrap()
                .forbid_bb,
            Some(0)
        );
        assert_eq!(season.new_ep.as_ref().unwrap().index.as_deref(), Some("14"));
        assert_eq!(season.user_status.as_ref().unwrap().follow_status, Some(2));
        assert_eq!(season.show.as_ref().unwrap().wide_screen, Some(1));
        assert_eq!(season.seasons[0].season_title.as_deref(), Some("第二季"));
        assert_eq!(season.seasons[0].season_type, Some(1));
        assert_eq!(season.series.as_ref().unwrap().series_id, Some(42));

        let stat = season.stat.as_ref().unwrap();
        assert_eq!(stat.favorite, Some(10));
        assert_eq!(stat.favorites, Some(11));
        assert_eq!(stat.display_favorites(), Some(11));
        assert_eq!(stat.follow_text.as_deref(), Some("追番"));

        assert_eq!(season.is_finished(), Some(true));
        assert_eq!(season.display_index_show(), Some("全12话"));
        assert_eq!(season.resolved_type_name(), Some("番剧"));
        assert!(!season.is_area_limited());
        assert!(season.is_followed());

        let episode = &season.episodes.as_ref().unwrap()[0];
        assert_eq!(episode.bvid.as_deref(), Some("BV1xx411c7mD"));
        assert_eq!(episode.pub_time, Some(1700000000));
        assert_eq!(episode.duration, Some(1430));
        assert_eq!(episode.subtitle.as_deref(), Some("字幕"));
        assert_eq!(episode.dimension.as_ref().unwrap().width, Some(1920));
        assert_eq!(episode.rights.as_ref().unwrap().allow_dm, Some(1));
        assert_eq!(episode.skip.as_ref().unwrap().op.as_ref().unwrap().end, 100);
        assert_eq!(
            episode.badge_info.as_ref().unwrap().bg_color.as_deref(),
            Some("#fff")
        );
        assert_eq!(episode.multi_view_eps[0].ep_id, 501);
        assert_eq!(episode.interaction.as_ref().unwrap().graph_version, Some(7));
    }

    #[test]
    fn minimal_and_null_heavy_payloads_deserialize() {
        let season: SeasonResult = serde_json::from_value(json!({})).unwrap();
        assert_eq!(season.season_id, 0);
        assert!(season.episodes.is_none());
        assert!(season.section.is_none());
        assert!(season.areas.is_empty());
        assert!(season.seasons.is_empty());
        assert!(season.section_result.is_none());
        assert!(season.review_user.is_none());

        let season: SeasonResult = serde_json::from_value(json!({
            "episodes": null,
            "section": [{"episodes": null, "title": "花絮"}],
            "stat": {"favorite": 3},
            "rating": null,
            "new_ep": {},
            "publish": null,
            "rights": {},
            "user_status": {}
        }))
        .unwrap();
        assert!(season.episodes.is_none());
        assert!(season.section.as_ref().unwrap()[0].episodes.is_empty());
        assert_eq!(season.stat.as_ref().unwrap().display_favorites(), Some(3));
        assert!(season.new_ep.is_some());

        let desc_fallback: SeasonResult = serde_json::from_value(json!({
            "new_ep": {"desc": "更新至第3话"}
        }))
        .unwrap();
        assert_eq!(desc_fallback.display_index_show(), Some("更新至第3话"));

        let episode: BangumiEpisode = serde_json::from_value(json!({
            "id": 1,
            "multi_view_eps": null
        }))
        .unwrap();
        assert_eq!(episode.id, 1);
        assert!(episode.multi_view_eps.is_empty());
        assert!(episode.skip.is_none());

        let null_arrays: SeasonResult = serde_json::from_value(json!({
            "areas": null,
            "styles": null,
            "seasons": null,
            "section": null,
            "episodes": null
        }))
        .unwrap();
        assert!(null_arrays.areas.is_empty());
        assert!(null_arrays.styles.is_empty());
        assert!(null_arrays.seasons.is_empty());
        assert!(null_arrays.section.is_none());

        let null_sections: SeasonSectionResult =
            serde_json::from_value(json!({"main_section": null, "section": null})).unwrap();
        assert!(null_sections.main_section.is_none());
        assert!(null_sections.section.is_empty());

        let null_review_areas: ReviewUser =
            serde_json::from_value(json!({"media": {"areas": null}})).unwrap();
        assert!(null_review_areas.media.unwrap().areas.is_empty());

        let numeric_index: SeasonNewEp = serde_json::from_value(json!({"index": 14})).unwrap();
        assert_eq!(numeric_index.index.as_deref(), Some("14"));
    }

    #[test]
    fn apply_section_result_enriches_and_reports_real_sections() {
        let mut season: SeasonResult = serde_json::from_value(json!({
            "season_id": 30000,
            "episodes": [{
                "id": 1,
                "bvid": "BV1",
                "duration": 1400,
                "cid": 10,
                "pub_time": 1700000000,
                "subtitle": "字幕",
                "dimension": {"width": 1920, "height": 1080, "rotate": 0},
                "skip": {"op": {"start": 5, "end": 95}},
                "multi_view_eps": [{"ep_id": 1}],
                "interaction": {"graph_version": 3, "interaction": true}
            }],
            "section": [
                {"id": 9000, "title": "花絮", "type": 1, "episodes": [{"id": 2, "bvid": "BV2"}]}
            ]
        }))
        .unwrap();

        let section_result: SeasonSectionResult = serde_json::from_value(json!({
            "main_section": {
                "episodes": [{"id": 1, "aid": 100, "cid": 0, "title": "1"}],
                "id": 0,
                "title": "正片",
                "type": 0
            },
            "section": [
                {"episodes": [{"id": 2, "title": "SP"}], "id": 9000, "title": "花絮", "type": 1},
                {"episodes": [], "id": 9001, "title": "空", "type": 1}
            ]
        }))
        .unwrap();

        season.apply_section_result(section_result);
        let sections = season.all_sections();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "正片");
        assert_eq!(sections[0].episodes[0].aid, 100);
        assert_eq!(sections[0].episodes[0].cid, 10);
        assert_eq!(sections[0].episodes[0].bvid.as_deref(), Some("BV1"));
        assert_eq!(sections[0].episodes[0].duration, Some(1400));
        assert_eq!(sections[0].episodes[0].subtitle.as_deref(), Some("字幕"));
        assert_eq!(
            sections[0].episodes[0]
                .skip
                .as_ref()
                .unwrap()
                .op
                .as_ref()
                .unwrap()
                .start,
            5
        );
        assert_eq!(sections[1].title, "花絮");
        assert_eq!(sections[1].episodes[0].bvid.as_deref(), Some("BV2"));
        // No synthesized "正片" duplicate from `self.episodes`.
        assert_eq!(sections.iter().map(|s| s.episodes.len()).sum::<usize>(), 2);
    }

    #[test]
    fn all_sections_falls_back_when_section_result_is_absent_or_empty() {
        let season: SeasonResult = serde_json::from_value(json!({
            "episodes": [{"id": 1}, {"id": 2}],
            "section": [
                {"id": 9000, "title": "花絮", "type": 1, "episodes": [{"id": 2}, {"id": 3}]},
                {"id": 9001, "title": "空", "type": 1, "episodes": []}
            ]
        }))
        .unwrap();
        let sections = season.all_sections();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "正片");
        assert_eq!(
            sections[0]
                .episodes
                .iter()
                .map(|ep| ep.id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        // Episode 2 was already included, empty section dropped, no duplicates.
        assert_eq!(
            sections[1]
                .episodes
                .iter()
                .map(|ep| ep.id)
                .collect::<Vec<_>>(),
            vec![3]
        );

        let mut season = season;
        season.apply_section_result(SeasonSectionResult::default());
        let sections = season.all_sections();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "正片");
        assert_eq!(sections.iter().map(|s| s.episodes.len()).sum::<usize>(), 3);
    }

    #[test]
    fn review_user_parses_for_guest_and_logged_in() {
        let guest: ReviewUser = serde_json::from_value(json!({
            "media": {
                "areas": [{"id": 2, "name": "中国"}],
                "cover": "//cover",
                "horizontal_picture": "//h",
                "media_id": 777,
                "new_ep": {"id": 500, "index": "14", "index_show": "全12话"},
                "rating": {"score": 8.5, "count": 30},
                "season_id": 30000,
                "share_url": "https://share",
                "title": "示例番剧",
                "type": 2,
                "type_name": "电影"
            },
            "review": null
        }))
        .unwrap();
        assert!(guest.review.is_none());
        let media = guest.media.as_ref().unwrap();
        assert_eq!(media.type_name.as_deref(), Some("电影"));
        assert_eq!(
            media.new_ep.as_ref().unwrap().index_show.as_deref(),
            Some("全12话")
        );

        let logged_in: ReviewUser = serde_json::from_value(json!({
            "media": null,
            "review": {"is_coin": 1, "is_open": 1}
        }))
        .unwrap();
        assert!(logged_in.media.is_none());
        assert_eq!(logged_in.review.as_ref().unwrap().is_coin, Some(1));

        let mut season: SeasonResult = serde_json::from_value(json!({"season_type": 2})).unwrap();
        season.review_user = Some(guest);
        assert_eq!(season.resolved_type_name(), Some("电影"));
        assert_eq!(season.display_index_show(), Some("全12话"));

        let mapped: SeasonResult = serde_json::from_value(json!({})).unwrap();
        assert_eq!(mapped.resolved_type_name(), None);
    }

    #[test]
    fn season_query_builds_expected_params() {
        assert_eq!(SeasonQuery::SeasonId(42).param(), ("season_id", 42));
        assert_eq!(SeasonQuery::EpId(7).param(), ("ep_id", 7));
    }

    #[test]
    fn area_limits_use_user_status_and_can_watch_not_rights_bitmask() {
        let watchable: SeasonResult = serde_json::from_value(json!({
            "rights": {"area_limit": 328, "can_watch": 1},
            "user_status": {"area_limit": 0}
        }))
        .unwrap();
        assert!(!watchable.is_area_limited());

        let user_limited: SeasonResult = serde_json::from_value(json!({
            "rights": {"area_limit": 328, "can_watch": 1},
            "user_status": {"area_limit": 1}
        }))
        .unwrap();
        assert!(user_limited.is_area_limited());

        let cannot_watch: SeasonResult = serde_json::from_value(json!({
            "rights": {"area_limit": 0, "can_watch": 0}
        }))
        .unwrap();
        assert!(cannot_watch.is_area_limited());

        let no_rights: SeasonResult = serde_json::from_value(json!({})).unwrap();
        assert!(!no_rights.is_area_limited());
        assert!(!no_rights.is_followed());
    }

    #[test]
    fn season_index_query_encodes_year_and_release_date_filters() {
        let param = |query: &SeasonIndexQuery, key: &'static str| {
            query
                .to_query_params()
                .into_iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v)
        };

        let anime = SeasonIndexQuery::default();
        assert_eq!(param(&anime, "type").as_deref(), Some("1"));
        assert_eq!(param(&anime, "season_type").as_deref(), Some("1"));
        assert_eq!(param(&anime, "st").as_deref(), Some("1"));
        assert_eq!(param(&anime, "page").as_deref(), Some("1"));
        assert_eq!(param(&anime, "pagesize").as_deref(), Some("20"));
        assert_eq!(param(&anime, "order").as_deref(), Some("3"));
        assert_eq!(param(&anime, "sort").as_deref(), Some("0"));
        assert_eq!(param(&anime, "year").as_deref(), Some("-1"));
        assert_eq!(param(&anime, "release_date"), None);

        let anime_range = SeasonIndexQuery {
            year: "[2025,2026)".to_string(),
            ..Default::default()
        };
        assert_eq!(param(&anime_range, "year").as_deref(), Some("[2025,2026)"));
        assert_eq!(param(&anime_range, "release_date"), None);

        let movie = SeasonIndexQuery {
            season_type: BangumiSeasonType::Movie,
            year: "[2025,2026)".to_string(),
            ..Default::default()
        };
        assert_eq!(param(&movie, "year"), None);
        assert_eq!(
            param(&movie, "release_date").as_deref(),
            Some("[2025-01-01 00:00:00,2026-01-01 00:00:00)")
        );

        let open_interval = SeasonIndexQuery {
            season_type: BangumiSeasonType::Tv,
            year: "[,1980)".to_string(),
            ..Default::default()
        };
        assert_eq!(
            param(&open_interval, "release_date").as_deref(),
            Some("[,1980-01-01 00:00:00)")
        );

        let already_formatted = SeasonIndexQuery {
            season_type: BangumiSeasonType::Documentary,
            year: "[2025-01-01 00:00:00,2026-01-01 00:00:00)".to_string(),
            ..Default::default()
        };
        assert_eq!(
            param(&already_formatted, "release_date").as_deref(),
            Some("[2025-01-01 00:00:00,2026-01-01 00:00:00)")
        );

        let movie_default = SeasonIndexQuery {
            season_type: BangumiSeasonType::Movie,
            ..Default::default()
        };
        assert_eq!(param(&movie_default, "release_date").as_deref(), Some("-1"));

        let sorted = SeasonIndexQuery {
            order: SeasonIndexOrder::Score,
            sort: SeasonIndexSort::Asc,
            ..Default::default()
        };
        assert_eq!(param(&sorted, "order").as_deref(), Some("4"));
        assert_eq!(param(&sorted, "sort").as_deref(), Some("1"));
    }

    #[test]
    fn season_index_page_deserializes_real_payload_and_null_lists() {
        let page: SeasonIndexPage = serde_json::from_value(json!({
            "has_next": 1,
            "list": [{
                "season_id": 30000,
                "media_id": 28223000,
                "title": "示例番剧",
                "subTitle": "更新至第3话",
                "cover": "//i0.hdslb.com/cover.jpg",
                "index_show": "全12话",
                "order": "3",
                "order_type": "1",
                "score": "8.8",
                "badge": "独家",
                "badge_info": {"bg_color": "#fff", "bg_color_night": "#000", "text": "会员"},
                "season_status": 13,
                "season_type": 1,
                "is_finish": 0,
                "link": "https://www.bilibili.com/bangumi/play/ss30000",
                "first_ep": {"ep_id": 500, "cover": "//i0.hdslb.com/ep.jpg"}
            }],
            "num": 1,
            "size": 20,
            "total": 100
        }))
        .unwrap();

        assert!(page.has_next_page());
        assert_eq!(page.list.len(), 1);
        let item = &page.list[0];
        assert_eq!(item.sub_title, "更新至第3话");
        assert_eq!(item.display_subtitle(), "更新至第3话");
        assert_eq!(item.cover_url(), "https://i0.hdslb.com/cover.jpg");
        assert_eq!(item.badge_text(), Some("独家"));
        assert_eq!(item.score_text(), "8.8");
        assert_eq!(item.first_ep.as_ref().map(|ep| ep.ep_id), Some(500));

        let sparse: SeasonIndexPage = serde_json::from_value(json!({"list": null})).unwrap();
        assert!(sparse.list.is_empty());
        assert!(!sparse.has_next_page());

        let sparse_item: SeasonIndexItem = serde_json::from_value(json!({})).unwrap();
        assert_eq!(sparse_item.display_subtitle(), "-");
        assert_eq!(sparse_item.score_text(), "-");
        assert_eq!(sparse_item.badge_text(), None);
        assert!(sparse_item.cover_url().is_empty());
    }

    #[test]
    fn timeline_episodes_tolerate_delay_fields_and_missing_values() {
        let day: TimelineDay = serde_json::from_value(json!({
            "date": "2026-11-27",
            "date_ts": 1764201600,
            "day_of_week": 5,
            "is_today": 1,
            "episodes": [{
                "episode_id": 500,
                "season_id": 7,
                "title": "标题",
                "delay": 0,
                "delay_id": 4,
                "delay_index": "1",
                "delay_reason": "延期"
            }]
        }))
        .unwrap();
        let episode = &day.episodes[0];
        assert_eq!(
            (
                episode.delay_id,
                episode.delay_index.as_str(),
                episode.delay_reason.as_str()
            ),
            (4, "1", "延期")
        );

        let sparse: TimelineDay = serde_json::from_value(json!({"episodes": null})).unwrap();
        assert!(sparse.episodes.is_empty());
        assert_eq!(sparse.date, "");

        let sparse_episode: TimelineEpisode = serde_json::from_value(json!({})).unwrap();
        assert_eq!(sparse_episode.delay_id, 0);
        assert!(sparse_episode.delay_index.is_empty());
        assert!(sparse_episode.delay_reason.is_empty());
    }
}
