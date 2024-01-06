//! Very inefficient emulated files until we can have efficient IO
use std::path::Path;
use std::{fs::OpenOptions, io::Result, os::unix::fs::OpenOptionsExt};

use std::{
    cmp,
    io::{self, Read, Seek, SeekFrom, Write},
    sync::Arc,
};

use parking_lot::RwLock;

const CHUNK_SIZE: u64 = 4096 * 256;

#[repr(align(4096))]
struct Aligned([u8; CHUNK_SIZE as usize]);

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
struct RawFile {
    pub inner: Vec<u8>,
}

#[derive(Default, Clone)]
pub struct DbFile {
    pos: u64,
    memory: Arc<RwLock<RawFile>>,
    // We should only have one here.. to be fixed
    file: Option<Arc<std::fs::File>>,
}

impl DbFile {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .custom_flags(libc::O_DIRECT)
            .open(path)?;

        Ok(Self {
            pos: 0,
            memory: Default::default(),
            file: Some(Arc::new(file)),
        })
    }

    pub fn size(&self) -> usize {
        self.memory.read().inner.len()
    }

    pub async fn save(&mut self) {
        todo!();
    }
}

impl Read for DbFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let amt = cmp::min(self.pos, self.memory.read().inner.len() as u64);
        let mut fill_buff = &self.memory.read().inner[(amt as usize)..];
        let n = Read::read(&mut fill_buff, buf)?;
        self.pos += n as u64;

        Ok(n)
    }
}

impl Write for DbFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let pos: usize = self.pos as usize;
        let len: usize = self.memory.read().inner.len();
        if len < pos {
            self.memory.write().inner.resize(pos, 0);
        }

        {
            let space = self.memory.read().inner.len() - pos;
            let (left, right) = buf.split_at(cmp::min(space, buf.len()));
            self.memory.write().inner[pos..pos + left.len()].copy_from_slice(left);
            self.memory.write().inner.extend_from_slice(right);
        }

        // Bump us forward
        self.pos = (pos + buf.len()) as u64;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for DbFile {
    fn seek(&mut self, style: SeekFrom) -> io::Result<u64> {
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.memory.read().inner.len() as u64, n),
            SeekFrom::Current(n) => (self.pos, n),
        };
        let new_pos = if offset >= 0 {
            base_pos.checked_add(offset as u64)
        } else {
            base_pos.checked_sub((offset.wrapping_neg()) as u64)
        };
        match new_pos {
            Some(n) => {
                self.pos = n;
                Ok(self.pos)
            }
            None => Err(std::io::Error::new(std::io::ErrorKind::Other, "Uh oh")),
        }
    }
}
