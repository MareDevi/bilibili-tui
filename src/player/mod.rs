use crate::api::client::ApiClient;
use crate::storage::Credentials;
use anyhow::Result;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::{interval, Instant};

/// Play a video using mpv with yt-dlp and report watch progress
pub async fn play_video(
    api_client: Arc<ApiClient>,
    bvid: &str,
    aid: i64,
    cid: i64,
    duration: i64,
    page_num: Option<i32>,
    credentials: Option<&Credentials>,
) -> Result<()> {
    let video_url = match page_num {
        Some(p) if p > 1 => format!("https://www.bilibili.com/video/{}?p={}", bvid, p),
        _ => format!("https://www.bilibili.com/video/{}", bvid),
    };

    // Report watch start
    let _ = crate::api::heartbeat::report_watch_start(&api_client, aid, cid, bvid, duration).await;

    let start_ts = chrono::Utc::now().timestamp();
    let mut played_time: i64 = 0;
    let mut real_played_time: i64;

    let mut cmd = Command::new("mpv");

    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let mut cookie_path_to_clean = None;

    if let Some(creds) = credentials {
        let cookie_path = crate::storage::export_cookies_for_ytdlp(creds)?;
        cmd.arg(format!(
            "--ytdl-raw-options=cookies={}",
            cookie_path.display()
        ));
        cookie_path_to_clean = Some(cookie_path);
    }

    cmd.arg("--force-window=immediate");
    cmd.arg(&video_url);

    let mut child = cmd.spawn()?;
    let start_time = Instant::now();

    let mut heartbeat_interval = interval(Duration::from_secs(15));

    loop {
        tokio::select! {
            _ = heartbeat_interval.tick() => {
                played_time += 15;
                real_played_time = start_time.elapsed().as_secs() as i64;

                let _ = crate::api::heartbeat::report_heartbeat(
                    &api_client,
                    aid,
                    cid,
                    bvid,
                    played_time,
                    real_played_time,
                    real_played_time,
                    start_ts,
                    0, // play_type: 0 = playing
                ).await;
            }
            result = child.wait() => {
                real_played_time = start_time.elapsed().as_secs() as i64;

                let _ = crate::api::heartbeat::report_heartbeat(
                    &api_client,
                    aid,
                    cid,
                    bvid,
                    played_time,
                    real_played_time,
                    real_played_time,
                    start_ts,
                    4, // play_type: 4 = end
                ).await;

                result?;
                break;
            }
        }
    }

    if let Some(path) = cookie_path_to_clean {
        let _ = tokio::fs::remove_file(path).await;
    }

    Ok(())
}
