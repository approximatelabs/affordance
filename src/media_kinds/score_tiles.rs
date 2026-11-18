//! Score-weighted InternVL tiles (Llama-3.1-Nemotron-Nano-VL).

/// Llama-3.1-Nemotron-Nano-VL-8B-V1: its image processor (`LlamaNemotronNanoVLImageProcessor`,
/// repository `image_processing.py`; vLLM `LlamaNemotronNanoVLProcessor`) cuts the picture into
/// square tiles of `tile` px. Every layout (cols, rows) with 1 <= cols x rows <= `max_tiles`,
/// in order of tile count, gets the score min(cols x rows x tile² / (W x H), `area_cap`) x
/// min(layout aspect / picture aspect, picture aspect / layout aspect); the first layout with
/// the highest score wins. A layout of more than one tile adds a thumbnail tile when `thumbnail`
/// is set. The template writes `<image>`, which the model's `_format_image_token` replaces with
/// `start` + `per_tile` x pieces copies of the placeholder + `end`. The picture is scored as
/// given (transformers 5.x and vLLM never double it). Reference counts come from the official
/// processor run in Docker (transformers 5.19.0) and agree with vLLM v0.33.0 on 7464 sizes.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let int = |key: &str| -> Result<u64, String> {
        params
            .get(key)
            .and_then(|v| v.as_u64())
            .ok_or_else(|| format!("score_tiles: missing integer parameter `{key}`"))
    };
    let text = |key: &str| -> Result<String, String> {
        params
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| format!("score_tiles: missing string parameter `{key}`"))
    };
    let tile = int("tile")?;
    let per_tile = int("per_tile")? as usize;
    let max_tiles = int("max_tiles")?;
    let thumbnail = params.get("thumbnail").and_then(|v| v.as_bool()).unwrap_or(true);
    let area_cap = params.get("area_cap").and_then(|v| v.as_f64()).unwrap_or(0.6);
    let start = text("start")?;
    let end = text("end")?;
    if width == 0 || height == 0 {
        return Err(format!("score_tiles: empty picture {width}x{height}"));
    }
    if tile == 0 || max_tiles == 0 {
        return Err("score_tiles: tile and max_tiles must be positive".to_string());
    }

    // Python: aspect_ratio = orig_width / orig_height; area = width * height (exact ints).
    let aspect = width as f64 / height as f64;
    let area = (width as u64 * height as u64) as f64;

    // Layouts sorted by tile count. Within one count the Python set order is arbitrary, but
    // layouts with the same count give the same number of pieces, so only the count order
    // matters: a later count wins only with a strictly higher score.
    let mut best_score = f64::NEG_INFINITY;
    let mut best_n = 1u64;
    for n in 1..=max_tiles {
        for cols in (1..=n).filter(|c| n % c == 0) {
            let rows = n / cols;
            let target = cols as f64 / rows as f64;
            let area_part = py_min((n * tile * tile) as f64 / area, area_cap);
            let aspect_part = py_min(target / aspect, aspect / target);
            let score = area_part * aspect_part;
            if score > best_score {
                best_score = score;
                best_n = n;
            }
        }
    }
    let pieces = best_n as usize + usize::from(thumbnail && best_n != 1);
    Ok(vec![(Some(start), 1), (None, per_tile * pieces), (Some(end), 1)])
}

/// Python's `min(a, b)`: `a` unless `b < a`.
fn py_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

#[cfg(test)]
mod tests {
    use super::layout;

    fn params() -> serde_json::Value {
        serde_json::json!({
            "tile": 512, "per_tile": 256, "max_tiles": 12, "thumbnail": true,
            "area_cap": 0.6, "start": "<img>", "end": "</img>"
        })
    }

    #[test]
    fn a_llama_nemotron_nano_vl_picture_is_laid_out_as_its_processor_lays_it_out() {
        let reference = [
            ("64x64", 258),
            ("224x224", 258),
            ("256x256", 258),
            ("448x448", 258),
            ("512x512", 258),
            ("640x480", 3330),
            ("800x600", 3330),
            ("1024x768", 3330),
            ("1280x720", 2306),
            ("1920x1080", 2306),
            ("2000x200", 2818),
            ("200x2000", 2818),
            ("768x768", 1282),
            ("1000x1000", 1282),
            ("100x37", 2818),
            ("513x513", 258),
            ("1023x1023", 1282),
            ("3000x3000", 3330),
            ("4000x500", 2306),
            ("17x900", 3330),
            ("600x800", 3330),
            ("1600x900", 2306),
            ("33x33", 258),
            ("4096x4096", 2562),
            ("1500x1499", 2562),
        ];
        let p = params();
        for (size, want) in reference {
            let (w, h) = size.split_once('x').unwrap();
            let runs = layout(&p, w.parse().unwrap(), h.parse().unwrap()).unwrap();
            let got: usize = runs.iter().map(|(_, n)| n).sum();
            assert_eq!(got, want, "{size}");
        }
        // 1280x720: 4 columns by 2 rows plus a thumbnail, 9 pieces of 256.
        assert_eq!(
            layout(&p, 1280, 720).unwrap(),
            vec![
                (Some("<img>".to_string()), 1),
                (None, 2304),
                (Some("</img>".to_string()), 1),
            ]
        );
        assert!(layout(&p, 0, 10).is_err());
    }
}
