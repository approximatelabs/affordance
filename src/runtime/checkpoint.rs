//! Optional, verified recovery images. The complete input journal stays authoritative.
use super::{
    agent::{Agent, recovery::Image},
    blobs::Blobs,
};
use crate::View;
use serde::{
    Deserialize, Serialize,
    de::{DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Value as J, json, value::RawValue};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

/// 2: images may be taken at a step boundary (`noted`, `fuel`); the journal check is its tail.
const FORMAT: u32 = 2;
const INTERVAL: usize = 256;
/// How much of the journal before an image's offset its check covers.
const TAIL: u64 = 64 << 10;

#[derive(Serialize, Deserialize)]
struct Saved {
    format: u32,
    identity: String,
    offset: u64,
    /// The SHA-256 of the journal's last `TAIL` bytes before `offset`.
    tail: String,
    /// Every blob the journal before `offset` refers to: a resume from the image checks they
    /// exist, as a full replay would, without reading the journal.
    blobs: Vec<String>,
    image: Image,
}
/// The file as written: the checksum is of `data`'s exact text. Reading it keeps `data` as text,
/// so a refused image is never built, and an accepted one is read straight into its types (a
/// `Value` tree of an image holds every token id as a separate string: ~25x the file).
#[derive(Deserialize)]
struct Envelope<'a> {
    #[serde(default)]
    checksum: J,
    #[serde(borrow)]
    data: Option<&'a RawValue>,
}
/// What a resume checks before it reads the image.
#[derive(Deserialize)]
struct Header<'a> {
    #[serde(default)]
    format: J,
    #[serde(default)]
    identity: J,
    #[serde(borrow)]
    image: Option<&'a RawValue>,
}
/// The first blob reference in a JSON text whose blob is missing: `Blobs::missing`, read from
/// the text without building it (strings are checked, object keys are not).
fn missing_in(blobs: &Blobs, json: &str) -> Result<Option<String>, String> {
    struct Seek<'b>(&'b Blobs);
    impl<'de> DeserializeSeed<'de> for Seek<'_> {
        type Value = Option<String>;
        fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_any(self)
        }
    }
    impl<'de> Visitor<'de> for Seek<'_> {
        type Value = Option<String>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("JSON")
        }
        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E> {
            Ok(self.0.missing_str(s))
        }
        fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut first = None;
            while let Some(found) = seq.next_element_seed(Seek(self.0))? {
                first = first.or(found);
            }
            Ok(first)
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut first = None;
            while map.next_key::<IgnoredAny>()?.is_some() {
                first = first.or(map.next_value_seed(Seek(self.0))?);
            }
            Ok(first)
        }
    }
    let mut de = serde_json::Deserializer::from_str(json);
    Seek(blobs).deserialize(&mut de).map_err(|e| e.to_string())
}

pub(super) struct Loaded {
    pub image: Image,
    pub offset: u64,
    pub refs: BTreeSet<String>,
}
/// Where the journal ends, and the latest step boundary found while replaying it.
#[derive(Default)]
pub(super) struct Seed {
    /// The input count of the image the agent was restored from (`None`: a full replay).
    pub saved: Option<usize>,
    pub bytes: u64,
    pub boundary: Option<(usize, u64)>,
    /// The image's offset: the suffix replayed from it starts here (0: a full replay).
    pub offset: u64,
    /// The blobs the journal refers to.
    pub refs: BTreeSet<String>,
}

/// The blob references in a journal line (as JSON text: `"\u0000blob:<sha>:<bytes>:<chars>"`).
pub(super) fn refs_in(line: &str, into: &mut BTreeSet<String>) {
    const MARK: &str = "\\u0000blob:";
    let mut rest = line;
    while let Some(i) = rest.find(MARK) {
        let tail = &rest[i + MARK.len()..];
        let end = tail.find('"').unwrap_or(tail.len());
        // A reference is a whole JSON string of this shape. Text that only contains the mark (a
        // tool's output quoting another journal, where the backslash is itself escaped) is not
        // one, and must not make an image depend on a blob that does not exist.
        let opens_string = rest[..i].ends_with('"') && !rest[..i].ends_with("\\\"");
        let mut parts = tail[..end].split(':');
        let shaped = parts
            .next()
            .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
            && parts
                .next()
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            && parts
                .next()
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            && parts.next().is_none();
        if opens_string && shaped {
            into.insert(format!("\0blob:{}", &tail[..end]));
        }
        rest = &tail[end..];
    }
}

/// The journal's bytes just before `offset`, hashed: an image belongs to this journal when they
/// match and `offset` ends a line. Reading the whole prefix would make every resume O(history);
/// the journal stays authoritative, and a full replay still reads all of it.
fn tail_hash(path: &Path, offset: u64) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    if len < offset {
        return Err("checkpoint journal prefix is truncated".into());
    }
    let from = offset.saturating_sub(TAIL);
    let mut buffer = vec![0u8; (offset - from) as usize];
    file.seek(SeekFrom::Start(from)).map_err(|e| e.to_string())?;
    file.read_exact(&mut buffer).map_err(|e| e.to_string())?;
    if offset > 0 && buffer.last() != Some(&b'\n') {
        return Err("checkpoint cursor is not a complete journal boundary".into());
    }
    let mut hash = Sha256::new();
    hash.update(offset.to_le_bytes());
    hash.update(&buffer);
    Ok(format!("{:x}", hash.finalize()))
}

pub(super) fn load(dir: &Path, identity: &str, blobs: Option<&Blobs>) -> Result<Option<Loaded>, String> {
    let file = match std::fs::read(dir.join("checkpoint.json")) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let envelope: Envelope = serde_json::from_slice(&file).map_err(|e| e.to_string())?;
    let data = envelope.data.ok_or("checkpoint payload is missing")?.get();
    if envelope.checksum.as_str() != Some(&crate::sha256_hex(data.as_bytes())) {
        return Err("checkpoint checksum mismatch".into());
    }
    let header: Header = serde_json::from_str(data).map_err(|e| e.to_string())?;
    if header.format != json!(FORMAT) {
        return Err("checkpoint format changed".into());
    }
    if header.identity.as_str() != Some(identity) {
        return Err("checkpoint configuration changed".into());
    }
    let saved: Saved = serde_json::from_str(data).map_err(|e| e.to_string())?;
    if tail_hash(&dir.join("journal.jsonl"), saved.offset)? != saved.tail {
        return Err("checkpoint journal prefix changed".into());
    }
    if let (Some(blobs), Some(image)) = (blobs, header.image)
        && let Some(missing) = missing_in(blobs, image.get())?
    {
        return Err(format!("checkpoint refers to missing blob {missing}"));
    }
    if let Some(missing) = blobs.and_then(|b| b.missing(&json!(saved.blobs))) {
        return Err(format!(
            "the journal refers to a missing blob ({missing}); copy the run's blobs/ directory with it"
        ));
    }
    Ok(Some(Loaded {
        image: saved.image,
        offset: saved.offset,
        refs: saved.blobs.into_iter().collect(),
    }))
}

pub(super) struct Checkpoint {
    path: PathBuf,
    journal: PathBuf,
    identity: String,
    bytes: u64,
    /// The latest step boundary the agent kept: its input count and the journal's length there.
    boundary: Option<(usize, u64)>,
    /// The last input count an image was refused at (the next try waits an interval).
    refused: Option<usize>,
    /// A refusal was reported since the last image (one report per run of refusals).
    told: bool,
    saved: Option<usize>,
    /// The blobs the journal refers to, and how many of them there were at the latest boundary.
    refs: BTreeSet<String>,
    boundary_refs: Option<BTreeSet<String>>,
}
impl Checkpoint {
    pub fn new(dir: &Path, identity: String, restored: Option<Seed>) -> Result<Self, String> {
        let journal = dir.join("journal.jsonl");
        let seed = restored.unwrap_or_else(|| Seed {
            bytes: std::fs::metadata(&journal).map(|m| m.len()).unwrap_or(0),
            ..Seed::default()
        });
        Ok(Self {
            path: dir.join("checkpoint.json"),
            journal,
            identity,
            bytes: seed.bytes,
            boundary: seed.boundary,
            boundary_refs: seed.boundary.map(|_| seed.refs.clone()),
            refused: None,
            told: false,
            saved: seed.saved,
            refs: seed.refs,
        })
    }
    /// Called only after the exact input bytes have been appended and synced.
    pub fn appended(&mut self, line: &[u8]) {
        self.bytes += line.len() as u64;
        if let Ok(text) = std::str::from_utf8(line) {
            refs_in(text, &mut self.refs);
        }
    }
    /// After an input: if the agent kept a step boundary in it, that is where the journal ends now.
    pub fn handled(&mut self, agent: &Agent) {
        if let Some(n) = agent.boundary_inputs().filter(|n| *n == agent.journal_len())
            && self.boundary.map(|b| b.0) != Some(n)
        {
            self.boundary = Some((n, self.bytes));
            self.boundary_refs = Some(self.refs.clone());
        }
    }
    /// Write an image when one is due (every `INTERVAL` inputs, or `force`): of the agent if it is
    /// idle, else of its latest step boundary.
    /// `before_write` runs before the image is written (it syncs the blobs the image may name).
    /// Returns why the agent could not be imaged, the first time since its last image it could not.
    pub fn save(
        &mut self,
        agent: &Agent,
        view: &View,
        force: bool,
        before_write: &dyn Fn() -> Result<(), String>,
    ) -> Result<Option<&'static str>, String> {
        match self.prepare(agent, view, force)? {
            Due::Nothing(why) => Ok(why),
            Due::Image(image) => {
                before_write()?;
                image.write()?;
                self.wrote(&image);
                Ok(None)
            }
        }
    }
    /// The image `image` is on disk.
    pub fn wrote(&mut self, image: &Ready) {
        self.saved = Some(image.inputs);
        self.told = false;
    }
    /// The image that is due, ready to write, without writing it: a caller with several
    /// instances to image (a worker that is stopping) writes them all at once. The blobs it may
    /// name are synced before it is written.
    pub fn prepare(&mut self, agent: &Agent, view: &View, force: bool) -> Result<Due, String> {
        let count = agent.journal_len();
        let since = self.saved.max(self.refused).unwrap_or(0);
        if self.saved == Some(count)
            || self.refused == Some(count)
            || (!force && count.saturating_sub(since) < INTERVAL)
        {
            return Ok(Due::Nothing(None));
        }
        let idle = if agent.quiescent() {
            agent.checkpoint_image(view)
        } else {
            Err("work is in progress")
        };
        let why = idle.as_ref().err().copied();
        let chosen = idle
            .ok()
            .map(|image| (count, self.bytes, image, self.refs.clone()))
            .or_else(|| {
                let (n, bytes) = self.boundary.filter(|(n, _)| self.saved.is_none_or(|s| *n > s))?;
                agent
                    .boundary_image(view)
                    .filter(|image| image.inputs() == n)
                    .map(|image| (n, bytes, image, self.boundary_refs.clone().unwrap_or_default()))
            });
        let Some((inputs, offset, image, blobs)) = chosen else {
            self.refused = Some(count);
            return Ok(Due::Nothing(why.filter(|_| !std::mem::replace(&mut self.told, true))));
        };
        // Written as text, never as a `Value` tree (see `Envelope`).
        let data = serde_json::to_string(&Saved {
            format: FORMAT,
            identity: self.identity.clone(),
            offset,
            tail: tail_hash(&self.journal, offset)?,
            blobs: blobs.into_iter().collect(),
            image,
        })
        .map_err(|e| e.to_string())?;
        // With AFFORDANCE_VERIFY_IMAGES set (the test suite sets it), an image of an idle agent
        // is read back, as a resume would read it, and the agent it gives is compared with the
        // one it was taken from: the same state, the same outstanding work, the same counters.
        // Every image any test writes is checked this way, whatever the test is about.
        // (An image of a step boundary is of an earlier moment than the live agent, even within
        // the same input: it is checked by the tests that resume from one.)
        if why.is_none() && std::env::var_os("AFFORDANCE_VERIFY_IMAGES").is_some() {
            let read: Saved = serde_json::from_str(&data).map_err(|e| format!("image verification: {e}"))?;
            let blobs = agent.blobs_handle();
            let restored = read.image.restore(agent.cfg.clone(), view, |a| {
                if let Some(b) = blobs {
                    a.set_blobs(b);
                }
            })?;
            let differs = if restored.state() != agent.state() {
                Some("state")
            } else if restored.outstanding() != agent.outstanding() {
                Some("outstanding work")
            } else if restored.metrics != agent.metrics {
                Some("counters")
            } else {
                None
            };
            // A panic, so the test that wrote the image fails, whatever it was testing.
            if let Some(what) = differs {
                panic!(
                    "image verification: the agent restored from the image written at input {inputs} differs from the live one in its {what}"
                );
            }
        }
        let envelope = format!(
            "{{\"checksum\":\"{}\",\"data\":{data}}}",
            crate::sha256_hex(data.as_bytes())
        );
        Ok(Due::Image(Ready {
            path: self.path.clone(),
            envelope,
            inputs,
        }))
    }
}

/// What `Cache::prepare` found.
pub enum Due {
    /// No image now; with a reason, the first time since the last image that there could be none.
    Nothing(Option<&'static str>),
    Image(Ready),
}

/// A recovery image ready to be written.
pub struct Ready {
    path: PathBuf,
    envelope: String,
    inputs: usize,
}
impl Ready {
    /// Write it in place of the last one, durably.
    pub fn write(&self) -> Result<(), String> {
        super::host::atomic_bytes(&self.path, self.envelope.as_bytes())
    }
}
