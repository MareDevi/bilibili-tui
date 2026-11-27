//! Bilibili watch progress reporting (watch start / heartbeat).
//!
//! Protocol research (per `qjwh/bilibili-API-collect`, `docs/video/report.md`):
//! Bilibili web bangumi playback pages reuse the same reporting endpoints as
//! normal UGC videos — there is NO separate PGC heartbeat endpoint:
//!   - Watch start:  `POST https://api.bilibili.com/x/click-interface/click/web/h5`
//!   - Heartbeats:   `POST https://api.bilibili.com/x/click-interface/web/heartbeat`
//!
//! Key differences for PGC (bangumi / movie / documentary / guochuang / tv /
//! variety):
//!   - `type = "4"` (PGC episode; vs `"3"` for a UGC video)
//!   - `sub_type` = `season_type` (`1` 番剧 / `2` 电影 / `3` 纪录片 / `4` 国创 /
//!     `5` 电视剧 / `7` 综艺; defaults to `1` when unknown)
//!   - `epid = ep_id` and `sid = season_id` are included (watch start and heartbeat)
//!   - `aid` / `cid` come from the `BangumiEpisode`, `bvid` is omitted
//!   - `refer_url = https://www.bilibili.com/bangumi/play/ep{ep_id}`
//!   - `csrf` is injected from the `bili_jct` cookie by `ApiClient::post`
//!
//! Both endpoints answer `{"code": 0, "message": "0", "ttl": 1}` with NO `data`
//! field, so the response must be parsed as `ApiResponse<serde_json::Value>` and
//! must not treat a missing `data` as an error (`ApiClient::post` already rejects
//! non-zero codes).

use anyhow::Result;
use serde::Deserialize;

use super::client::{ApiClient, ApiResponse, BilibiliApiDomain};

/// PGC (bangumi) identity needed by the shared heartbeat endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PgcHeartbeatContext {
    pub season_id: i64,
    pub ep_id: i64,
    pub season_type: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct HeartbeatResponse {
    pub code: i32,
    pub message: String,
}

/// Build the watch-start form body for `click/web/h5`.
///
/// UGC (`pgc = None`): `type = 3`, `sub_type = 0`, `bvid` present when given,
/// `refer_url = https://www.bilibili.com/video/{bvid}`.
/// PGC (`pgc = Some`): `type = 4`, `sub_type = season_type` (default `1`),
/// `epid` / `sid` present, `bvid` omitted,
/// `refer_url = https://www.bilibili.com/bangumi/play/ep{ep_id}`.
pub fn build_watch_start_form(
    aid: i64,
    cid: i64,
    bvid: Option<&str>,
    pgc: Option<PgcHeartbeatContext>,
    _duration: i64,
) -> Vec<(&'static str, String)> {
    let mut form = base_form(aid, cid, bvid, pgc);
    form.push(("mid", "0".to_string()));
    form.push(("dt", "2".to_string()));
    form.push(("auto_continued_play", "0".to_string()));
    form.push(("bsource", String::new()));
    form.push(("stime", chrono::Utc::now().timestamp().to_string()));
    form
}

/// Build the heartbeat form body for `click-interface/web/heartbeat`.
///
/// Same PGC/UGC divergence as [`build_watch_start_form`], plus `play_type`
/// (`0` while playing, `4` at playback end) and `video_duration` when known.
#[allow(clippy::too_many_arguments)]
pub fn build_heartbeat_form(
    aid: i64,
    cid: i64,
    bvid: Option<&str>,
    pgc: Option<PgcHeartbeatContext>,
    played_time: i64,
    real_played_time: i64,
    realtime: i64,
    start_ts: i64,
    play_type: i32,
    duration: i64,
) -> Vec<(&'static str, String)> {
    let mut form = base_form(aid, cid, bvid, pgc);
    form.push(("mid", "0".to_string()));
    form.push(("played_time", played_time.to_string()));
    form.push(("realtime", realtime.to_string()));
    form.push(("real_played_time", real_played_time.to_string()));
    form.push(("start_ts", start_ts.to_string()));
    form.push(("play_type", play_type.to_string()));
    form.push(("dt", "2".to_string()));
    form.push(("auto_continued_play", "0".to_string()));
    form.push(("bsource", String::new()));
    if duration > 0 {
        form.push(("video_duration", duration.to_string()));
    }
    form
}

/// Fields shared by both endpoints: video identity, kind, and referer.
fn base_form(
    aid: i64,
    cid: i64,
    bvid: Option<&str>,
    pgc: Option<PgcHeartbeatContext>,
) -> Vec<(&'static str, String)> {
    let mut form = vec![("aid", aid.to_string()), ("cid", cid.to_string())];
    if let Some(ctx) = pgc {
        form.push(("type", "4".to_string()));
        form.push(("sub_type", ctx.season_type.unwrap_or(1).to_string()));
        form.push(("epid", ctx.ep_id.to_string()));
        form.push(("sid", ctx.season_id.to_string()));
        form.push((
            "refer_url",
            format!("https://www.bilibili.com/bangumi/play/ep{}", ctx.ep_id),
        ));
    } else {
        form.push(("type", "3".to_string()));
        form.push(("sub_type", "0".to_string()));
        let bvid = bvid.unwrap_or("");
        if !bvid.is_empty() {
            form.push(("bvid", bvid.to_string()));
        }
        form.push((
            "refer_url",
            format!("https://www.bilibili.com/video/{}", bvid),
        ));
    }
    form
}

/// Both reporting endpoints return `{"code":0,"message":"0","ttl":1}` with no
/// `data`; `ApiClient::post` has already validated `code == 0`.
fn heartbeat_response(resp: ApiResponse<serde_json::Value>) -> HeartbeatResponse {
    HeartbeatResponse {
        code: resp.code,
        message: resp.message,
    }
}

/// Report video / bangumi watch start to Bilibili
pub async fn report_watch_start(
    client: &ApiClient,
    aid: i64,
    cid: i64,
    bvid: Option<&str>,
    pgc: Option<PgcHeartbeatContext>,
    duration: i64,
) -> Result<HeartbeatResponse> {
    let url = format!(
        "{}/x/click-interface/click/web/h5",
        BilibiliApiDomain::Main.as_str()
    );

    let form_data = build_watch_start_form(aid, cid, bvid, pgc, duration);
    let resp: ApiResponse<serde_json::Value> = client.post(&url, form_data).await?;
    Ok(heartbeat_response(resp))
}

/// Report video / bangumi playback heartbeat
#[allow(clippy::too_many_arguments)]
pub async fn report_heartbeat(
    client: &ApiClient,
    aid: i64,
    cid: i64,
    bvid: Option<&str>,
    pgc: Option<PgcHeartbeatContext>,
    played_time: i64,
    real_played_time: i64,
    realtime: i64,
    start_ts: i64,
    play_type: i32,
    duration: i64,
) -> Result<HeartbeatResponse> {
    let url = format!(
        "{}/x/click-interface/web/heartbeat",
        BilibiliApiDomain::Main.as_str()
    );

    let form_data = build_heartbeat_form(
        aid,
        cid,
        bvid,
        pgc,
        played_time,
        real_played_time,
        realtime,
        start_ts,
        play_type,
        duration,
    );
    let resp: ApiResponse<serde_json::Value> = client.post(&url, form_data).await?;
    Ok(heartbeat_response(resp))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form_value<'a>(form: &'a [(&'static str, String)], key: &str) -> Option<&'a str> {
        form.iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.as_str())
    }

    fn pgc(season_type: Option<i32>) -> PgcHeartbeatContext {
        PgcHeartbeatContext {
            season_id: 456,
            ep_id: 123,
            season_type,
        }
    }

    #[test]
    fn pgc_watch_start_form_uses_pgc_fields() {
        let form = build_watch_start_form(1001, 2002, None, Some(pgc(Some(4))), 1440);

        assert_eq!(form_value(&form, "aid"), Some("1001"));
        assert_eq!(form_value(&form, "cid"), Some("2002"));
        assert_eq!(form_value(&form, "type"), Some("4"));
        assert_eq!(form_value(&form, "sub_type"), Some("4"));
        assert_eq!(form_value(&form, "epid"), Some("123"));
        assert_eq!(form_value(&form, "sid"), Some("456"));
        assert_eq!(
            form_value(&form, "refer_url"),
            Some("https://www.bilibili.com/bangumi/play/ep123")
        );
        assert_eq!(form_value(&form, "bvid"), None);
    }

    #[test]
    fn pgc_heartbeat_form_defaults_sub_type_and_carries_play_type() {
        let playing = build_heartbeat_form(
            1001,
            2002,
            None,
            Some(pgc(None)),
            30,
            30,
            31,
            1_700_000_000,
            0,
            1440,
        );
        assert_eq!(form_value(&playing, "type"), Some("4"));
        assert_eq!(form_value(&playing, "sub_type"), Some("1"));
        assert_eq!(form_value(&playing, "play_type"), Some("0"));
        assert_eq!(form_value(&playing, "video_duration"), Some("1440"));
        assert_eq!(form_value(&playing, "played_time"), Some("30"));
        assert_eq!(form_value(&playing, "real_played_time"), Some("30"));
        assert_eq!(form_value(&playing, "realtime"), Some("31"));
        assert_eq!(form_value(&playing, "start_ts"), Some("1700000000"));
        assert_eq!(
            form_value(&playing, "refer_url"),
            Some("https://www.bilibili.com/bangumi/play/ep123")
        );
        assert_eq!(form_value(&playing, "bvid"), None);
        assert_eq!(form_value(&playing, "epid"), Some("123"));
        assert_eq!(form_value(&playing, "sid"), Some("456"));

        let ended = build_heartbeat_form(1001, 2002, None, Some(pgc(Some(2))), 60, 60, 61, 1, 4, 0);
        assert_eq!(form_value(&ended, "play_type"), Some("4"));
        assert_eq!(form_value(&ended, "sub_type"), Some("2"));
        assert_eq!(form_value(&ended, "video_duration"), None);
    }

    #[test]
    fn ugc_watch_start_form_uses_bvid_and_video_referer() {
        let form = build_watch_start_form(1001, 2002, Some("BV1xx411c7mD"), None, 1440);

        assert_eq!(form_value(&form, "type"), Some("3"));
        assert_eq!(form_value(&form, "sub_type"), Some("0"));
        assert_eq!(form_value(&form, "bvid"), Some("BV1xx411c7mD"));
        assert_eq!(
            form_value(&form, "refer_url"),
            Some("https://www.bilibili.com/video/BV1xx411c7mD")
        );
        assert_eq!(form_value(&form, "epid"), None);
        assert_eq!(form_value(&form, "sid"), None);
    }

    #[test]
    fn ugc_heartbeat_form_uses_bvid_and_video_referer() {
        let form = build_heartbeat_form(
            1001,
            2002,
            Some("BV1xx411c7mD"),
            None,
            10,
            10,
            12,
            1_700_000_000,
            4,
            300,
        );

        assert_eq!(form_value(&form, "type"), Some("3"));
        assert_eq!(form_value(&form, "sub_type"), Some("0"));
        assert_eq!(form_value(&form, "bvid"), Some("BV1xx411c7mD"));
        assert_eq!(form_value(&form, "play_type"), Some("4"));
        assert_eq!(form_value(&form, "video_duration"), Some("300"));
        assert_eq!(
            form_value(&form, "refer_url"),
            Some("https://www.bilibili.com/video/BV1xx411c7mD")
        );
        assert_eq!(form_value(&form, "epid"), None);
        assert_eq!(form_value(&form, "sid"), None);
    }

    #[test]
    fn ugc_empty_bvid_is_omitted() {
        let form = build_watch_start_form(1, 2, Some(""), None, 0);
        assert_eq!(form_value(&form, "bvid"), None);
    }

    #[test]
    fn response_without_data_field_is_success() {
        // Real Bilibili reply: no `data` field at all.
        let parsed: ApiResponse<serde_json::Value> =
            serde_json::from_str(r#"{"code":0,"message":"0","ttl":1}"#).unwrap();
        assert_eq!(parsed.code, 0);
        assert!(parsed.data.is_none());

        let response = heartbeat_response(parsed);
        assert_eq!(response.code, 0);
        assert_eq!(response.message, "0");
    }
}
