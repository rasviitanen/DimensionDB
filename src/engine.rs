use std::{
    future::Future,
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    ops::Range,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use parking_lot::Mutex;
use rio::Rio;
use tokio::sync::{oneshot, RwLock};

use crate::{
    file::DbFile,
    io::{BufReaderWithPos, BufWriterWithPos},
    mdlist::MdList,
};

pub struct Engine {
    current_gen: u64,
    rio: Rio,
    index: Arc<MdList<String, CommandPos>>,
    pool: Arc<rayon::ThreadPool>,
    readers: Arc<MdList<usize, Mutex<crate::io::BufReaderWithPos<DbFile>>>>, // TODO: impl prioq ontop of this with better readers
    writers: Arc<MdList<usize, Mutex<crate::io::BufWriterWithPos<DbFile>>>>, // TODO: impl prioq ontop of this with better writers
}

impl Engine {
    pub fn new(threads: usize) -> Self {
        let current_gen = 1;
        let file = DbFile::open("gen1").unwrap();

        let readers = Arc::new(MdList::default());
        readers.insert(
            current_gen,
            Mutex::new(BufReaderWithPos::new(file.clone()).unwrap()),
        );

        let writers = Arc::new(MdList::default());
        writers.insert(
            current_gen,
            Mutex::new(BufWriterWithPos::new(file).unwrap()),
        );

        Self {
            current_gen,
            index: Default::default(),
            pool: Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .unwrap(),
            ),
            rio: rio::new().unwrap(),
            readers,
            writers,
        }
    }

    pub async fn get(&self, key: String) -> anyhow::Result<Option<String>> {
        let readers = self.readers.clone();
        let index = self.index.clone();
        let gen = self.current_gen;
        let (tx, rx) = oneshot::channel();
        self.pool.spawn(move || {
            let res = (|| {
                if let Some(cmd_pos) = index.get(key.as_str()) {
                    let mut reader = readers
                        .get(gen)
                        .expect("generation should be present")
                        .lock();
                    reader.seek(SeekFrom::Start(cmd_pos.pos))?;
                    let cmd_reader = (&mut *reader).take(cmd_pos.len);
                    if let Command::Set { value, .. } = serde_json::from_reader(cmd_reader)? {
                        Ok(Some(value))
                    } else {
                        Err(anyhow::anyhow!("FIXME:(rasviitanen) Internal Error"))
                    }
                } else {
                    Ok(None)
                }
            })();
            let _ = tx.send(res);
        });

        rx.await?
    }

    pub async fn set(&self, key: String, value: String) -> anyhow::Result<()> {
        let cmd = Command::set(key, value);
        let gen = self.current_gen;
        let writers = self.writers.clone();
        let index = self.index.clone();
        let (tx, rx) = oneshot::channel();
        self.pool.spawn(move || {
            let _ = tx.send((|| {
                let writer = writers.get(gen).unwrap();
                let mut writer = writer.lock();
                let pos = writer.pos;
                serde_json::to_writer(&mut *writer, &cmd)?;
                writer.flush()?;
                if let Command::Set { key, .. } = cmd {
                    index.insert(key.as_ref(), (gen, pos..writer.pos).into());
                }

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
    gen: u64,
    pos: u64,
    len: u64,
}

impl From<(u64, Range<u64>)> for CommandPos {
    fn from((gen, range): (u64, Range<u64>)) -> Self {
        CommandPos {
            gen,
            pos: range.start,
            len: range.end - range.start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine() -> anyhow::Result<()> {
        let engine = Engine::new(1);
        engine
            .set(String::from("key1"), String::from("value1"))
            .await?;
        assert_eq!(
            engine.get(String::from("key1")).await?,
            Some(String::from("value1"))
        );

        Ok(())
    }
}
