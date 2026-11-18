//! The `aligned` picture rule: sides rounded up to whole cells, with a token floor and a cap.

/// GLM-5.3-Flash (`zai-org/GLM-5.3-Flash`). Its processor (`Glm5NextImageProcessor`,
/// transformers 5.19, `smart_resize`; SGLang passes pictures through it unchanged) rounds each
/// side up to a multiple of `cell` (28) pixels and pads with black; one cell is one placeholder
/// token. Below `min_tokens` (16) cells it scales the picture up to that area
/// (`s = sqrt(min * 2 * cell^2 / (2 * H * W))`, sides `ceil(side * s)`, then rounded up again).
/// Above `max_tokens` (8000) cells it binary-searches the largest content height `c` in
/// `1..=H` whose rounded-up sides, with width `max(1, floor(W * c / H))`, fit the cap. The
/// template already writes `<|begin_of_image|>` and `<|end_of_image|>` around its one
/// `<|image|>`, so the result is N copies of the placeholder. Reference: the processor run in
/// Docker (lmsysorg/sglang:v0.5.23) on 24 sizes, and OpenRouter (Modal) at N + 2 on 14 sizes.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let get = |key: &str| -> Result<u64, String> {
        params
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("aligned: missing integer parameter `{key}`"))
    };
    let cell = get("cell")?;
    let min_tokens = get("min_tokens")?;
    let max_tokens = get("max_tokens")?;
    if cell == 0 {
        return Err("aligned: `cell` must be positive".into());
    }
    if width == 0 || height == 0 {
        return Err(format!("aligned: a {width}x{height} picture has no pixels"));
    }
    let (w0, h0) = (u128::from(width), u128::from(height));
    let cell = u128::from(cell);
    // The processor treats a picture as two identical frames (temporal factor 2) and counts
    // pixels: budget = 2 * h * w against tokens * 2 * cell^2.
    let frames: u128 = 2;
    let per_token = frames * cell * cell;
    let min_pixels = u128::from(min_tokens) * per_token;
    let max_pixels = u128::from(max_tokens) * per_token;
    let align = |v: u128| v.div_ceil(cell) * cell;

    let mut h = align(h0);
    let mut w = align(w0);
    let mut budget = frames * h * w;
    if budget < min_pixels {
        // math.sqrt(min_pixels / (num_frames * height * width)), float64 throughout.
        let scale = (min_pixels as f64 / (frames * h0 * w0) as f64).sqrt();
        h = align(((h0 as f64 * scale).ceil() as u128).max(1));
        w = align(((w0 as f64 * scale).ceil() as u128).max(1));
        budget = frames * h * w;
    }
    if budget > max_pixels {
        if max_pixels < frames * cell * cell {
            return Err(format!("aligned: max_tokens {max_tokens} is below one cell"));
        }
        let (mut lo, mut hi) = (1u128, h0);
        let (mut best_h, mut best_w) = (cell, cell);
        while lo <= hi {
            let c = (lo + hi) / 2;
            // max(1, math.floor(width * content_height / height)): true division in float64.
            let cw = (((w0 * c) as f64 / h0 as f64).floor() as u128).max(1);
            let (ch, cwa) = (align(c), align(cw));
            if frames * ch * cwa <= max_pixels {
                (best_h, best_w) = (ch, cwa);
                lo = c + 1;
            } else {
                hi = c - 1;
            }
        }
        (h, w) = (best_h, best_w);
    }
    let tokens = (h / cell) * (w / cell);
    Ok(vec![(None, tokens as usize)])
}

#[cfg(test)]
mod tests {
    use super::layout;

    fn glm53() -> serde_json::Value {
        serde_json::json!({"cell": 28, "min_tokens": 16, "max_tokens": 8000})
    }

    fn count(width: u32, height: u32) -> usize {
        layout(&glm53(), width, height).unwrap().iter().map(|(_, n)| n).sum()
    }

    #[test]
    fn a_glm53_flash_picture_is_laid_out_as_its_processor_lays_it_out() {
        let processor: [(u32, u32, usize); 24] = [
            (64, 64, 16),
            (224, 224, 64),
            (256, 256, 100),
            (448, 448, 256),
            (512, 512, 361),
            (640, 480, 414),
            (800, 600, 638),
            (1024, 768, 1036),
            (1280, 720, 1196),
            (1920, 1080, 2691),
            (2000, 200, 576),
            (200, 2000, 576),
            (768, 768, 784),
            (1000, 1000, 1296),
            (4000, 3000, 7931),
            (8000, 8000, 7921),
            (3000, 3000, 7921),
            (2500, 2501, 7921),
            (10000, 100, 1432),
            (100, 10000, 1432),
            (30, 30, 16),
            (10, 10, 16),
            (1, 1, 16),
            (5000, 1, 179),
        ];
        for (w, h, n) in processor {
            assert_eq!(count(w, h), n, "{w}x{h}");
        }
        // OpenRouter (Modal) counts include the template's two image markers.
        let openrouter: [(u32, u32, usize); 14] = [
            (64, 64, 18),
            (224, 224, 66),
            (256, 256, 102),
            (448, 448, 258),
            (512, 512, 363),
            (640, 480, 416),
            (800, 600, 640),
            (1024, 768, 1038),
            (1280, 720, 1198),
            (1920, 1080, 2693),
            (2000, 200, 578),
            (200, 2000, 578),
            (768, 768, 786),
            (1000, 1000, 1298),
        ];
        for (w, h, n) in openrouter {
            assert_eq!(count(w, h) + 2, n, "{w}x{h}");
        }
        // The whole picture is one run of the placeholder: 4000x3000 is 103 x 77 cells.
        assert_eq!(layout(&glm53(), 4000, 3000).unwrap(), vec![(None, 7931)]);
        assert!(layout(&glm53(), 0, 10).is_err());
    }
}
