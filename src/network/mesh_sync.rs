use std::net::{UdpSocket, SocketAddr, Ipv4Addr};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex};
use std::collections::BTreeMap;

const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(239, 255, 0, 1);
const PORT: u16 = 50051;

#[derive(Clone)]
pub struct AudioPacket {
    pub timestamp_us: u64,
    pub payload: Vec<f32>,
}

impl AudioPacket {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.timestamp_us.to_le_bytes());
        for &sample in &self.payload {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 { return None; }
        let mut ts_bytes = [0u8; 8];
        ts_bytes.copy_from_slice(&bytes[0..8]);
        let timestamp_us = u64::from_le_bytes(ts_bytes);

        let mut payload = Vec::new();
        let mut i = 8;
        while i + 4 <= bytes.len() {
            let mut float_bytes = [0u8; 4];
            float_bytes.copy_from_slice(&bytes[i..i+4]);
            payload.push(f32::from_le_bytes(float_bytes));
            i += 4;
        }

        Some(Self { timestamp_us, payload })
    }
}

pub struct MeshBroadcaster {
    socket: UdpSocket,
    addr: SocketAddr,
}

impl MeshBroadcaster {
    pub fn new() -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let addr = SocketAddr::new(MULTICAST_IP.into(), PORT);
        Ok(Self { socket, addr })
    }

    pub fn broadcast(&self, block: &[f32]) -> std::io::Result<usize> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64;
        let packet = AudioPacket {
            timestamp_us: now,
            payload: block.to_vec(),
        };
        let bytes = packet.to_bytes();
        self.socket.send_to(&bytes, &self.addr)
    }
}

pub struct MeshReceiver {
    socket: UdpSocket,
    pub buffer: Arc<Mutex<BTreeMap<u64, Vec<f32>>>>,
}

impl MeshReceiver {
    pub fn new() -> std::io::Result<Self> {
        let addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), PORT);
        let socket = UdpSocket::bind(&addr)?;
        socket.join_multicast_v4(&MULTICAST_IP, &Ipv4Addr::UNSPECIFIED)?;
        socket.set_nonblocking(true)?;
        
        Ok(Self {
            socket,
            buffer: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }

    pub fn poll(&self) {
        let mut buf = [0u8; 65536];
        while let Ok((len, _src)) = self.socket.recv_from(&mut buf) {
            if let Some(packet) = AudioPacket::from_bytes(&buf[..len]) {
                if let Ok(mut map) = self.buffer.lock() {
                    map.insert(packet.timestamp_us, packet.payload);
                    
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64;
                    map.retain(|&ts, _| now.saturating_sub(ts) < 1_000_000);
                }
            }
        }
    }
}
