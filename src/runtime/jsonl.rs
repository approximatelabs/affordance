//! One recovery policy for durable append-only JSONL, at every instance depth.
use serde::de::DeserializeOwned;
use std::{
    io::{BufRead, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
};

pub(crate) struct Row<T> {
    pub offset: u64,
    pub len: usize,
    /// Where the next line starts (after this one's newline).
    pub end: u64,
    pub value: T,
}
pub(crate) struct JsonLines<T> {
    reader: std::io::BufReader<std::fs::File>,
    path: PathBuf,
    kind: &'static str,
    offset: u64,
    done: bool,
    finish_line: bool,
    marker: PhantomData<T>,
}
impl<T: DeserializeOwned> JsonLines<T> {
    pub fn open(path: &Path, kind: &'static str) -> Result<Option<Self>, String> {
        Self::open_at(path, kind, 0)
    }
    pub fn open_at(path: &Path, kind: &'static str, offset: u64) -> Result<Option<Self>, String> {
        use std::io::{Seek, SeekFrom};
        let mut file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
        Ok(Some(Self {
            reader: std::io::BufReader::new(file),
            path: path.into(),
            kind,
            offset,
            done: false,
            finish_line: false,
            marker: PhantomData,
        }))
    }
    fn repair_torn(&self, bytes: &[u8]) -> Result<(), String> {
        let parent = self.path.parent().ok_or("JSONL path has no parent")?;
        let mut number = 0;
        let mut saved = loop {
            let name = if number == 0 {
                format!("torn-{}-{}.bin", self.kind, self.offset)
            } else {
                format!("torn-{}-{}-{number}.bin", self.kind, self.offset)
            };
            match std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(parent.join(name))
            {
                Ok(file) => break file,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => number += 1,
                Err(e) => return Err(e.to_string()),
            }
        };
        saved
            .write_all(bytes)
            .and_then(|_| saved.sync_all())
            .map_err(|e| e.to_string())?;
        std::fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;
        file.set_len(self.offset)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn next_row(&mut self) -> Result<Option<Row<T>>, String> {
        // Yield the final record before completing its terminator. Consumers can reject
        // semantic corruption (such as a store counter moving backwards) without editing
        // the file. Every recovery caller drains this iterator before opening for append.
        if self.finish_line {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&self.path)
                .map_err(|e| e.to_string())?;
            file.write_all(b"\n")
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            self.finish_line = false;
            return Ok(None);
        }
        loop {
            let mut bytes = vec![];
            let count = self.reader.read_until(b'\n', &mut bytes).map_err(|e| e.to_string())?;
            if count == 0 {
                return Ok(None);
            }
            let newline = bytes.ends_with(b"\n");
            let body = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
            let body = body.strip_suffix(b"\r").unwrap_or(body);
            if body.is_empty() {
                self.offset += count as u64;
                continue;
            }
            let json = match serde_json::from_slice::<serde_json::Value>(body) {
                Ok(json) => json,
                Err(error)
                    if !newline
                        && error.is_eof()
                        && std::str::from_utf8(body).err().is_none_or(|e| e.error_len().is_none()) =>
                {
                    self.repair_torn(&bytes)?;
                    return Ok(None);
                }
                Err(error) => return Err(format!("invalid JSON: {error}")),
            };
            // A valid JSON value with a wrong record shape is corruption, never a torn write.
            let value = serde_json::from_value(json).map_err(|e| format!("invalid {} record: {e}", self.kind))?;
            let start = self.offset;
            self.finish_line = !newline;
            self.offset += count as u64 + u64::from(!newline);
            return Ok(Some(Row {
                offset: start,
                len: body.len(),
                end: self.offset,
                value,
            }));
        }
    }
}

impl<T: DeserializeOwned> Iterator for JsonLines<T> {
    type Item = Result<Row<T>, String>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.next_row() {
            Ok(Some(row)) => Some(Ok(row)),
            Ok(None) => {
                self.done = true;
                None
            }
            Err(error) => {
                self.done = true;
                Some(Err(format!("{} at byte {}: {error}", self.path.display(), self.offset)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Record {
        text: String,
    }
    struct Files(PathBuf);
    impl Files {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "aff-jsonl-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn read(&self) -> Result<Vec<Record>, String> {
            JsonLines::<Record>::open(&self.0.join("records.jsonl"), "test")?
                .unwrap()
                .map(|row| row.map(|r| r.value))
                .collect()
        }
    }
    impl Drop for Files {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn every_byte_boundary_recovers_and_can_append_and_recover_again() {
        let committed = b"{\"text\":\"saved\"}\n";
        let next = serde_json::to_vec(&Record {
            text: "emoji 🦀 and 世界".into(),
        })
        .unwrap();
        for end in 1..next.len() {
            let files = Files::new();
            let path = files.0.join("records.jsonl");
            let mut original = committed.to_vec();
            original.extend_from_slice(&next[..end]);
            std::fs::write(&path, &original).unwrap();
            assert_eq!(
                files.read().unwrap(),
                vec![Record { text: "saved".into() }],
                "cut at {end}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), committed);
            let saved = std::fs::read_dir(&files.0)
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| p.extension().is_some_and(|e| e == "bin"))
                .unwrap();
            assert_eq!(std::fs::read(saved).unwrap(), next[..end]);
            let mut append = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
            append.write_all(&next).unwrap(); // valid final record without LF is completed once
            assert_eq!(files.read().unwrap().len(), 2);
            let complete = std::fs::read(&path).unwrap();
            assert!(complete.ends_with(b"\n"));
            assert_eq!(files.read().unwrap().len(), 2);
            assert_eq!(std::fs::read(&path).unwrap(), complete);
        }
    }

    #[test]
    fn complete_wrong_shapes_and_invalid_syntax_never_modify_the_file() {
        for tail in [
            b"{}".as_slice(),
            b"null",
            b"{\"text\":4}",
            b"{\"text\":]}",
            b"{\"text\":\"\xff",
            b"{\"text\":]\"\xf0\x9f",
        ] {
            for newline in [false, true] {
                let files = Files::new();
                let path = files.0.join("records.jsonl");
                let mut bytes = b"{\"text\":\"saved\"}\n".to_vec();
                bytes.extend_from_slice(tail);
                if newline {
                    bytes.push(b'\n')
                }
                std::fs::write(&path, &bytes).unwrap();
                assert!(files.read().is_err(), "accepted {tail:?}");
                assert_eq!(std::fs::read(&path).unwrap(), bytes);
                assert_eq!(std::fs::read_dir(&files.0).unwrap().count(), 1);
            }
        }
    }
}
