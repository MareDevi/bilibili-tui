use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HeartbeatResponse {
    pub code: i32,
    pub message: String,
}

/// Report video watch start to Bilibili
pub async fn report_watch_start(
    client: &super::client::ApiClient,
    aid: i64,
    cid: i64,
    bvid: &str,
    _duration: i64,
) -> Result<HeartbeatResponse> {
    let url = format!(
        "{}/x/click-interface/click/web/h5",
        super::client::BilibiliApiDomain::Main.as_str()
    );

    let form_data = vec![
        ("aid", aid.to_string()),
        ("cid", cid.to_string()),
        ("bvid", bvid.to_string()),
        ("mid", "0".to_string()),
        ("type", "3".to_string()),
        ("dt", "2".to_string()),
        ("auto_continued_play", "0".to_string()),
        (
            "refer_url",
            format!("https://www.bilibili.com/video/{}", bvid),
        ),
        ("bsource", "".to_string()),
        ("stime", chrono::Utc::now().timestamp().to_string()),
    ];

    let resp: super::client::ApiResponse<HeartbeatResponse> = client.post(&url, form_data).await?;
    resp.data
        .ok_or_else(|| anyhow::anyhow!("No data in watch start response"))
}

/// Report video playback heartbeat
#[allow(clippy::too_many_arguments)]
pub async fn report_heartbeat(
    client: &super::client::ApiClient,
    aid: i64,
    cid: i64,
    bvid: &str,
    played_time: i64,
    real_played_time: i64,
    realtime: i64,
    start_ts: i64,
    play_type: i32,
) -> Result<HeartbeatResponse> {
    let url = format!(
        "{}/x/click-interface/web/heartbeat",
        super::client::BilibiliApiDomain::Main.as_str()
    );

    let form_data = vec![
        ("aid", aid.to_string()),
        ("cid", cid.to_string()),
        ("bvid", bvid.to_string()),
        ("mid", "0".to_string()),
        ("played_time", played_time.to_string()),
        ("realtime", realtime.to_string()),
        ("real_played_time", real_played_time.to_string()),
        ("start_ts", start_ts.to_string()),
        ("type", "3".to_string()),
        ("dt", "2".to_string()),
        ("play_type", play_type.to_string()),
        ("auto_continued_play", "0".to_string()),
        (
            "refer_url",
            format!("https://www.bilibili.com/video/{}", bvid),
        ),
        ("bsource", "".to_string()),
    ];

    let resp: super::client::ApiResponse<HeartbeatResponse> = client.post(&url, form_data).await?;
    resp.data
        .ok_or_else(|| anyhow::anyhow!("No data in heartbeat response"))
}
