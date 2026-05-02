pub mod connection;
pub mod encoding;
pub mod metainfo;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::bittorent::{connection::peer_discovering, encoding::Bencoding, metainfo::MetaInfo};

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(name = "decode")]
    Decode { encoded_value: String },

    #[command(name = "info")]
    Info { file_path: String },

    #[command(name = "peers")]
    Peers { file_path: String },
}

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Command::Decode { encoded_value } => {
                let decoded_value = Bencoding::decode(encoded_value.into_bytes())?;
                println!("{}", decoded_value);
                Ok(())
            }
            Command::Info { file_path } => {
                let metainfo = MetaInfo::from_file(&file_path)?;
                println!("Tracker URL: {}", metainfo.announce);
                println!("Length: {}", metainfo.info.length);
                println!("Info Hash: {}", hex::encode(metainfo.info.hash));
                println!("Piece Length: {}", metainfo.info.piece_length);
                println!("Piece Hashes:");
                for piece in metainfo.info.pieces {
                    println!("{}", hex::encode(piece));
                }
                Ok(())
            }
            Command::Peers { file_path } => {
                let metainfo = MetaInfo::from_file(&file_path)?;
                let (_, peers) = peer_discovering(&metainfo)?;
                for peer in peers {
                    println!("{peer}");
                }
                Ok(())
            }
        }
    }
}
