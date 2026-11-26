//! Dynamic feed API types and functions

use serde::{Deserialize, Deserializer};

fn deserialize_optional_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(number)) => number
            .as_i64()
            .ok_or_else(|| serde::de::Error::custom("integer is out of range"))
            .map(Some),
        Some(serde_json::Value::String(value)) => value
            .parse::<i64>()
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected integer or numeric string, got {other}"
        ))),
    }
}

fn deserialize_optional_i32<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_i64(deserializer)?
        .map(|value| i32::try_from(value).map_err(serde::de::Error::custom))
        .transpose()
}

fn deserialize_optional_f32<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(number)) => number
            .as_f64()
            .map(|value| Some(value as f32))
            .ok_or_else(|| serde::de::Error::custom("invalid floating-point number")),
        Some(serde_json::Value::String(value)) => value
            .parse::<f32>()
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected number or numeric string, got {other}"
        ))),
    }
}

/// Dynamic feed response
#[derive(Debug, Deserialize)]
pub struct DynamicFeedData {
    pub items: Option<Vec<DynamicItem>>,
    pub offset: Option<String>,
    pub has_more: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_i32")]
    pub update_num: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::{DynamicFeedData, DynamicItem};
    use serde::Deserialize;

    fn dynamic_item(json: &str) -> DynamicItem {
        serde_json::from_str(json).expect("dynamic item")
    }

    fn video_item(archive_aid: &str, basic: &str) -> DynamicItem {
        dynamic_item(&format!(
            r#"{{
              "id_str": "9001",
              "type": "DYNAMIC_TYPE_AV",
              "basic": {basic},
              "modules": {{
                "module_dynamic": {{"major": {{
                  "type": "MAJOR_TYPE_ARCHIVE",
                  "archive": {{"bvid": "BV1test", "aid": {archive_aid}}}
                }}}}
              }}
            }}"#
        ))
    }

    #[test]
    fn archive_aid_accepts_string_number_and_null() {
        for (raw, expected) in [
            ("\"112981396619958\"", Some(112_981_396_619_958)),
            ("112981396619958", Some(112_981_396_619_958)),
            ("null", None),
        ] {
            let item = video_item(raw, "null");
            let aid = item
                .modules
                .as_ref()
                .and_then(|m| m.module_dynamic.as_ref())
                .and_then(|d| d.major.as_ref())
                .and_then(|m| m.archive.as_ref())
                .and_then(|a| a.aid);
            assert_eq!(aid, expected, "archive aid {raw}");
        }
    }

    #[test]
    fn video_aid_prefers_archive_then_basic_comment_id() {
        let item = video_item(
            "\"112981396619958\"",
            r#"{"comment_id_str": "999", "comment_type": 1}"#,
        );
        assert_eq!(item.video_aid(), Some(112_981_396_619_958));

        let fallback = video_item("null", r#"{"comment_id_str": "999", "comment_type": 1}"#);
        assert_eq!(fallback.video_aid(), Some(999));

        let none = video_item("0", r#"{"comment_id_str": "", "comment_type": 0}"#);
        assert_eq!(none.video_aid(), None);
    }

    #[test]
    fn video_dynamic_comment_target_uses_basic() {
        let item = video_item(
            "null",
            r#"{"comment_id_str": "112981396619958", "comment_type": 1}"#,
        );
        assert_eq!(item.comment_type(), 1);
        assert_eq!(item.comment_oid("9001"), Some(112_981_396_619_958));
    }

    #[test]
    fn opus_album_comment_target_uses_basic_album_id() {
        let item = dynamic_item(
            r#"{
              "id_str": "9002",
              "type": "DYNAMIC_TYPE_DRAW",
              "basic": {"comment_id_str": "555", "comment_type": 11},
              "modules": {"module_dynamic": {"major": {"type": "MAJOR_TYPE_OPUS"}}}
            }"#,
        );
        assert_eq!(item.comment_type(), 11);
        assert_eq!(item.comment_oid("9002"), Some(555));
    }

    #[test]
    fn pure_text_opus_comment_target_uses_basic_dynamic_id() {
        let item = dynamic_item(
            r#"{
              "id_str": "9003",
              "type": "DYNAMIC_TYPE_WORD",
              "basic": {"comment_id_str": "9003", "comment_type": 17},
              "modules": {"module_dynamic": {"major": {"type": "MAJOR_TYPE_OPUS"}}}
            }"#,
        );
        assert_eq!(item.comment_type(), 17);
        assert_eq!(item.comment_oid("9003"), Some(9003));
    }

    #[test]
    fn empty_basic_falls_back_to_major_type_heuristics() {
        let item = dynamic_item(
            r#"{
              "id_str": "9004",
              "type": "DYNAMIC_TYPE_DRAW",
              "basic": {"comment_id_str": "", "comment_type": 0},
              "modules": {"module_dynamic": {"major": {
                "type": "MAJOR_TYPE_DRAW",
                "draw": {"id": "42"}
              }}}
            }"#,
        );
        assert_eq!(item.comment_type(), 11);
        assert_eq!(item.comment_oid("9004"), Some(42));

        let video = video_item("null", r#"{"comment_id_str": "", "comment_type": 0}"#);
        assert_eq!(video.comment_type(), 1);
    }

    #[test]
    fn feed_accepts_string_update_num() {
        let data: DynamicFeedData =
            serde_json::from_str(r#"{"items":[],"offset":"1","has_more":true,"update_num":"0"}"#)
                .expect("string update_num should deserialize");

        assert_eq!(data.update_num, Some(0));
    }

    #[test]
    fn feed_accepts_numeric_fields_as_numbers_strings_and_null() {
        let json = r#"{
          "items": [{
            "id_str": "1",
            "type": "DYNAMIC_TYPE_DRAW",
            "modules": {
              "module_author": {"pub_ts": "1720000000"},
              "module_dynamic": {"major": {
                "type": "MAJOR_TYPE_DRAW",
                "draw": {"id": "42", "items": [
                  {"src": "a", "height": "1080", "width": 1920, "size": "1.25"},
                  {"src": "b", "height": null, "width": null, "size": 2.5}
                ]}
              }}
            }
          }],
          "offset": null,
          "has_more": false,
          "update_num": 0
        }"#;
        let data: DynamicFeedData = serde_json::from_str(json).expect("mixed numeric forms");
        let item = &data.items.expect("items")[0];
        let modules = item.modules.as_ref().expect("modules");
        assert_eq!(
            modules.module_author.as_ref().and_then(|a| a.pub_ts),
            Some(1_720_000_000)
        );
        let draw = modules
            .module_dynamic
            .as_ref()
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.draw.as_ref())
            .expect("draw");
        assert_eq!(draw.id, Some(42));
        let pictures = draw.items.as_ref().expect("pictures");
        assert_eq!(
            (pictures[0].height, pictures[0].width, pictures[0].size),
            (Some(1080), Some(1920), Some(1.25))
        );
        assert_eq!(
            (pictures[1].height, pictures[1].width, pictures[1].size),
            (None, None, Some(2.5))
        );
    }

    #[test]
    fn feed_rejects_invalid_or_out_of_range_numeric_strings() {
        let invalid = r#"{"items":[{"modules":{"module_author":{"pub_ts":"not-a-number"}}}]}"#;
        assert!(serde_json::from_str::<DynamicFeedData>(invalid).is_err());
        let overflow = r#"{"items":[{"modules":{"module_dynamic":{"major":{"draw":{"items":[{"height":"2147483648"}]}}}}}]}"#;
        assert!(serde_json::from_str::<DynamicFeedData>(overflow).is_err());
    }

    #[test]
    fn saved_real_feed_response_deserializes_when_fixture_is_available() {
        let Ok(path) = std::env::var("BILIBILI_DYNAMIC_FIXTURE") else {
            return;
        };
        #[derive(Deserialize)]
        struct Response {
            data: DynamicFeedData,
        }

        let body = std::fs::read(path).expect("read dynamic fixture");
        let response: Response =
            serde_json::from_slice(&body).expect("deserialize dynamic fixture");
        assert!(response.data.items.is_some());
    }
}

/// Portal data response (frequently watched UPs)
#[derive(Debug, Deserialize)]
pub struct PortalData {
    pub up_list: Option<Vec<UpListItem>>,
    // my_info and live_users are available but not needed for now
}

/// UP master in portal up_list
#[derive(Debug, Clone, Deserialize)]
pub struct UpListItem {
    pub mid: i64,
    pub uname: String,
    pub face: String,
    pub has_update: bool,
    #[serde(default)]
    pub is_reserve_recall: bool,
}

/// Basic dynamic metadata, including the authoritative comment target.
#[derive(Debug, Clone, Deserialize)]
pub struct DynamicBasic {
    pub comment_id_str: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i32")]
    pub comment_type: Option<i32>,
    pub rid_str: Option<String>,
}

/// Individual dynamic item
#[derive(Debug, Clone, Deserialize)]
pub struct DynamicItem {
    pub id_str: Option<String>,
    #[serde(rename = "type")]
    pub dynamic_type: Option<String>,
    #[serde(default)]
    pub basic: Option<DynamicBasic>,
    pub modules: Option<DynamicModules>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DynamicModules {
    pub module_author: Option<ModuleAuthor>,
    pub module_dynamic: Option<ModuleDynamic>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModuleAuthor {
    pub name: Option<String>,
    pub face: Option<String>,
    pub mid: Option<i64>,
    pub pub_time: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub pub_ts: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModuleDynamic {
    pub major: Option<DynamicMajor>,
    pub desc: Option<DynamicDesc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DynamicMajor {
    #[serde(rename = "type")]
    pub major_type: Option<String>,
    pub archive: Option<ArchiveInfo>,
    pub draw: Option<DrawInfo>,
    pub opus: Option<OpusInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveInfo {
    pub bvid: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub aid: Option<i64>,
    pub title: Option<String>,
    pub cover: Option<String>,
    pub desc: Option<String>,
    pub duration_text: Option<String>,
    pub stat: Option<ArchiveStat>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveStat {
    pub play: Option<String>,
    pub danmaku: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DynamicDesc {
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DrawInfo {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub id: Option<i64>,
    pub items: Option<Vec<DrawItem>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DrawItem {
    pub src: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i32")]
    pub height: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_optional_i32")]
    pub width: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_optional_f32")]
    pub size: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpusInfo {
    pub title: Option<String>,
    pub summary: Option<OpusSummary>,
    pub pics: Option<Vec<OpusPic>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpusSummary {
    pub text: Option<String>,
    pub rich_text_nodes: Option<Vec<RichTextNode>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpusPic {
    pub url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i32")]
    pub height: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_optional_i32")]
    pub width: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_optional_f32")]
    pub size: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RichTextNode {
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub node_type: Option<String>,
}

impl DynamicItem {
    pub fn is_video(&self) -> bool {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.major_type.as_ref())
            .map(|t| t == "MAJOR_TYPE_ARCHIVE")
            .unwrap_or(false)
    }

    pub fn author_name(&self) -> &str {
        self.modules
            .as_ref()
            .and_then(|m| m.module_author.as_ref())
            .and_then(|a| a.name.as_deref())
            .unwrap_or("未知")
    }

    pub fn author_face(&self) -> Option<&str> {
        self.modules
            .as_ref()
            .and_then(|m| m.module_author.as_ref())
            .and_then(|a| a.face.as_deref())
    }

    pub fn author_mid(&self) -> Option<i64> {
        self.modules
            .as_ref()
            .and_then(|modules| modules.module_author.as_ref())
            .and_then(|author| author.mid)
    }

    pub fn pub_time(&self) -> &str {
        self.modules
            .as_ref()
            .and_then(|m| m.module_author.as_ref())
            .and_then(|a| a.pub_time.as_deref())
            .unwrap_or("")
    }

    pub fn video_title(&self) -> Option<&str> {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.archive.as_ref())
            .and_then(|a| a.title.as_deref())
    }

    pub fn video_cover(&self) -> Option<&str> {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.archive.as_ref())
            .and_then(|a| a.cover.as_deref())
    }

    pub fn video_bvid(&self) -> Option<&str> {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.archive.as_ref())
            .and_then(|a| a.bvid.as_deref())
    }

    /// Authoritative video aid for opening comments, preferring the archive payload.
    pub fn video_aid(&self) -> Option<i64> {
        let archive_aid = self
            .modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.archive.as_ref())
            .and_then(|a| a.aid);
        if let Some(aid) = archive_aid.filter(|aid| *aid > 0) {
            return Some(aid);
        }
        if !self.is_video() {
            return None;
        }
        self.basic
            .as_ref()
            .and_then(|basic| basic.comment_id_str.as_deref())
            .and_then(|id| id.parse::<i64>().ok())
            .filter(|aid| *aid > 0)
    }

    pub fn video_play(&self) -> &str {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.archive.as_ref())
            .and_then(|a| a.stat.as_ref())
            .and_then(|s| s.play.as_deref())
            .unwrap_or("-")
    }

    pub fn video_danmaku(&self) -> &str {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.archive.as_ref())
            .and_then(|a| a.stat.as_ref())
            .and_then(|s| s.danmaku.as_deref())
            .unwrap_or("-")
    }

    pub fn video_duration(&self) -> &str {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.archive.as_ref())
            .and_then(|a| a.duration_text.as_deref())
            .unwrap_or("")
    }

    pub fn is_draw(&self) -> bool {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.major_type.as_ref())
            .map(|t| t == "MAJOR_TYPE_DRAW")
            .unwrap_or(false)
    }

    pub fn is_opus(&self) -> bool {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.major_type.as_ref())
            .map(|t| t == "MAJOR_TYPE_OPUS")
            .unwrap_or(false)
    }

    pub fn draw_images(&self) -> Vec<&str> {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.draw.as_ref())
            .and_then(|draw| draw.items.as_ref())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.src.as_deref())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn opus_text(&self) -> Option<&str> {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.opus.as_ref())
            .and_then(|opus| opus.summary.as_ref())
            .and_then(|summary| summary.text.as_deref())
    }

    pub fn opus_images(&self) -> Vec<&str> {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.opus.as_ref())
            .and_then(|opus| opus.pics.as_ref())
            .map(|pics| pics.iter().filter_map(|pic| pic.url.as_deref()).collect())
            .unwrap_or_default()
    }

    pub fn desc_text(&self) -> Option<&str> {
        self.modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.desc.as_ref())
            .and_then(|desc| desc.text.as_deref())
    }

    /// Get the correct comment type code for this dynamic
    /// Prefers `basic.comment_type` from the API; falls back to guessing from the major type.
    /// Type 11: 相簿（图片动态） - image/photo albums (MAJOR_TYPE_DRAW, MAJOR_TYPE_OPUS)
    /// Type 17: 动态（纯文字动态&分享） - text dynamics and shares
    /// Type 1: 视频 - videos (MAJOR_TYPE_ARCHIVE)
    pub fn comment_type(&self) -> i32 {
        if let Some(comment_type) = self
            .basic
            .as_ref()
            .and_then(|basic| basic.comment_type)
            .filter(|comment_type| *comment_type > 0)
        {
            return comment_type;
        }
        if let Some(major_type) = self
            .modules
            .as_ref()
            .and_then(|m| m.module_dynamic.as_ref())
            .and_then(|d| d.major.as_ref())
            .and_then(|m| m.major_type.as_ref())
        {
            match major_type.as_str() {
                "MAJOR_TYPE_DRAW" | "MAJOR_TYPE_OPUS" => 11, // 相簿（图片动态）
                "MAJOR_TYPE_ARCHIVE" => 1,                   // 视频
                _ => 17,                                     // 其他类型的动态（纯文字、转发等）
            }
        } else {
            17 // 默认使用类型17
        }
    }

    /// Get the correct oid for comments
    /// Prefers `basic.comment_id_str` from the API.
    /// For image dynamics (MAJOR_TYPE_DRAW), falls back to the draw.id.
    /// For videos, falls back to the archive aid.
    /// For other types, falls back to the dynamic id itself.
    pub fn comment_oid(&self, dynamic_id: &str) -> Option<i64> {
        if let Some(oid) = self
            .basic
            .as_ref()
            .and_then(|basic| basic.comment_id_str.as_deref())
            .and_then(|id| id.parse::<i64>().ok())
            .filter(|oid| *oid > 0)
        {
            return Some(oid);
        }
        if self.is_draw() {
            // For draw type, use the draw.id as oid
            self.modules
                .as_ref()
                .and_then(|m| m.module_dynamic.as_ref())
                .and_then(|d| d.major.as_ref())
                .and_then(|m| m.draw.as_ref())
                .and_then(|draw| draw.id)
        } else if self.is_video() {
            self.video_aid()
        } else {
            // For other types, use the dynamic_id
            dynamic_id.parse::<i64>().ok()
        }
    }
}

/// Following users response
#[derive(Debug, Deserialize)]
pub struct FollowingsData {
    pub list: Option<Vec<FollowingUser>>,
    pub total: Option<i32>,
}

/// Following user info
#[derive(Debug, Clone, Deserialize)]
pub struct FollowingUser {
    pub mid: Option<i64>,
    pub uname: Option<String>,
    pub face: Option<String>,
    pub sign: Option<String>,
}
