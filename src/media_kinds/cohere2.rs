/// Cohere's Command A vision models (CohereLabs command-a-plus-05-2026) lay a picture out with
/// `Cohere2VisionProcessor` and `Cohere2VisionImageProcessor` (transformers 5.19,
/// `models/cohere2_vision/`; SGLang hands pictures to the same processor). The processor lists
/// every grid of `cols` x `rows` tiles with at most `max_tiles` tiles, columns first, stably
/// sorted by tile count. It picks the smallest grid that holds the picture without shrinking it
/// (the first with the largest scale when every grid would shrink it). A 1x1 grid is one block;
/// a larger grid is its tiles plus one thumbnail tile. Each block is `per_tile` patch tokens and
/// a line break, and the whole picture sits between a start and an end marker. The chat
/// template writes only the `<|IMG_PATCH|>` placeholder, so the markers are returned here. The
/// reference counts came from the processor itself, run in `lmsysorg/sglang:v0.5.23`.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let int = |key: &str, default: u64| -> Result<u64, String> {
        match params.get(key) {
            None => Ok(default),
            Some(v) => v
                .as_u64()
                .ok_or_else(|| format!("cohere2: `{key}` must be a whole number")),
        }
    };
    let text = |key: &str, default: &str| -> Result<String, String> {
        match params.get(key) {
            None => Ok(default.to_string()),
            Some(v) => v
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("cohere2: `{key}` must be a string")),
        }
    };
    let tile = int("tile", 512)?;
    let max_tiles = int("max_tiles", 12)?;
    let per_tile = int("per_tile", 256)? as usize;
    let start = text("start", "<|START_OF_IMG|>")?;
    let row_end = text("row_end", "<|IMG_LINE_BREAK|>")?;
    let end = text("end", "<|END_OF_IMG|>")?;
    if width == 0 || height == 0 {
        return Err(format!("cohere2: a {width}x{height} picture has no pixels"));
    }
    if tile == 0 || max_tiles == 0 {
        return Err("cohere2: `tile` and `max_tiles` must be at least 1".to_string());
    }

    // Columns first, then a stable sort by tile count, as the processor lists them.
    let mut grids: Vec<(u64, u64)> = Vec::new();
    for cols in 1..=max_tiles {
        for rows in 1..=max_tiles {
            if cols * rows <= max_tiles {
                grids.push((cols, rows));
            }
        }
    }
    grids.sort_by_key(|&(c, r)| c * r);

    let scales: Vec<f64> = grids
        .iter()
        .map(|&(c, r)| {
            let sx = (c * tile) as f64 / width as f64;
            let sy = (r * tile) as f64 / height as f64;
            sx.min(sy)
        })
        .collect();
    let pick = if scales.iter().all(|&s| s < 1.0) {
        // The first grid with the largest scale.
        let mut best = 0;
        for (i, &s) in scales.iter().enumerate() {
            if s > scales[best] {
                best = i;
            }
        }
        best
    } else {
        // Ignore grids that would shrink the picture; the first with the smallest scale.
        let mut best: Option<usize> = None;
        for (i, &s) in scales.iter().enumerate() {
            if s >= 1.0 && best.is_none_or(|b| s < scales[b]) {
                best = Some(i);
            }
        }
        best.expect("some grid has scale >= 1")
    };
    let (cols, rows) = grids[pick];
    let tiles = (cols * rows) as usize;
    let blocks = if tiles == 1 { 1 } else { tiles + 1 };

    let mut runs = Vec::with_capacity(2 + 2 * blocks);
    runs.push((Some(start), 1));
    for _ in 0..blocks {
        runs.push((None, per_tile));
        runs.push((Some(row_end.clone()), 1));
    }
    runs.push((Some(end), 1));
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> serde_json::Value {
        serde_json::json!({
            "tile": 512,
            "max_tiles": 12,
            "per_tile": 256,
            "start": "<|START_OF_IMG|>",
            "row_end": "<|IMG_LINE_BREAK|>",
            "end": "<|END_OF_IMG|>"
        })
    }

    #[test]
    fn a_command_a_picture_is_laid_out_as_its_processor_lays_it_out() {
        let reference: &[(u32, u32, usize)] = &[
            (64, 64, 259),
            (224, 224, 259),
            (256, 256, 259),
            (448, 448, 259),
            (512, 512, 259),
            (640, 480, 773),
            (800, 600, 1287),
            (1024, 768, 1287),
            (1280, 720, 1801),
            (1920, 1080, 3343),
            (2000, 200, 1287),
            (200, 2000, 1287),
            (768, 768, 1287),
            (1000, 1000, 1287),
            (20, 300, 259),
            (300, 20, 259),
            (70, 70, 259),
            (80, 2000, 1287),
            (48, 48, 259),
            (3000, 3000, 2572),
            (4000, 100, 2315),
            (100, 4000, 2315),
            (1025, 1025, 2572),
            (1540, 10, 1287),
            (513, 512, 773),
            (1600, 1200, 3343),
        ];
        for &(w, h, want) in reference {
            let runs = layout(&params(), w, h).unwrap();
            let total: usize = runs.iter().map(|r| r.1).sum();
            assert_eq!(total, want, "{w}x{h}");
        }

        // 640x480: a 2x1 grid, its two tiles and a thumbnail.
        let s = |t: &str| Some(t.to_string());
        assert_eq!(
            layout(&params(), 640, 480).unwrap(),
            vec![
                (s("<|START_OF_IMG|>"), 1),
                (None, 256),
                (s("<|IMG_LINE_BREAK|>"), 1),
                (None, 256),
                (s("<|IMG_LINE_BREAK|>"), 1),
                (None, 256),
                (s("<|IMG_LINE_BREAK|>"), 1),
                (s("<|END_OF_IMG|>"), 1),
            ]
        );
        // A picture that fits one tile has no thumbnail.
        assert_eq!(
            layout(&params(), 512, 512).unwrap(),
            vec![
                (s("<|START_OF_IMG|>"), 1),
                (None, 256),
                (s("<|IMG_LINE_BREAK|>"), 1),
                (s("<|END_OF_IMG|>"), 1),
            ]
        );
    }
}
