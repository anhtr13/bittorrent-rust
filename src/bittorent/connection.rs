use anyhow::Result;

use crate::bittorent::{encoding::Bencoding, metainfo::MetaInfo};

pub fn peer_discovering(metainfo: &MetaInfo) -> Result<(u64, Vec<String>)> {
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact={}",
        metainfo.announce,
        url_encode(&metainfo.info.hash).as_str(),
        "00112233445566778899",
        "6881",
        "0",
        "0",
        metainfo.info.length.to_string().as_str(),
        "1"
    );
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

fn url_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| format!("%{}", hex::encode([b])))
        .collect::<Vec<_>>()
        .join("")
}
