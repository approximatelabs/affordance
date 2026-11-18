//! HunyuanOCR picture layout.

use serde_json::Value;

/// HunyuanOCR (tencent/HunyuanOCR). Its processor (transformers' `HunYuanVLProcessor` with
/// `HunYuanVLImageProcessor`, also used by vLLM) resizes the picture with Qwen2-VL's
/// `smart_resize` on a 32-pixel cell (patch 16 times merge 2) between `min` and `max` pixels
/// (262,144 and 16,777,216 for this model; `max_pixels` overrides `size.longest_edge`), then
/// replaces the template's placeholder token with `rows * (cols + 1) + extra` copies of itself:
/// one extra position per row for the line break, and 2 more because `cat_extra_token` is on.
/// The template already writes the start and end tokens around the placeholder. The reference
/// counts came from running the processor (transformers 5.19.0) on 21 picture sizes.
pub fn layout(params: &Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let int = |key: &str, default: Option<u64>| -> Result<u64, String> {
        match params.get(key) {
            Some(v) => v
                .as_u64()
                .ok_or_else(|| format!("hunyuan: `{key}` must be a non-negative integer")),
            None => default.ok_or_else(|| format!("hunyuan: missing `{key}`")),
        }
    };
    let cell = int("cell", Some(32))?;
    let min = int("min", None)?;
    let max = int("max", None)?;
    let extra = int("extra", Some(2))?;
    if cell == 0 {
        return Err("hunyuan: `cell` must be positive".into());
    }
    if width == 0 || height == 0 {
        return Err(format!("hunyuan: picture {width}x{height} has no pixels"));
    }
    let (w, h) = (width as f64, height as f64);
    if w.max(h) / w.min(h) > 200.0 {
        return Err(format!(
            "hunyuan: picture {width}x{height} has an aspect ratio over 200"
        ));
    }
    let f = cell as f64;
    // Python round(): halves go to the even neighbour. No at-least-one-cell guard here.
    let mut hb = (h / f).round_ties_even() as u64 * cell;
    let mut wb = (w / f).round_ties_even() as u64 * cell;
    if hb * wb > max {
        let beta = ((h * w) / max as f64).sqrt();
        hb = cell.max((h / beta / f).floor() as u64 * cell);
        wb = cell.max((w / beta / f).floor() as u64 * cell);
    } else if hb * wb < min {
        let beta = (min as f64 / (h * w)).sqrt();
        hb = (h * beta / f).ceil() as u64 * cell;
        wb = (w * beta / f).ceil() as u64 * cell;
    }
    let rows = (hb / cell) as usize;
    let cols = (wb / cell) as usize;
    Ok(vec![(None, rows * (cols + 1) + extra as usize)])
}

#[cfg(test)]
mod tests {
    use super::layout;
    use serde_json::json;

    #[test]
    fn a_hunyuan_ocr_picture_is_laid_out_as_its_processor_lays_it_out() {
        let params = json!({"cell": 32, "min": 262144, "max": 16777216});
        let reference = [
            ((64, 64), 274),
            ((224, 224), 274),
            ((256, 256), 274),
            ((448, 448), 274),
            ((512, 512), 274),
            ((640, 480), 317),
            ((800, 600), 496),
            ((1024, 768), 794),
            ((1280, 720), 904),
            ((1920, 1080), 2076),
            ((2000, 200), 380),
            ((200, 2000), 436),
            ((768, 768), 602),
            ((1000, 1000), 994),
            ((3000, 2000), 5892),
            ((4000, 4000), 15752),
            ((5000, 4000), 16418),
            ((6000, 5000), 16358),
            ((3000, 20), 396),
            ((1, 1), 274),
            ((513, 511), 274),
        ];
        for ((w, h), want) in reference {
            let runs = layout(&params, w, h).unwrap();
            let total: usize = runs.iter().map(|(_, n)| n).sum();
            assert_eq!(total, want, "{w}x{h}");
        }
        // 640x480 resizes to 15 rows of 20 cells: 15 * 21 + 2, all the placeholder token.
        assert_eq!(layout(&params, 640, 480).unwrap(), vec![(None, 317)]);
        assert!(layout(&params, 2010, 10).is_err());
    }
}
