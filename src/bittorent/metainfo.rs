use std::fs;

use anyhow::Result;
use sha1::{Digest, Sha1};

use crate::bittorent::encoding::Bencoding;

#[allow(unused)]
pub struct Info {
    pub length: u64,
    pub name: String,
    pub piece_length: u64,
    pub pieces: Vec<Vec<u8>>,
    pub hash: String,
}

pub struct MetaInfo {
    pub announce: String,
    pub info: Info,
}

impl MetaInfo {
    pub fn from_file(file_path: &str) -> Result<Self> {
        let data = fs::read(file_path)?;
        let Bencoding::Dictionary(dict) = Bencoding::decode(data)? else {
            anyhow::bail!("metainfo must be a dictionary");
        };
        let Some(announce) = dict.get("announce") else {
            anyhow::bail!("announce not found")
        };
        let Bencoding::String(url) = announce else {
            anyhow::bail!("announce must be string")
        };
        let url = String::from_utf8(url.to_owned())?;
        let Some(info) = dict.get("info") else {
            anyhow::bail!("info not found")
        };
        let mut encoder = Sha1::new();
        encoder.update(info.encode());
        let info_hash = hex::encode(encoder.finalize());
        let Bencoding::Dictionary(info) = info else {
            anyhow::bail!("info must be encode as dictionary")
        };
        let Some(length) = info.get("length") else {
            anyhow::bail!("length not found")
        };
        let Bencoding::Integer(length) = length else {
            anyhow::bail!("length must be encode as integer")
        };
        let Some(plength) = info.get("piece length") else {
            anyhow::bail!("piece length not found")
        };
        let Bencoding::Integer(plength) = plength else {
            anyhow::bail!("piece length must be encode as integer")
        };
        let Some(name) = info.get("name") else {
            anyhow::bail!("name length not found")
        };
        let Bencoding::String(name) = name else {
            anyhow::bail!("name must be encode as string")
        };
        let name = String::from_utf8(name.to_owned())?;
        let Some(pieces) = info.get("pieces") else {
            anyhow::bail!("pieces not found")
        };
        let Bencoding::String(pieces) = pieces else {
            anyhow::bail!("pieces must be encode as string")
        };
        let pieces: Vec<_> = pieces.chunks(20).map(|chunk| chunk.to_vec()).collect();
        Ok(Self {
            announce: url,
            info: Info {
                length: *length as u64,
                name,
                piece_length: *plength as u64,
                pieces,
                hash: info_hash,
            },
        })
    }
}
