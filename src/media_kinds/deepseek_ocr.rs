/// DeepSeek-OCR-style views, used by baidu/Unlimited-OCR. SGLang's `UnlimitedOCRHFProcessor`
/// (`tokenize_with_images` in `sglang/srt/configs/unlimited_ocr.py`) replaces the template's
/// `<image>` with N copies of that same token, with no markers. The per-request
/// `images_config.image_mode` picks a preset. In the default `gundam` mode every picture gets a
/// 1024-pixel global view of (16 + 1) * 16 + 1 = 273 tokens; a picture with a side over 640
/// pixels is also cut into a grid of 2 to 32 tiles of 640 pixels (DeepSeek-OCR's
/// `dynamic_preprocess`), adding (10 * cw + 1) * 10 * ch tokens. The fixed modes take every
/// picture at one size: tiny 73, small 111, base 273, large 421. The reference counts came from
/// running SGLang v0.5.23's processor with the repo's processor and tokenizer configs.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    if width == 0 || height == 0 {
        return Err(format!("a {width}x{height} picture has no area"));
    }
    let mode = match params.get("mode") {
        None => "gundam",
        Some(v) => v
            .as_str()
            .ok_or_else(|| format!("deepseek_ocr: mode must be a string, got {v}"))?,
    };
    // (base_size, image_size, crop_mode) as SGLang's _IMAGE_MODE_PRESETS.
    let (base, image, crop) = match mode {
        "tiny" => (512, 512, false),
        "small" => (640, 640, false),
        "base" => (1024, 1024, false),
        "large" => (1280, 1280, false),
        "gundam" => (1024, 640, true),
        other => {
            return Err(format!(
                "deepseek_ocr: unknown mode {other:?} (tiny, small, base, large, gundam)"
            ));
        }
    };
    const PATCH: usize = 16;
    const DOWNSAMPLE: usize = 4;
    let queries = |size: usize| (size / PATCH).div_ceil(DOWNSAMPLE);
    let q = queries(image);
    let n = if !crop {
        (q + 1) * q + 1
    } else {
        let qb = queries(base);
        let mut n = (qb + 1) * qb + 1;
        // The 640 threshold is a constant in the processor, independent of image_size.
        if width > 640 || height > 640 {
            let (cw, ch) = closest_grid(width, height, image);
            if cw > 1 || ch > 1 {
                n += (q * cw + 1) * (q * ch);
            }
        }
        n
    };
    Ok(vec![(None, n)])
}

/// The candidate grids (columns, rows) of `dynamic_preprocess` with min 2 and max 32 tiles, in
/// the order the processor walks them: a Python set sorted (stably) by columns * rows.
const GRIDS: [(usize, usize); 118] = [
    (1, 2),
    (2, 1),
    (3, 1),
    (1, 3),
    (2, 2),
    (4, 1),
    (1, 4),
    (5, 1),
    (1, 5),
    (6, 1),
    (3, 2),
    (2, 3),
    (1, 6),
    (7, 1),
    (1, 7),
    (4, 2),
    (2, 4),
    (1, 8),
    (8, 1),
    (9, 1),
    (3, 3),
    (1, 9),
    (1, 10),
    (5, 2),
    (10, 1),
    (2, 5),
    (1, 11),
    (11, 1),
    (6, 2),
    (1, 12),
    (2, 6),
    (12, 1),
    (3, 4),
    (4, 3),
    (13, 1),
    (1, 13),
    (14, 1),
    (1, 14),
    (7, 2),
    (2, 7),
    (1, 15),
    (5, 3),
    (3, 5),
    (15, 1),
    (8, 2),
    (16, 1),
    (4, 4),
    (1, 16),
    (2, 8),
    (1, 17),
    (17, 1),
    (18, 1),
    (3, 6),
    (9, 2),
    (1, 18),
    (6, 3),
    (2, 9),
    (1, 19),
    (19, 1),
    (20, 1),
    (5, 4),
    (2, 10),
    (1, 20),
    (4, 5),
    (10, 2),
    (7, 3),
    (1, 21),
    (3, 7),
    (21, 1),
    (2, 11),
    (11, 2),
    (22, 1),
    (1, 22),
    (1, 23),
    (23, 1),
    (1, 24),
    (24, 1),
    (6, 4),
    (3, 8),
    (4, 6),
    (2, 12),
    (8, 3),
    (12, 2),
    (5, 5),
    (1, 25),
    (25, 1),
    (1, 26),
    (2, 13),
    (13, 2),
    (26, 1),
    (9, 3),
    (27, 1),
    (3, 9),
    (1, 27),
    (1, 28),
    (7, 4),
    (14, 2),
    (2, 14),
    (4, 7),
    (28, 1),
    (29, 1),
    (1, 29),
    (3, 10),
    (15, 2),
    (1, 30),
    (2, 15),
    (10, 3),
    (5, 6),
    (30, 1),
    (6, 5),
    (1, 31),
    (31, 1),
    (8, 4),
    (1, 32),
    (4, 8),
    (32, 1),
    (16, 2),
    (2, 16),
];

/// `find_closest_aspect_ratio`: the grid whose columns/rows is nearest W/H in float64; on an
/// exact tie a later grid wins when the picture's area is over half the grid's tiled area.
fn closest_grid(width: u32, height: u32, tile: usize) -> (usize, usize) {
    let aspect = f64::from(width) / f64::from(height);
    let area = f64::from(width) * f64::from(height);
    let mut best_diff = f64::INFINITY;
    let mut best = (1, 1);
    for &(i, j) in &GRIDS {
        let diff = (aspect - i as f64 / j as f64).abs();
        if diff < best_diff {
            best_diff = diff;
            best = (i, j);
        } else if diff == best_diff && area > 0.5 * tile as f64 * tile as f64 * (i * j) as f64 {
            best = (i, j);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn total(params: &serde_json::Value, w: u32, h: u32) -> usize {
        layout(params, w, h).unwrap().iter().map(|r| r.1).sum()
    }

    const GUNDAM: [(u32, u32, usize, (usize, usize)); 19] = [
        (64, 64, 273, (1, 1)),
        (224, 224, 273, (1, 1)),
        (256, 256, 273, (1, 1)),
        (448, 448, 273, (1, 1)),
        (512, 512, 273, (1, 1)),
        (640, 480, 273, (1, 1)),
        (800, 600, 1503, (4, 3)),
        (1024, 768, 1503, (4, 3)),
        (1280, 720, 3113, (7, 4)),
        (1920, 1080, 3113, (7, 4)),
        (2000, 200, 1283, (10, 1)),
        (200, 2000, 1373, (1, 10)),
        (768, 768, 693, (2, 2)),
        (1000, 1000, 693, (2, 2)),
        (640, 640, 273, (1, 1)),
        (641, 640, 693, (2, 2)),
        (3000, 2000, 2713, (6, 4)),
        (4000, 4000, 2823, (5, 5)),
        (3000, 20, 3483, (32, 1)),
    ];

    #[test]
    fn an_unlimited_ocr_picture_is_laid_out_as_its_processor_lays_it_out() {
        for (w, h, count, grid) in GUNDAM {
            assert_eq!(total(&json!({}), w, h), count, "gundam {w}x{h}");
            assert_eq!(total(&json!({"mode": "gundam"}), w, h), count, "gundam {w}x{h}");
            if w > 640 || h > 640 {
                assert_eq!(closest_grid(w, h, 640), grid, "grid {w}x{h}");
            }
            for (mode, fixed) in [("tiny", 73), ("small", 111), ("base", 273), ("large", 421)] {
                assert_eq!(total(&json!({"mode": mode}), w, h), fixed, "{mode} {w}x{h}");
            }
        }
        // A tiled picture is one run of the placeholder: the global view, then the 4x3 tiles.
        assert_eq!(layout(&json!({}), 800, 600).unwrap(), vec![(None, 1503)]);
    }

    #[test]
    fn an_empty_picture_or_an_unknown_mode_is_refused() {
        assert!(layout(&json!({}), 0, 10).is_err());
        assert!(layout(&json!({"mode": "huge"}), 10, 10).is_err());
    }
}
