use std::collections::BTreeMap;

use anyhow::Result;

use crate::bittorent::encoding::Bencoding;

pub struct ExtensionMetadata {
    pub ut_metadata: u8,
    pub ut_pex: Option<u8>,
}

pub struct ExtensionMessage {
    pub id: u8,
    pub metadata: ExtensionMetadata,
}

impl ExtensionMessage {
    pub fn new(ut_metadata: u8, ut_pex: Option<u8>) -> Self {
        Self {
            id: 0,
            metadata: ExtensionMetadata {
                ut_metadata,
                ut_pex,
            },
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut metadata = BTreeMap::new();
        metadata.insert(
            String::from("ut_metadata"),
            Bencoding::Integer(self.metadata.ut_metadata as i64),
        );
        if let Some(ut_pex) = self.metadata.ut_pex {
            metadata.insert(String::from("ut_pex"), Bencoding::Integer(ut_pex as i64));
        };
        let dict = BTreeMap::from([(String::from("m"), Bencoding::Dictionary(metadata))]);
        let mut bytes = vec![self.id];
        bytes.extend(Bencoding::Dictionary(dict).encode());
        bytes
    }

    pub fn decode(bytes: Vec<u8>) -> Result<Self> {
        anyhow::ensure!(bytes.len() > 0);
        let id = bytes[0];
        let Bencoding::Dictionary(mut dict) = Bencoding::decode(bytes[1..].to_owned())? else {
            anyhow::bail!("parse failed")
        };
        let Some(Bencoding::Dictionary(mut metadata)) = dict.remove("m") else {
            anyhow::bail!("metadata 'm' not found")
        };
        let Some(Bencoding::Integer(ut_metadata)) = metadata.remove("ut_metadata") else {
            anyhow::bail!("ut_metadata not found")
        };
        let ut_pex = match metadata.remove("ut_pex") {
            Some(Bencoding::Integer(ut_pex)) => Some(ut_pex as u8),
            _ => None,
        };
        Ok(Self {
            id,
            metadata: ExtensionMetadata {
                ut_metadata: ut_metadata as u8,
                ut_pex,
            },
        })
    }
}
