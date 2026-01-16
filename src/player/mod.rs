use crate::api::client::ApiClient;
use crate::storage::Credentials;
use anyhow::Result;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::{interval, Instant};

/// Play a video using mpv with yt-dlp and report watch progress
/// This function spawns mpv in a background task to avoid blocking the TUI
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

    let mut cmd = Command::new("mpv");

    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let cookie_path_to_clean = if let Some(creds) = credentials {
        let cookie_path = crate::storage::export_cookies_for_ytdlp(creds)?;
        cmd.arg(format!(
            "--ytdl-raw-options=cookies={}",
            cookie_path.display()
        ));
        Some(cookie_path)
    } else {
        None
    };

    cmd.arg("--force-window=immediate");
    cmd.arg(&video_url);

    let mut child = cmd.spawn()?;

    // Clone bvid for the background task (needs 'static lifetime)
    let bvid = bvid.to_string();

    // Spawn a background task to handle heartbeat and cleanup
    // This prevents blocking the TUI
    tokio::spawn(async move {
        let start_time = Instant::now();
        let mut played_time: i64 = 0;
        let mut heartbeat_interval = interval(Duration::from_secs(15));

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    played_time += 15;
                    let real_played_time = start_time.elapsed().as_secs() as i64;

                    let _ = crate::api::heartbeat::report_heartbeat(
                        &api_client,
                        aid,
                        cid,
                        &bvid,
                        played_time,
                        real_played_time,
                        real_played_time,
                        start_ts,
                        0, // play_type: 0 = playing
                    ).await;
                }
                result = child.wait() => {
                    let real_played_time = start_time.elapsed().as_secs() as i64;

                    let _ = crate::api::heartbeat::report_heartbeat(
                        &api_client,
                        aid,
                        cid,
                        &bvid,
                        played_time,
                        real_played_time,
                        real_played_time,
                        start_ts,
                        4, // play_type: 4 = end
                    ).await;

                    let _ = result;
                    break;
                }
            }
        }

        // Cleanup cookie file
        if let Some(path) = cookie_path_to_clean {
            let _ = tokio::fs::remove_file(path).await;
        }
    });

    Ok(())
}

/// Play a live stream using mpv
/// This function spawns mpv in a background task to avoid blocking the TUI
pub async fn play_live(room_id: i64) -> Result<()> {
    let live_url = format!("https://live.bilibili.com/{}", room_id);

    let mut cmd = Command::new("mpv");
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    cmd.arg("--force-window=immediate");
    cmd.arg(&live_url);

    let mut child = cmd.spawn()?;

    // Spawn a background task to wait for the process
    // This prevents blocking the TUI
    tokio::spawn(async move {
        let _ = child.wait().await;
    });

    Ok(())
}
