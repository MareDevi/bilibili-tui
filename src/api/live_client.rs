//! Bilibili Live WebSocket Client
//!
//! Manages WebSocket connection for receiving live stream messages.

use super::live_ws::{
    make_auth_packet, make_heartbeat_packet, parse_message, DanmuInfoData, LiveMessage, Packet,
};
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Live WebSocket client
pub struct LiveClient {
    /// Sender to signal shutdown
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// Receiver for live messages
    message_rx: mpsc::Receiver<LiveMessage>,
}

impl LiveClient {
    /// Connect to live room WebSocket
    pub async fn connect(room_id: i64, uid: i64, danmu_info: &DanmuInfoData) -> Result<Self> {
        // Get first available host
        let host = danmu_info
            .host_list
            .first()
            .ok_or_else(|| anyhow!("No WebSocket hosts available"))?;

        let url = host.wss_url();
        let token = danmu_info.token.clone();

        // Create channels
        let (message_tx, message_rx) = mpsc::channel::<LiveMessage>(256);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Spawn connection task
        tokio::spawn(async move {
            let _ = run_connection(&url, room_id, uid, &token, message_tx, &mut shutdown_rx).await;
        });

        Ok(Self {
            shutdown_tx: Some(shutdown_tx),
            message_rx,
        })
    }

    /// Try to receive a message (non-blocking)
    pub fn try_recv(&mut self) -> Option<LiveMessage> {
        self.message_rx.try_recv().ok()
    }

    /// Disconnect from WebSocket
    pub async fn disconnect(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }
}

impl Drop for LiveClient {
    fn drop(&mut self) {
        // Signal shutdown (best effort)
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.try_send(());
        }
    }
}

/// Run WebSocket connection loop
async fn run_connection(
    url: &str,
    room_id: i64,
    uid: i64,
    token: &str,
    message_tx: mpsc::Sender<LiveMessage>,
    shutdown_rx: &mut mpsc::Receiver<()>,
) -> Result<()> {
    // Connect to WebSocket
    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Send auth packet
    let auth_packet = make_auth_packet(room_id, uid, token);
    write.send(Message::Binary(auth_packet.into())).await?;

    // Heartbeat interval (30 seconds)
    let mut heartbeat_interval = interval(Duration::from_secs(30));
    heartbeat_interval.tick().await; // Skip first immediate tick

    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.recv() => {
                break;
            }

            // Send heartbeat
            _ = heartbeat_interval.tick() => {
                let hb = make_heartbeat_packet();
                if write.send(Message::Binary(hb.into())).await.is_err() {
                    break;
                }
            }

            // Receive messages
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        let _ = process_message(&data[..], &message_tx).await;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Err(_)) => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

/// Process received WebSocket message
async fn process_message(data: &[u8], message_tx: &mpsc::Sender<LiveMessage>) -> Result<()> {
    let packets = Packet::decode(data)?;

    for packet in packets {
        if let Some(msg) = parse_message(&packet) {
            // Send message (ignore if channel is full)
            let _ = message_tx.try_send(msg);
        }
    }

    Ok(())
}
