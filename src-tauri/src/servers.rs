//! Reads a per-instance `servers.dat` and, separately, queries a server live
//! over the real Minecraft Server List Ping protocol.

use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEntry {
    pub name: String,
    pub address: String,
    pub icon_data_url: Option<String>,
    /// The game's own `acceptTextures` flag; `None` if never prompted,
    /// `Some(true)`/`Some(false)` for an explicit accept/decline.
    pub resource_pack_accepted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub online: bool,
    pub players_online: Option<i64>,
    pub players_max: Option<i64>,
    pub ping_ms: Option<u64>,
    pub favicon_data_url: Option<String>,
    pub error: Option<String>,
}

// servers.dat is plain, uncompressed NBT (unlike gzipped level.dat); a
// separate minimal reader from worlds.rs's, the shapes don't share enough.

struct NbtReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> NbtReader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        if end > self.data.len() {
            return None;
        }
        let s = &self.data[self.pos..end];
        self.pos = end;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(*self.take(1)?.first()?)
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_be_bytes(self.take(2)?.try_into().ok()?))
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_be_bytes(self.take(4)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<String> {
        let len = self.u16()? as usize;
        Some(String::from_utf8_lossy(self.take(len)?).into_owned())
    }

    /// Reads and discards the payload of one tag whose id/name are already
    /// consumed, so unrecognized fields don't break parsing of the ones we want.
    fn skip_value(&mut self, tag: u8) -> Option<()> {
        match tag {
            1 => {
                self.take(1)?;
            }
            2 => {
                self.take(2)?;
            }
            3 | 5 => {
                self.take(4)?;
            }
            4 | 6 => {
                self.take(8)?;
            }
            7 => {
                let n = self.i32()?.max(0) as usize;
                self.take(n)?;
            }
            8 => {
                self.string()?;
            }
            9 => {
                let elem = self.u8()?;
                let n = self.i32()?.max(0);
                for _ in 0..n {
                    self.skip_value(elem)?;
                }
            }
            10 => loop {
                let id = self.u8()?;
                if id == 0 {
                    break;
                }
                self.string()?;
                self.skip_value(id)?;
            },
            11 => {
                let n = self.i32()?.max(0) as usize;
                self.take(n.checked_mul(4)?)?;
            }
            12 => {
                let n = self.i32()?.max(0) as usize;
                self.take(n.checked_mul(8)?)?;
            }
            _ => return None,
        }
        Some(())
    }
}

fn read_server_entry(r: &mut NbtReader) -> Option<ServerEntry> {
    let mut name = None;
    let mut ip = None;
    let mut icon_b64 = None;
    let mut accept_textures: Option<bool> = None;
    loop {
        let id = r.u8()?;
        if id == 0 {
            break;
        }
        let field = r.string()?;
        match (id, field.as_str()) {
            (8, "name") => name = Some(r.string()?),
            (8, "ip") => ip = Some(r.string()?),
            (8, "icon") => icon_b64 = Some(r.string()?),
            (1, "acceptTextures") => accept_textures = Some(r.u8()? != 0),
            _ => r.skip_value(id)?,
        }
    }
    Some(ServerEntry {
        name: name.unwrap_or_default(),
        address: ip.unwrap_or_default(),
        icon_data_url: icon_b64.map(|b64| format!("data:image/png;base64,{b64}")),
        resource_pack_accepted: accept_textures,
    })
}

fn read_servers_dat(path: &Path) -> Option<Vec<ServerEntry>> {
    let buf = std::fs::read(path).ok()?;
    let mut r = NbtReader { data: &buf, pos: 0 };
    let root_tag = r.u8()?;
    if root_tag != 10 {
        return None;
    }
    r.string()?; // root name, unused

    let mut servers = Vec::new();
    loop {
        let id = r.u8()?;
        if id == 0 {
            break;
        }
        let field = r.string()?;
        if id == 9 && field == "servers" {
            let elem = r.u8()?;
            let n = r.i32()?.max(0);
            for _ in 0..n {
                if elem == 10 {
                    servers.push(read_server_entry(&mut r)?);
                } else {
                    r.skip_value(elem)?;
                }
            }
        } else {
            r.skip_value(id)?;
        }
    }
    Some(servers)
}

pub fn list_servers(instance_dir: &Path) -> AppResult<Vec<ServerEntry>> {
    let path = instance_dir.join("servers.dat");
    Ok(read_servers_dat(&path).unwrap_or_default())
}

// Server List Ping: handshake -> status request/response -> ping/pong
// (Java Edition protocol, 1.7+), framed as [VarInt length][id][payload].

fn split_address(address: &str) -> (String, u16) {
    if let Some((host, port)) = address.rsplit_once(':') {
        if let Ok(p) = port.parse::<u16>() {
            return (host.to_string(), p);
        }
    }
    (address.to_string(), 25565)
}

fn write_varint(buf: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if v == 0 {
            break;
        }
    }
}

async fn read_varint(stream: &mut TcpStream) -> AppResult<i32> {
    let mut result: i32 = 0;
    let mut shift = 0u32;
    loop {
        let mut b = [0u8; 1];
        stream.read_exact(&mut b).await?;
        result |= ((b[0] & 0x7F) as i32) << shift;
        if b[0] & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            return Err(AppError::Other("malformed varint in server response".into()));
        }
    }
    Ok(result)
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_varint(buf, bytes.len() as i32);
    buf.extend_from_slice(bytes);
}

fn write_framed_packet(buf: &mut Vec<u8>, body: &[u8]) {
    write_varint(buf, body.len() as i32);
    buf.extend_from_slice(body);
}

/// Handshake (0x00): protocol version (-1 lets the server report its own),
/// address, port, next_state (1 enters the Status sub-protocol).
async fn send_handshake(stream: &mut TcpStream, host: &str, port: u16) -> AppResult<()> {
    let mut hs_body = Vec::new();
    write_varint(&mut hs_body, 0x00);
    write_varint(&mut hs_body, -1);
    write_string(&mut hs_body, host);
    hs_body.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut hs_body, 1);
    let mut packet = Vec::new();
    write_framed_packet(&mut packet, &hs_body);
    stream.write_all(&packet).await?;
    Ok(())
}

struct StatusInfo {
    players_online: Option<i64>,
    players_max: Option<i64>,
    favicon_data_url: Option<String>,
}

/// One connection: handshake, status request/response, then ping/pong.
/// Proxies like Hypixel's Velocity silently drop a Ping with no prior Status.
async fn fetch_status_and_ping(host: &str, port: u16) -> AppResult<(StatusInfo, u64)> {
    let mut stream = TcpStream::connect((host, port)).await?;
    stream.set_nodelay(true).ok();
    send_handshake(&mut stream, host, port).await?;

    // Status request (0x00): empty body.
    stream.write_all(&[0x01, 0x00]).await?;

    // Status response (0x00): a length-prefixed JSON string.
    let _packet_len = read_varint(&mut stream).await?;
    let packet_id = read_varint(&mut stream).await?;
    if packet_id != 0x00 {
        return Err(AppError::Other(format!("unexpected status packet id {packet_id}")));
    }
    let json_len = read_varint(&mut stream).await? as usize;
    let mut json_buf = vec![0u8; json_len];
    stream.read_exact(&mut json_buf).await?;
    let value: serde_json::Value =
        serde_json::from_slice(&json_buf).map_err(|e| AppError::Other(format!("bad status response: {e}")))?;

    let status = StatusInfo {
        players_online: value.pointer("/players/online").and_then(|v| v.as_i64()),
        players_max: value.pointer("/players/max").and_then(|v| v.as_i64()),
        favicon_data_url: value.get("favicon").and_then(|v| v.as_str()).map(|s| s.to_string()),
    };

    // Ping/pong (0x01), timed only from here so the status payload's own
    // transfer time never pollutes the reported latency.
    let start = Instant::now();
    let mut ping_body = Vec::new();
    write_varint(&mut ping_body, 0x01);
    ping_body.extend_from_slice(&0i64.to_be_bytes());
    let mut ping_packet = Vec::new();
    write_framed_packet(&mut ping_packet, &ping_body);
    stream.write_all(&ping_packet).await?;

    let _pong_len = read_varint(&mut stream).await?;
    let _pong_id = read_varint(&mut stream).await?;
    let mut pong_payload = [0u8; 8];
    stream.read_exact(&mut pong_payload).await?;

    Ok((status, start.elapsed().as_millis() as u64))
}

pub async fn ping_server(address: &str) -> AppResult<ServerStatus> {
    let (host, port) = split_address(address);
    let outcome = timeout(Duration::from_secs(6), fetch_status_and_ping(&host, port)).await;

    match outcome {
        Ok(Ok((status, ping_ms))) => Ok(ServerStatus {
            online: true,
            players_online: status.players_online,
            players_max: status.players_max,
            ping_ms: Some(ping_ms),
            favicon_data_url: status.favicon_data_url,
            error: None,
        }),
        Ok(Err(e)) => Ok(ServerStatus {
            online: false,
            error: Some(e.to_string()),
            ..Default::default()
        }),
        Err(_) => Ok(ServerStatus {
            online: false,
            error: Some("timed out".into()),
            ..Default::default()
        }),
    }
}
