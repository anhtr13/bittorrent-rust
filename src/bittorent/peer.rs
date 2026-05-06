use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::TcpStream,
};

use anyhow::{Context, Result};
use rand::{RngExt, distr::Alphanumeric};
use sha1::{Digest, Sha1};

use crate::bittorent::{encoding::Bencoding, torrent::Torrent};

pub const BLOCK_SIZE: u64 = 16384;

#[allow(clippy::too_many_arguments)]
pub fn discover_peers(
    torrent: &Torrent,
    port: u16,
    uploaded: u32,
    downloaded: u32,
    left: u64,
    compact: bool,
) -> Result<(u64, Vec<String>)> {
    let peer_id = new_peer_id();
    let url = format!(
        "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact={}",
        torrent.announce,
        url_encode(&torrent.info.hash).as_str(),
        peer_id,
        port,
        uploaded,
        downloaded,
        left,
        compact as u8
    );
    let client = reqwest::blocking::Client::new();
    let res = client.get(&url).send()?.bytes()?.to_vec();
    let data = Bencoding::decode(res)?;
    let Bencoding::Dictionary(dict) = data else {
        anyhow::bail!("tracker response must be a dictionary")
    };
    let Some(Bencoding::Integer(interval)) = dict.get("interval") else {
        anyhow::bail!("failed to parse tracker response");
    };
    let Some(Bencoding::String(peers)) = dict.get("peers") else {
        anyhow::bail!("failed to parse tracker response");
    };
    let interval = *interval as u64;
    let peers: Vec<_> = peers
        .chunks(6)
        .map(|addr| {
            let port = u16::from_be_bytes([addr[4], addr[5]]);
            format!("{}.{}.{}.{}:{}", addr[0], addr[1], addr[2], addr[3], port)
        })
        .collect();
    Ok((interval, peers))
}

pub fn establish_hanshake(stream: &mut TcpStream, info_hash: &[u8; 20]) -> Result<Vec<u8>> {
    let protocol = String::from("BitTorrent protocol");
    let reserved = [0u8; 8];
    let peer_id = new_peer_id();

    let mut buf = Vec::new();
    buf.push(protocol.len() as u8);
    buf.extend(protocol.into_bytes());
    buf.extend(reserved);
    buf.extend(info_hash);
    buf.extend(peer_id.as_bytes());
    stream.write_all(&buf)?;

    let mut buf = [0u8; 68];
    stream.read_exact(&mut buf)?;
    anyhow::ensure!(buf[0] == 19);
    anyhow::ensure!(&buf[1..20] == b"BitTorrent protocol");
    anyhow::ensure!(&buf[28..48] == info_hash);

    Ok(buf[48..].to_owned())
}

pub fn download_piece(
    stream: &mut TcpStream,
    piece_index: u64,
    torrent: &Torrent,
    ouput: &str,
) -> Result<()> {
    let total_length = torrent.info.length;
    let Some(piece_hash) = torrent.info.pieces.get(piece_index as usize) else {
        anyhow::bail!("piece_index out of range");
    };
    let piece_length = torrent.info.piece_length;
    let mut piece_length = total_length
        .saturating_sub(piece_length * piece_index)
        .min(piece_length);
    let piece_index = piece_index as u32;
    let mut buffer = Vec::<u8>::new();
    let mut offset: u32 = 0;

    while piece_length > 0 {
        let length = piece_length.min(BLOCK_SIZE) as u32;
        piece_length = piece_length.saturating_sub(BLOCK_SIZE);

        let mut payload = Vec::new();
        payload.extend(piece_index.to_be_bytes());
        payload.extend(offset.to_be_bytes());
        payload.extend(length.to_be_bytes());

        let request = Message::new(MessageId::Request, payload);
        stream.write_all(&request.into_bytes())?;

        let block = Message::from_stream(stream)?;
        anyhow::ensure!(block.id == MessageId::Piece);
        anyhow::ensure!(block.payload.len() >= 8);
        anyhow::ensure!(&block.payload[..4] == piece_index.to_be_bytes());
        anyhow::ensure!(&block.payload[4..8] == offset.to_be_bytes());

        buffer.extend(&block.payload[8..]);
        offset += length;
    }

    let mut encoder = Sha1::new();
    encoder.update(&buffer);
    let hash: [u8; 20] = encoder
        .finalize()
        .to_vec()
        .try_into()
        .map_err(|_| anyhow::Error::msg("failed to encode piece to 20 bytes array"))?;

    anyhow::ensure!(&hash == piece_hash, "piece_hash miss match");

    let mut file = OpenOptions::new().create(true).append(true).open(ouput)?;
    file.write_all(&buffer)?;
    Ok(())
}

pub fn new_peer_id() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(char::from)
        .collect()
}

#[derive(PartialEq, Clone, Copy)]
pub enum MessageId {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
}

pub struct Message {
    pub id: MessageId,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new(id: MessageId, payload: Vec<u8>) -> Self {
        Self { id, payload }
    }

    pub fn from_stream(stream: &mut TcpStream) -> Result<Self> {
        let mut buf = [0u8; 4];
        stream
            .read_exact(&mut buf)
            .context("failed to read message length")?;

        let length = u32::from_be_bytes(buf);
        anyhow::ensure!(length > 0);

        let mut id = [0u8; 1];
        stream
            .read_exact(&mut id)
            .context("failed to read message id")?;

        let length = length as usize - 1;
        if length == 0 {
            return Ok(Self {
                id: MessageId::try_from(id[0])?,
                payload: Vec::new(),
            });
        }

        let mut payload = vec![0u8; length];
        stream
            .read_exact(&mut payload)
            .context("failed to read message payload")?;

        Ok(Self {
            id: MessageId::try_from(id[0])?,
            payload,
        })
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let length = self.payload.len() as u32 + 1;
        let mut bytes = Vec::new();
        bytes.extend(length.to_be_bytes());
        bytes.push(self.id as u8);
        bytes.extend(self.payload);
        bytes
    }
}

impl TryFrom<u8> for MessageId {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Choke),
            1 => Ok(Self::Unchoke),
            2 => Ok(Self::Interested),
            3 => Ok(Self::NotInterested),
            4 => Ok(Self::Have),
            5 => Ok(Self::Bitfield),
            6 => Ok(Self::Request),
            7 => Ok(Self::Piece),
            8 => Ok(Self::Cancel),
            v => anyhow::bail!("Invalid message id: {}", v),
        }
    }
}

fn url_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| format!("%{}", hex::encode([b])))
        .collect::<Vec<_>>()
        .join("")
}
