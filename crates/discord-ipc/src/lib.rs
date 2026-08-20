// Copyright (C) 2026 rain (despondence) <308736589+despondence@users.noreply.github.com>
// SPDX-License-Identifier: AGPL-3.0-or-later

use bytes::Buf;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::Path;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use zerocopy::little_endian::U32;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use self::model::{Args, Cmd, Handshake, SetActivity};

pub mod env;
pub mod model;

#[derive(Debug, Default, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned)]
#[repr(C)]
pub struct Header {
    pub opcode: U32,
    pub len: U32,
}

pub struct IpcStream {
    stream: UnixStream,
}

impl IpcStream {
    pub async fn connect(path: impl AsRef<Path>) -> io::Result<Self> {
        UnixStream::connect(path)
            .await
            .map(|stream| Self { stream })
    }

    pub async fn send(&mut self, opcode: u32, message: impl Serialize) -> io::Result<()> {
        let string = serde_json::to_string(&message)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

        let len = u32::try_from(string.len())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

        let header = Header {
            opcode: U32::from(opcode),
            len: U32::from(len),
        };

        let mut buf = Buf::chain(header.as_bytes(), string.as_bytes());

        self.stream.write_all_buf(&mut buf).await
    }

    pub async fn recv<D: DeserializeOwned>(&mut self) -> io::Result<(u32, D)> {
        let mut header = Header::default();

        self.stream.read_exact(header.as_mut_bytes()).await?;

        let mut bytes = vec![0; header.len.get() as usize];

        self.stream.read_exact(&mut bytes).await?;

        serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
    }

    pub async fn roundtrip<D: DeserializeOwned>(
        &mut self,
        opcode: u32,
        message: impl Serialize,
    ) -> io::Result<(u32, D)> {
        self.send(opcode, message).await?;
        self.recv::<D>().await
    }

    pub async fn handshake(&mut self, client_id: u64) -> io::Result<serde_json::Value> {
        let message = Handshake { v: 1, client_id };
        let (_opcode, message) = self.roundtrip(0, message).await?;

        Ok(message)
    }

    pub async fn set_activity(
        &mut self,
        process_id: u32,
        activity: Option<model::Activity>,
    ) -> io::Result<serde_json::Value> {
        let message = SetActivity {
            cmd: Cmd::SetActivity,
            args: Args {
                pid: process_id,
                activity,
            },
            nonce: uuid::Uuid::new_v4(),
        };

        let (_opcode, message) = self.roundtrip(1, message).await?;

        Ok(message)
    }
}
