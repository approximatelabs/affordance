/// Picture tokens for stepfun-ai step3, Step3-VL-10B and Step-3.7-Flash. Each repository's
/// `processing_step3.py` (`Step3VLProcessor` with `ImagePatcher`, also run line for line by
/// SGLang's `step3_vl.py`) pads a tiny strip to a square, caps the long side at 3024 px with
/// `int()`, picks a window size (504 px, the short side for long thin pictures, or none for a
/// small near-square picture), and cuts the picture into square windows row by row. Each window
/// is `<patch_start>` + 81 `<im_patch>` + `<patch_end>`, with one `<patch_newline>` after every
/// row but the last; a global view `<im_start>` + 169 `<im_patch>` + `<im_end>` always comes
/// last. The windows per side round up only when `side / window - side // window > 0.2` in
/// float64, so 420x100 makes 5 windows. The reference counts come from the processor run in
/// Docker for all three models, the real Step3-VL-10B server and OpenRouter's StepFun provider.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let num = |key: &str, default: f64| -> Result<f64, String> {
        match params.get(key) {
            None => Ok(default),
            Some(v) => v
                .as_f64()
                .ok_or_else(|| format!("step3: parameter {key} must be a number")),
        }
    };
    let count = |key: &str, default: u64| -> Result<u64, String> {
        match params.get(key) {
            None => Ok(default),
            Some(v) => v
                .as_u64()
                .ok_or_else(|| format!("step3: parameter {key} must be a whole number")),
        }
    };
    let name = |key: &str| -> Result<String, String> {
        params
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| format!("step3: parameter {key} (a token name) is required"))
    };

    let global_tokens = count("global_tokens", 169)? as usize;
    let window_tokens = count("window_tokens", 81)? as usize;
    let max_side = count("max_side", 3024)?;
    let small_long = count("small_long", 728)?;
    let big_window = count("big_window", 504)?;
    let tiny_side = count("tiny_side", 32)?;
    let tiny_ratio = num("tiny_ratio", 4.0)?;
    let split_ratio_small = num("split_ratio_small", 1.5)?;
    let split_ratio_big = num("split_ratio_big", 4.0)?;
    let round_up_over = num("round_up_over", 0.2)?;
    let im_start = name("im_start")?;
    let im_end = name("im_end")?;
    let patch_start = name("patch_start")?;
    let patch_end = name("patch_end")?;
    let patch_newline = name("patch_newline")?;

    if width == 0 || height == 0 {
        return Err(format!("step3: a {width}x{height} picture has no pixels"));
    }
    let (mut w, mut h) = (u64::from(width), u64::from(height));

    // (1) get_image_size_for_padding: a tiny strip is padded to a square.
    let ratio = w as f64 / h as f64;
    if w.min(h) < tiny_side && (ratio > tiny_ratio || ratio < 1.0 / tiny_ratio) {
        let side = w.max(h);
        w = side;
        h = side;
    }
    // (2) get_image_size_for_preprocess: cap the long side, int() truncation.
    if w.max(h) > max_side {
        let scale = max_side as f64 / w.max(h) as f64;
        w = (w as f64 * scale) as u64;
        h = (h as f64 * scale) as u64;
    }
    // (3) determine_window_size.
    let (long, short) = (w.max(h), w.min(h));
    if short == 0 {
        return Err(format!(
            "step3: a {width}x{height} picture scales to {w}x{h}, which the processor cannot divide"
        ));
    }
    let window = if long <= small_long {
        if long as f64 / short as f64 > split_ratio_small {
            short
        } else {
            0
        }
    } else if long as f64 / short as f64 > split_ratio_big {
        short.min(big_window)
    } else {
        big_window
    };

    let mut runs: Vec<(Option<String>, usize)> = Vec::new();
    if window > 0 {
        // (4) get_image_size_for_crop, then slide_window with step = size.
        let per_side = |side: u64| -> u64 {
            let r = side as f64 / window as f64;
            if r < 1.0 {
                1
            } else {
                let k = side / window;
                if r - k as f64 > round_up_over { k + 1 } else { k }
            }
        };
        let (cols, rows) = (per_side(w), per_side(h));
        for row in 0..rows {
            for _ in 0..cols {
                runs.push((Some(patch_start.clone()), 1));
                runs.push((None, window_tokens));
                runs.push((Some(patch_end.clone()), 1));
            }
            if row + 1 < rows {
                runs.push((Some(patch_newline.clone()), 1));
            }
        }
    }
    // (5) the global view always comes last.
    runs.push((Some(im_start), 1));
    runs.push((None, global_tokens));
    runs.push((Some(im_end), 1));
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::layout;

    fn params() -> serde_json::Value {
        serde_json::json!({
            "global_tokens": 169, "window_tokens": 81, "max_side": 3024, "small_long": 728,
            "big_window": 504, "tiny_side": 32, "tiny_ratio": 4, "split_ratio_small": 1.5,
            "split_ratio_big": 4, "round_up_over": 0.2,
            "im_start": "<im_start>", "im_end": "<im_end>", "patch_start": "<patch_start>",
            "patch_end": "<patch_end>", "patch_newline": "<patch_newline>"
        })
    }

    const REFERENCE: [(u32, u32, usize); 35] = [
        (64, 64, 171),
        (224, 224, 171),
        (256, 256, 171),
        (448, 448, 171),
        (512, 512, 171),
        (640, 480, 171),
        (800, 600, 337),
        (1024, 768, 504),
        (1280, 720, 670),
        (1920, 1080, 836),
        (2000, 200, 1001),
        (200, 2000, 1010),
        (768, 768, 504),
        (1000, 1000, 504),
        (420, 100, 586),
        (100, 420, 590),
        (600, 500, 171),
        (700, 400, 337),
        (729, 486, 337),
        (730, 486, 337),
        (3200, 2400, 2665),
        (4000, 100, 3574),
        (20, 200, 171),
        (200, 20, 171),
        (30, 500, 171),
        (1108, 504, 337),
        (1109, 504, 420),
        (605, 504, 171),
        (3025, 3025, 3164),
        (6048, 1000, 669),
        (1, 1, 171),
        (10, 1000, 504),
        (1500, 300, 586),
        (2000, 400, 586),
        (2016, 504, 503),
    ];

    #[test]
    fn a_step3_picture_is_laid_out_as_its_processor_lays_it_out() {
        let p = params();
        for (w, h, expected) in REFERENCE {
            let runs = layout(&p, w, h).unwrap();
            let total: usize = runs.iter().map(|(_, n)| n).sum();
            assert_eq!(total, expected, "{w}x{h}");
        }

        // 1024x768 from the processor's input_ids: two rows of two windows, then the global view.
        let s = |t: &str| (Some(t.to_string()), 1);
        let window = [s("<patch_start>"), (None, 81), s("<patch_end>")];
        let mut expected = Vec::new();
        expected.extend(window.clone());
        expected.extend(window.clone());
        expected.push(s("<patch_newline>"));
        expected.extend(window.clone());
        expected.extend(window);
        expected.extend([s("<im_start>"), (None, 169), s("<im_end>")]);
        assert_eq!(layout(&p, 1024, 768).unwrap(), expected);

        // 420x100: the 0.2 comparison lands just above 0.2 in float64, so 5 windows in one row.
        let runs = layout(&p, 420, 100).unwrap();
        let windows = runs
            .iter()
            .filter(|(t, _)| t.as_deref() == Some("<patch_start>"))
            .count();
        assert_eq!(windows, 5);
        assert!(!runs.iter().any(|(t, _)| t.as_deref() == Some("<patch_newline>")));

        assert!(layout(&p, 0, 10).is_err());
    }
}
