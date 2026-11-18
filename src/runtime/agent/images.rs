//! Media sources a view names: data URLs only. An image's type can be sniffed from its bytes.

/// What a prompt's `<img>`, `<audio>` and `<video>` spans carry, in prompt order: the model
/// request sends them with the ids (each lays out its placeholder tokens; `src` is the image or
/// the clip). A source is a data URL, and its type says which of the three it is.
pub(super) fn image_sources(spans: &[crate::emit::Span]) -> Result<Vec<String>, String> {
    // A file that could not be laid out went as its alt text: say so, the first few times.
    static SAID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    for span in spans.iter().filter(|s| s.meta.get("not_shown").is_some()) {
        if SAID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 5 {
            eprintln!(
                "affordance: {} was not shown to the model, and its alt text stands in: {}",
                match span.meta["media"].as_str() {
                    Some("audio") => "a sound clip",
                    Some("video") => "a video",
                    _ => "a picture",
                },
                span.meta["not_shown"].as_str().unwrap_or("")
            );
        }
    }
    let mut imgs: Vec<&crate::emit::Span> = spans.iter().filter(|s| s.meta.get("img").is_some()).collect();
    imgs.sort_by_key(|s| s.start);
    imgs.iter()
        .map(|s| {
            let element = s.meta["media"].as_str().unwrap_or("img");
            let src = s.meta["src"]
                .as_str()
                .ok_or_else(|| format!("an <{element}> in a model's prompt needs src= (what its request sends)"))?;
            match element {
                // A picture layout gave another size is sent at that size (src/media.rs).
                "img" => match (s.meta["size"][0].as_u64(), s.meta["size"][1].as_u64()) {
                    (Some(w), Some(h)) => crate::media::at_size(&image_source(src)?, (w as u32, h as u32)),
                    _ => image_source(src),
                },
                // A clip layout trimmed is sent trimmed.
                "audio" if s.meta["clip"]["ms"].is_u64() => crate::media::clipped(
                    &clip_source(src, "audio")?,
                    s.meta["clip"]["ms"].as_u64().unwrap_or(0),
                    s.meta["clip"]["from_end"] == true,
                ),
                kind => clip_source(src, kind),
            }
        })
        .collect()
}

/// Where each media element's tokens are in a prompt's ids, in order, and its placeholder:
/// (start, count, placeholder).
/// Layout holds one placeholder for each token the file takes, which is what the model reads;
/// a server that expands a placeholder itself is sent each run as one.
pub(super) fn media_runs(spans: &[crate::emit::Span]) -> Vec<(usize, usize, u32)> {
    let mut runs: Vec<(usize, usize, u32)> = spans
        .iter()
        .filter(|s| s.meta.get("img").is_some() && s.end > s.start)
        .filter_map(|s| Some((s.start, s.end - s.start, s.meta["placeholder"].as_u64()? as u32)))
        .collect();
    runs.sort();
    runs
}

/// An audio or video clip a model request may carry: a data URL of that kind (`kind`: `audio`
/// or `video`). Never a URL or a path, for the reason an image is not one.
pub(crate) fn clip_source(src: &str, kind: &str) -> Result<String, String> {
    let types: &[&str] = match kind {
        "audio" => &["wav", "x-wav", "mpeg", "mp3", "flac", "ogg", "webm", "mp4", "aac"],
        _ => &["mp4", "webm", "quicktime", "x-matroska"],
    };
    let typed = src
        .strip_prefix("data:")
        .and_then(|rest| rest.strip_prefix(kind))
        .and_then(|rest| rest.strip_prefix('/'))
        .and_then(|rest| rest.split_once(";base64,"))
        .is_some_and(|(t, _)| types.contains(&t));
    if typed {
        return Ok(src.to_string());
    }
    let shown: String = src.chars().take(40).collect();
    Err(format!(
        "an <{kind}> source must be a data URL (data:{kind}/{};base64,...), not a URL or a path: {shown:?}",
        types[0]
    ))
}

/// The media of one kind (`image`, `audio`, `video`) among a request's sources, in order. A
/// source that is not an audio or a video data URL is an image.
pub(crate) fn of_kind<'a>(sources: &'a [String], kind: &str) -> Vec<&'a str> {
    let is = |s: &str, kind: &str| s.starts_with(&format!("data:{kind}/"));
    sources
        .iter()
        .map(String::as_str)
        .filter(|s| match kind {
            "image" => !is(s, "audio") && !is(s, "video"),
            kind => is(s, kind),
        })
        .collect()
}

/// An image a model request may carry: a data URL of a PNG, JPEG, WebP or GIF, or those bytes in
/// base64 (sent as a data URL). Never a URL or a path, which a model server would fetch or open
/// (CSP `img-src data:`): a page's images are what it holds, not what it points at.
pub(crate) fn image_source(src: &str) -> Result<String, String> {
    const TYPES: [&str; 4] = ["png", "jpeg", "webp", "gif"];
    if let Some(rest) = src.strip_prefix("data:image/") {
        if rest.split_once(";base64,").is_some_and(|(t, _)| TYPES.contains(&t)) {
            return Ok(src.to_string());
        }
    } else if let Some(t) = sniff_base64(src) {
        return Ok(format!("data:image/{t};base64,{src}"));
    }
    let shown: String = src.chars().take(40).collect();
    Err(format!(
        "an image source must be a data URL (data:image/png;base64,...) or base64 PNG, JPEG, WebP or GIF bytes, not a URL or a path: {shown:?}"
    ))
}

/// The image type of base64 text whose leading bytes are a PNG, JPEG, WebP or GIF signature.
pub(super) fn sniff_base64(s: &str) -> Option<&'static str> {
    let sextet = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    if !s.bytes().all(|c| sextet(c).is_some() || c == b'=') {
        return None;
    }
    let head: Vec<u8> = s
        .as_bytes()
        .get(..16)?
        .chunks(4)
        .flat_map(|q| {
            let n = q.iter().fold(0u32, |n, &c| n << 6 | sextet(c).unwrap_or(0) as u32);
            [(n >> 16) as u8, (n >> 8) as u8, n as u8]
        })
        .collect();
    match &head[..] {
        [0x89, b'P', b'N', b'G', ..] => Some("png"),
        [0xFF, 0xD8, 0xFF, ..] => Some("jpeg"),
        [b'G', b'I', b'F', b'8', ..] => Some("gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("webp"),
        _ => None,
    }
}
