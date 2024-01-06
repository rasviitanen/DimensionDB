use std::path::Path;
use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};

use crate::mdlist::ToCoords;
use std::{
    cmp,
    io::{self, Read, Seek, SeekFrom, Write},
    sync::Arc,
};

pub const PAGE_SIZE: u64 = 4096 * 1;
const CHUNK_SIZE: u64 = 4096 * 2;

#[repr(align(4096))]
struct Aligned([u8; CHUNK_SIZE as usize]);

#[derive(Debug, Clone, Copy)]
pub struct PageId(pub u64);

impl<const DIM: usize> ToCoords<DIM> for PageId {
    #[inline]
    fn to_coords(mut self) -> [u8; DIM] {
        [(); DIM].map(|_| {
            let k = (self.0 % DIM as u64) as u8;
            self.0 /= DIM as u64;
            k
        })
    }
}

#[derive(Clone)]
pub struct PageSlottedFile {
    file: Arc<std::fs::File>,
}

impl PageSlottedFile {
    pub fn open(path: impl AsRef<Path>, ring: &rio::Rio) -> Self {
        let file = Self {
            file: Arc::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .custom_flags(libc::O_DIRECT)
                    .open(path)
                    .unwrap(),
            ),
        };

        let v = Aligned([0; CHUNK_SIZE as usize]);
        ring.write_at(&file.file, &v.0, 0);
        file
    }

    // pub fn allocate_page(&mut self, ring: &rio::Rio) -> anyhow::Result<()> {
    //     let v = Aligned([0; CHUNK_SIZE as usize]);
    //     ring.write_at(&self.file, &v.0, pos);
    //     Ok(())
    // }

    pub fn open_additional(path: impl AsRef<Path>) -> Self {
        Self {
            file: Arc::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    // .custom_flags(libc::O_DIRECT)
                    .open(path)
                    .unwrap(),
            ),
        }
    }
}

impl Read for PageSlottedFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl Write for PageSlottedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Seek for PageSlottedFile {
    fn seek(&mut self, style: SeekFrom) -> io::Result<u64> {
        self.file.seek(style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slotted_page() {
        // let page = SlottedPage::default();
    }
}
