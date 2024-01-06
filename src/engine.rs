use std::{
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use rio::Rio;
use tokio::sync::oneshot;

use crate::{
    io::{BufReaderWithPos, BufWriterWithPos},
    layout::{PageId, PageSlottedFile, PAGE_SIZE},
    mdlist::MdList,
};

pub struct Engine {
    ring: Rio,
    // FIXME:(rasviitanen) merge all into one state
    index: Arc<MdList<String, CommandPos>>,
    pool: Arc<rayon::ThreadPool>,
    // Dynamic value pointer in this page
    cell_ptr: Arc<AtomicU64>,
    // Cell count for this page, to know how to store keys in the page trailer
    cell_count: Arc<AtomicU64>,
    // Size of unallocated memory in the page
    unallocated: Arc<AtomicU64>,
    file: Arc<PageSlottedFile>,
}

impl Engine {
    pub fn new(threads: usize) -> Self {
        let ring = rio::new().unwrap();
        let file = Arc::new(PageSlottedFile::open("gen1", &ring));

        Self {
            index: Default::default(),
            pool: Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .unwrap(),
            ),
            ring,
            file,
            cell_ptr: Default::default(),
            cell_count: Default::default(),
            unallocated: Arc::new(AtomicU64::new(PAGE_SIZE)),
        }
    }

    pub async fn get(&self, key: String) -> anyhow::Result<Option<String>> {
        let index = self.index.clone();
        let (tx, rx) = oneshot::channel();
        self.pool.spawn(move || {
            let res = (|| {
                if let Some(cmd_pos) = index.get(key.as_str()) {
                    let mut reader =
                        BufReaderWithPos::new(PageSlottedFile::open_additional("gen1")).unwrap();
                    reader.seek(SeekFrom::Start(cmd_pos.pos))?;
                    let mut cmd_reader = (&mut reader).take(cmd_pos.len);
                    let mut out = String::new();
                    let _ = cmd_reader.read_to_string(&mut out);
                    Ok(Some(out))
                } else {
                    Ok(None)
                }
            })();
            let _ = tx.send(res);
        });

        rx.await?
    }

    pub async fn set(&self, key: String, value: String) -> anyhow::Result<()> {
        let content_len = (value.len() + key.len()) as u64;
        let index = self.index.clone();
        let cell_ptr = self.cell_ptr.clone();
        let count = self.cell_count.clone();
        let unallocated = self.unallocated.clone();
        let (tx, rx) = oneshot::channel();
        self.pool.spawn(move || {
            let _ = tx.send((|| {
                let mut writer =
                    BufWriterWithPos::new(PageSlottedFile::open_additional("gen1")).unwrap();
                let page = cell_ptr.load(Ordering::SeqCst) / PAGE_SIZE;
                let curr_unallocated = unallocated.load(Ordering::SeqCst);

                if curr_unallocated < content_len {
                    // FIXME:(rasviitanen) make concurrent
                    cell_ptr.store((page + 1) * PAGE_SIZE, Ordering::SeqCst);
                    count.store(0, Ordering::SeqCst);
                    unallocated.store(PAGE_SIZE, Ordering::SeqCst);
                }

                let _ = writer.seek(SeekFrom::Start(cell_ptr.load(Ordering::SeqCst)));
                let pos = writer.pos;
                // serde_json::to_writer(&mut writer, &value)?;
                let _ = writer.write(value.as_bytes());
                writer.flush()?;

                cell_ptr.fetch_add(writer.pos - pos, Ordering::SeqCst);

                index.insert(key.as_ref(), (PageId(page), pos..writer.pos).into());
                let unallocated_end = unallocated.fetch_sub(content_len, Ordering::SeqCst);
                count.fetch_add(1, Ordering::SeqCst);
                let _ = writer.seek(SeekFrom::Current(unallocated_end as i64));
                // serde_json::to_writer(&mut writer, &key)?;
                let _ = writer.write(key.as_bytes());
                writer.flush()?;

                Ok(())
            })());
        });

        rx.await?
    }
}

/// Struct representing a command
#[derive(serde::Serialize, serde::Deserialize, Debug)]
enum Command {
    Set { key: String, value: String },
    Remove { key: String },
}

impl Command {
    fn set(key: String, value: String) -> Command {
        Command::Set { key, value }
    }

    fn remove(key: String) -> Command {
        Command::Remove { key }
    }
}

/// Represents the position and length of a json-serialized command in the log
#[derive(Debug, Clone, Copy)]
struct CommandPos {
    page: PageId,
    pos: u64,
    len: u64,
}

impl From<(PageId, Range<u64>)> for CommandPos {
    fn from((page, range): (PageId, Range<u64>)) -> Self {
        CommandPos {
            page,
            pos: range.start,
            len: range.end - range.start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[tokio::test]
    // async fn test_update() -> anyhow::Result<()> {
    //     let engine = Engine::new(1);
    //     engine
    //         .set(String::from("key0"), String::from("value0"))
    //         .await?;

    //     engine
    //         .set(String::from("key1"), String::from("value1"))
    //         .await?;

    //     engine
    //         .set(String::from("key1"), String::from("value1.v2"))
    //         .await?;

    //     assert_eq!(
    //         engine.get(String::from("key0")).await?,
    //         Some(String::from("value0"))
    //     );
    //     assert_eq!(
    //         engine.get(String::from("key1")).await?,
    //         Some(String::from("value1.v2"))
    //     );
    //     Ok(())
    // }

    #[tokio::test]
    async fn test_allocate_page() -> anyhow::Result<()> {
        let engine = Engine::new(1);
        for k in 0..4 {
            engine.set(format!("key{k}"), format!("value{k}")).await?;
        }

        for k in 0..4 {
            assert_eq!(
                engine.get(format!("key{k}")).await?,
                Some(format!("value{k}"))
            );
        }

        Ok(())
    }
}
