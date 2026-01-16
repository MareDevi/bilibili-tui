//! Bilibili Live WebSocket Protocol
//!
//! Implements packet encoding/decoding and message parsing for live stream messages.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::io::Read;

/// Header size in bytes
const HEADER_SIZE: usize = 16;

/// Operation codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OpCode {
    Heartbeat = 2,
    HeartbeatReply = 3,
    Message = 5,
    Auth = 7,
    AuthReply = 8,
}

impl TryFrom<u32> for OpCode {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self> {
        match value {
            2 => Ok(OpCode::Heartbeat),
            3 => Ok(OpCode::HeartbeatReply),
            5 => Ok(OpCode::Message),
            7 => Ok(OpCode::Auth),
            8 => Ok(OpCode::AuthReply),
            _ => Err(anyhow!("Unknown opcode: {}", value)),
        }
    }
}

/// Protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ProtoVer {
    Raw = 0,
    Zlib = 2,
    Brotli = 3,
}

/// Packet structure
#[derive(Debug, Clone)]
pub struct Packet {
    pub proto_ver: u16,
    pub op_code: u32,
    pub seq_id: u32,
    pub body: Vec<u8>,
}

impl Packet {
    /// Create a new packet
    pub fn new(op_code: OpCode, body: Vec<u8>) -> Self {
        Self {
            proto_ver: 1,
            op_code: op_code as u32,
            seq_id: 1,
            body,
        }
    }

    /// Encode packet to bytes
    pub fn encode(&self) -> Vec<u8> {
        let packet_len = (HEADER_SIZE + self.body.len()) as u32;
        let header_len: u16 = HEADER_SIZE as u16;

        let mut buf = Vec::with_capacity(packet_len as usize);

        // Packet length (4 bytes, big-endian)
        buf.extend_from_slice(&packet_len.to_be_bytes());
        // Header length (2 bytes, big-endian)
        buf.extend_from_slice(&header_len.to_be_bytes());
        // Protocol version (2 bytes, big-endian)
        buf.extend_from_slice(&self.proto_ver.to_be_bytes());
        // Operation code (4 bytes, big-endian)
        buf.extend_from_slice(&self.op_code.to_be_bytes());
        // Sequence ID (4 bytes, big-endian)
        buf.extend_from_slice(&self.seq_id.to_be_bytes());
        // Body
        buf.extend_from_slice(&self.body);

        buf
    }

    /// Decode packets from bytes (may contain multiple packets)
    pub fn decode(data: &[u8]) -> Result<Vec<Packet>> {
        let mut packets = Vec::new();
        let mut offset = 0;

        while offset + HEADER_SIZE <= data.len() {
            // Parse header
            let packet_len = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            let header_len = u16::from_be_bytes([data[offset + 4], data[offset + 5]]) as usize;
            let proto_ver = u16::from_be_bytes([data[offset + 6], data[offset + 7]]);
            let op_code = u32::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]);
            let seq_id = u32::from_be_bytes([
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15],
            ]);

            if offset + packet_len > data.len() {
                break;
            }

            let body = data[offset + header_len..offset + packet_len].to_vec();

            // Handle compressed data
            let body = match proto_ver {
                3 => {
                    // Brotli compressed
                    decompress_brotli(&body)?
                }
                2 => {
                    // Zlib compressed (legacy)
                    decompress_zlib(&body)?
                }
                _ => body,
            };

            // If decompressed, may contain nested packets
            if proto_ver == 3 || proto_ver == 2 {
                let nested = Self::decode(&body)?;
                packets.extend(nested);
            } else {
                packets.push(Packet {
                    proto_ver,
                    op_code,
                    seq_id,
                    body,
                });
            }

            offset += packet_len;
        }

        Ok(packets)
    }
}

/// Decompress brotli data
fn decompress_brotli(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = brotli::Decompressor::new(data, 4096);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

/// Decompress zlib data (legacy)
fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>> {
    use std::io::Cursor;
    let mut decoder = flate2::read::ZlibDecoder::new(Cursor::new(data));
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

/// Authentication packet body
#[derive(Debug, Serialize)]
pub struct AuthBody {
    pub uid: i64,
    pub roomid: i64,
    pub protover: u8,
    pub platform: String,
    #[serde(rename = "type")]
    pub auth_type: u8,
    pub key: String,
}

/// Create authentication packet
pub fn make_auth_packet(room_id: i64, uid: i64, token: &str) -> Vec<u8> {
    let body = AuthBody {
        uid,
        roomid: room_id,
        protover: 3, // Use brotli compression
        platform: "web".to_string(),
        auth_type: 2,
        key: token.to_string(),
    };

    let body_json = serde_json::to_vec(&body).unwrap_or_default();
    Packet::new(OpCode::Auth, body_json).encode()
}

/// Create heartbeat packet
pub fn make_heartbeat_packet() -> Vec<u8> {
    Packet::new(OpCode::Heartbeat, b"[object Object]".to_vec()).encode()
}

// ============ Message Types ============

/// Parsed live message
#[derive(Debug, Clone)]
pub enum LiveMessage {
    /// Danmaku message
    Danmaku {
        uid: i64,
        uname: String,
        content: String,
        color: u32,
    },
    /// User entered room
    Enter { uid: i64, uname: String },
    /// Gift sent
    Gift {
        uid: i64,
        uname: String,
        gift_name: String,
        gift_id: i64,
        num: i32,
        price: i64,
    },
    /// Popularity update (from heartbeat reply)
    Popularity(u32),
    /// Online rank data
    OnlineRank { list: Vec<RankUser> },
    /// Auth reply
    AuthReply { code: i32 },
    /// Unknown/unhandled message
    Unknown(String),
}

/// Rank user info
#[derive(Debug, Clone, Deserialize)]
pub struct RankUser {
    pub uid: i64,
    pub uname: String,
    pub rank: i32,
    #[serde(default)]
    pub face: String,
    #[serde(default)]
    pub score: String,
}

/// Raw message from WebSocket
#[derive(Debug, Deserialize)]
struct RawMessage {
    cmd: String,
    #[serde(default)]
    info: Option<serde_json::Value>,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

/// Parse packet into LiveMessage
pub fn parse_message(packet: &Packet) -> Option<LiveMessage> {
    match OpCode::try_from(packet.op_code).ok()? {
        OpCode::HeartbeatReply => {
            // First 4 bytes are popularity count
            if packet.body.len() >= 4 {
                let popularity = u32::from_be_bytes([
                    packet.body[0],
                    packet.body[1],
                    packet.body[2],
                    packet.body[3],
                ]);
                return Some(LiveMessage::Popularity(popularity));
            }
            None
        }
        OpCode::AuthReply => {
            // Parse auth reply JSON
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&packet.body) {
                let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
                return Some(LiveMessage::AuthReply { code });
            }
            None
        }
        OpCode::Message => {
            // Parse JSON message
            let json_str = String::from_utf8_lossy(&packet.body);
            parse_json_message(&json_str)
        }
        _ => None,
    }
}

/// Parse JSON message string
fn parse_json_message(json_str: &str) -> Option<LiveMessage> {
    let raw: RawMessage = serde_json::from_str(json_str).ok()?;

    match raw.cmd.as_str() {
        "DANMU_MSG" => parse_danmu_msg(&raw.info?),
        "INTERACT_WORD" => parse_interact_word(&raw.data?),
        "SEND_GIFT" => parse_send_gift(&raw.data?),
        "ONLINE_RANK_V2" => parse_online_rank(&raw.data?),
        cmd if cmd.starts_with("DANMU_MSG") => parse_danmu_msg(&raw.info?),
        _ => Some(LiveMessage::Unknown(raw.cmd)),
    }
}

/// Parse DANMU_MSG
fn parse_danmu_msg(info: &serde_json::Value) -> Option<LiveMessage> {
    let info_arr = info.as_array()?;

    // info[1] = content
    let content = info_arr.get(1)?.as_str()?.to_string();

    // info[2] = [uid, uname, ...]
    let user_arr = info_arr.get(2)?.as_array()?;
    let uid = user_arr.first()?.as_i64().unwrap_or(0);
    let uname = user_arr.get(1)?.as_str().unwrap_or("").to_string();

    // info[0][3] = color (decimal)
    let color = info_arr
        .first()?
        .as_array()?
        .get(3)?
        .as_u64()
        .unwrap_or(0xFFFFFF) as u32;

    Some(LiveMessage::Danmaku {
        uid,
        uname,
        content,
        color,
    })
}

/// Parse INTERACT_WORD (user entering room)
fn parse_interact_word(data: &serde_json::Value) -> Option<LiveMessage> {
    let uid = data.get("uid")?.as_i64().unwrap_or(0);
    let uname = data.get("uname")?.as_str().unwrap_or("").to_string();

    // msg_type: 1 = enter, 2 = follow, 3 = share
    let msg_type = data.get("msg_type")?.as_i64().unwrap_or(0);

    if msg_type == 1 {
        Some(LiveMessage::Enter { uid, uname })
    } else {
        None
    }
}

/// Parse SEND_GIFT
fn parse_send_gift(data: &serde_json::Value) -> Option<LiveMessage> {
    let uid = data.get("uid")?.as_i64().unwrap_or(0);
    let uname = data.get("uname")?.as_str().unwrap_or("").to_string();
    let gift_name = data.get("giftName")?.as_str().unwrap_or("").to_string();
    let gift_id = data.get("giftId")?.as_i64().unwrap_or(0);
    let num = data.get("num")?.as_i64().unwrap_or(1) as i32;
    let price = data.get("price")?.as_i64().unwrap_or(0);

    Some(LiveMessage::Gift {
        uid,
        uname,
        gift_name,
        gift_id,
        num,
        price,
    })
}

/// Parse ONLINE_RANK_V2
fn parse_online_rank(data: &serde_json::Value) -> Option<LiveMessage> {
    let list_arr = data.get("list")?.as_array()?;
    let list: Vec<RankUser> = list_arr
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    Some(LiveMessage::OnlineRank { list })
}

// ============ API Types ============

/// Danmu info response
#[derive(Debug, Deserialize)]
pub struct DanmuInfoData {
    pub token: String,
    pub host_list: Vec<DanmuHost>,
}

/// Danmu host info
#[derive(Debug, Clone, Deserialize)]
pub struct DanmuHost {
    pub host: String,
    pub port: u16,
    pub wss_port: u16,
    pub ws_port: u16,
}

impl DanmuHost {
    /// Get WebSocket URL
    pub fn wss_url(&self) -> String {
        format!("wss://{}:{}/sub", self.host, self.wss_port)
    }
}

// ============ History Danmaku Types ============

/// History danmaku response
#[derive(Debug, Deserialize)]
pub struct HistoryDanmakuData {
    #[serde(default)]
    pub admin: Vec<HistoryDanmakuItem>,
    #[serde(default)]
    pub room: Vec<HistoryDanmakuItem>,
}

/// History danmaku item
#[derive(Debug, Deserialize)]
pub struct HistoryDanmakuItem {
    pub text: String,
    pub uid: i64,
    pub nickname: String,
    #[serde(default)]
    pub uname_color: String,
    pub timeline: String,
}
