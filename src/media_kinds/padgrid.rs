//! The `padgrid` picture layout.

/// Lays out a picture for Tencent's Youtu-Parsing-Omni (`tencent__Youtu-Parsing-Omni`), as its
/// `YoutuVITAImageProcessor` does in `add_image_input_discrete_or_contiguous` and
/// `process_native_v2`. The processor resizes the picture only when its area is outside
/// `[min, max]` pixels, then pads each side up to a multiple of `cell` pixels (rounding down
/// instead when the padded canvas would exceed `max`). The template's `<|image|>` is replaced by
/// `<|vision_start|>`, the canvas size written as text (height, `*`, width, in pixels), a newline,
/// then one row of `<|image_pad|>` per `cell` pixels of height, each row `width / cell` pads long
/// and followed by a newline, then `<|vision_end|>`. The size text and the newlines are plain text,
/// not special tokens: they are returned one character per run as `(Some("text:<char>"), 1)`, and
/// the caller must tokenize each character on its own (this model's tokenizer gives one token per
/// character, and `"\n"` is id 198). The reference counts come from running the model repository's
/// processor code in `lmsysorg/sglang:v0.5.23` with transformers 5.19 on 17 picture sizes.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let int = |key: &str, default: u64| -> Result<u64, String> {
        match params.get(key) {
            None => Ok(default),
            Some(v) => v
                .as_u64()
                .ok_or_else(|| format!("padgrid: `{key}` must be a non-negative integer")),
        }
    };
    let name = |key: &str, default: &str| -> Result<String, String> {
        match params.get(key) {
            None => Ok(default.to_string()),
            Some(v) => v
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("padgrid: `{key}` must be a string")),
        }
    };
    let cell = int("cell", 32)?;
    let min = int("min", 1024)?;
    let max = int("max", 33_554_432)?;
    let start = name("start", "<|vision_start|>")?;
    let pad = name("pad", "<|image_pad|>")?;
    let end = name("end", "<|vision_end|>")?;
    if cell == 0 {
        return Err("padgrid: `cell` must be positive".into());
    }
    if width == 0 || height == 0 {
        return Err(format!("padgrid: a {width}x{height} picture has no pixels"));
    }

    // (1) Resize only when the area is out of bounds, in float64 as the processor does.
    let (w0, h0) = (u64::from(width), u64::from(height));
    let area = w0 * h0;
    let (w, h) = if area > max {
        let beta = (area as f64 / max as f64).sqrt();
        (
            ((w0 as f64 / beta).floor() as u64).max(1),
            ((h0 as f64 / beta).floor() as u64).max(1),
        )
    } else if area < min {
        let beta = (min as f64 / area as f64).sqrt();
        (
            ((w0 as f64 * beta).ceil() as u64).max(1),
            ((h0 as f64 * beta).ceil() as u64).max(1),
        )
    } else {
        (w0, h0)
    };

    // (2) Pad up to whole cells; round down instead if the canvas would exceed `max`.
    let (mut pw, mut ph) = (w.div_ceil(cell) * cell, h.div_ceil(cell) * cell);
    if pw * ph > max {
        pw = (w / cell * cell).max(cell);
        ph = (h / cell * cell).max(cell);
    }
    let (rows, cols) = ((ph / cell) as usize, (pw / cell) as usize);

    let text =
        |s: &str| -> Vec<(Option<String>, usize)> { s.chars().map(|c| (Some(format!("text:{c}")), 1)).collect() };
    let mut runs = Vec::with_capacity(2 * rows + 16);
    runs.push((Some(start), 1));
    runs.extend(text(&format!("{ph}*{pw}\n")));
    for _ in 0..rows {
        runs.push((Some(pad.clone()), cols));
        runs.extend(text("\n"));
    }
    runs.push((Some(end), 1));
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::layout;

    fn youtu() -> serde_json::Value {
        serde_json::json!({
            "cell": 32, "min": 1024, "max": 33554432,
            "start": "<|vision_start|>", "pad": "<|image_pad|>", "end": "<|vision_end|>"
        })
    }

    #[test]
    fn a_youtu_parsing_omni_picture_is_laid_out_as_its_processor_lays_it_out() {
        // (width, height, <|image_pad|> tokens, whole run where measured)
        let reference: [(u32, u32, usize, Option<usize>); 17] = [
            (64, 64, 4, Some(14)),
            (224, 224, 49, None),
            (256, 256, 64, None),
            (448, 448, 196, None),
            (512, 512, 256, None),
            (640, 480, 300, None),
            (800, 600, 475, None),
            (1024, 768, 768, None),
            (1280, 720, 920, Some(954)),
            (1920, 1080, 2040, None),
            (2000, 200, 441, None),
            (200, 2000, 441, Some(515)),
            (768, 768, 576, None),
            (1000, 1000, 1024, None),
            (4000, 3000, 11750, None),
            (3000, 200, 658, None),
            (7000, 6000, 32565, Some(32744)),
        ];
        for (w, h, pads, whole) in reference {
            let runs = layout(&youtu(), w, h).unwrap();
            let got_pads: usize = runs
                .iter()
                .filter(|(t, _)| t.as_deref() == Some("<|image_pad|>"))
                .map(|(_, n)| n)
                .sum();
            assert_eq!(got_pads, pads, "pads for {w}x{h}");
            if let Some(whole) = whole {
                let total: usize = runs.iter().map(|(_, n)| n).sum();
                assert_eq!(total, whole, "whole run for {w}x{h}");
            }
        }

        let s = |t: &str| (Some(t.to_string()), 1);
        let pads = |n| (Some("<|image_pad|>".to_string()), n);
        assert_eq!(
            layout(&youtu(), 64, 64).unwrap(),
            vec![
                s("<|vision_start|>"),
                s("text:6"),
                s("text:4"),
                s("text:*"),
                s("text:6"),
                s("text:4"),
                s("text:\n"),
                pads(2),
                s("text:\n"),
                pads(2),
                s("text:\n"),
                s("<|vision_end|>"),
            ]
        );
        let wide = layout(&youtu(), 1280, 720).unwrap();
        let head: Vec<_> = wide[1..10].iter().map(|r| r.0.clone().unwrap()).collect();
        assert_eq!(head.concat(), "text:7text:3text:6text:*text:1text:2text:8text:0text:\n");
        assert_eq!(wide.iter().filter(|r| r.1 == 40).count(), 23);
    }

    #[test]
    fn a_picture_with_no_pixels_is_refused() {
        assert!(layout(&youtu(), 0, 10).is_err());
    }
}
