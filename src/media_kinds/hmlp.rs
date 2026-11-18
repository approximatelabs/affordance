/// Inkling and Inkling-Small (Thinking Machines). The server first enlarges the picture: with
/// L the long edge, the target long edge is min(L * frac, max(cap, L)), so a small picture
/// doubles, a long edge stops at `cap` pixels, and a picture already longer than `cap` is left
/// as is; when the scale r = target / L is not 1, each side becomes max(1, floor(side * r + 0.5))
/// in 64-bit floats (halves round up). The hMLP vision encoder then emits one token per
/// `patch` by `patch` square: ceil(H' / patch) rows of floor(W' / patch) + 1 columns, so every
/// row has one extra column even when the width is a multiple of the patch. The template writes
/// `<|content_image|><|unused_200054|><|end_message|>` and the processor replaces the one
/// placeholder with N copies of itself, adding no other markers. Reference: SGLang v0.5.23's
/// `InklingImageProcessor` (rescale_image_frac 2.0, cap 2048, patch_size 40) and vLLM nightly's
/// `_rescale_image` with transformers' `InklingImageProcessor`, run in docker on 22 sizes; the
/// OpenRouter counts (DeepInfra and Together) are these plus a constant per provider.
/// Parameters: `{"patch": 40, "frac": 2.0, "cap": 2048}`; `frac` may be null for no enlargement.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let patch = params
        .get("patch")
        .and_then(|v| v.as_u64())
        .filter(|&p| p > 0)
        .ok_or("hmlp: params need a positive integer \"patch\"")?;
    let frac = match params.get("frac") {
        None => return Err("hmlp: params need \"frac\" (a number, or null)".into()),
        Some(serde_json::Value::Null) => None,
        Some(v) => Some(v.as_f64().ok_or("hmlp: \"frac\" must be a number or null")?),
    };
    if width == 0 || height == 0 {
        return Err(format!("hmlp: a {width}x{height} picture has no pixels"));
    }
    let (mut w, mut h) = (u64::from(width), u64::from(height));
    if let Some(frac) = frac {
        let cap = params
            .get("cap")
            .and_then(|v| v.as_u64())
            .ok_or("hmlp: params need an integer \"cap\" when \"frac\" is set")?;
        let long = w.max(h);
        let target = (long as f64 * frac).min(cap.max(long) as f64);
        let r = target / long as f64;
        if r != 1.0 {
            let scale = |side: u64| ((side as f64 * r + 0.5).floor() as u64).max(1);
            (w, h) = (scale(w), scale(h));
        }
    }
    let rows = h.div_ceil(patch);
    let cols = w / patch + 1;
    Ok(vec![(None, (rows * cols) as usize)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(w: u32, h: u32) -> usize {
        let params = serde_json::json!({"patch": 40, "frac": 2.0, "cap": 2048});
        layout(&params, w, h).unwrap().iter().map(|(_, n)| n).sum()
    }

    fn size(s: &str) -> (u32, u32) {
        let (w, h) = s.split_once('x').unwrap();
        (w.parse().unwrap(), h.parse().unwrap())
    }

    #[test]
    fn an_inkling_picture_is_laid_out_as_its_processor_lays_it_out() {
        // SGLang v0.5.23 InklingImageProcessor and vLLM nightly, identical on every size.
        let processor = [
            ("64x64", 16),
            ("224x224", 144),
            ("256x256", 169),
            ("448x448", 529),
            ("512x512", 676),
            ("640x480", 792),
            ("800x600", 1230),
            ("1024x768", 2028),
            ("1280x720", 1508),
            ("1920x1080", 1508),
            ("2000x200", 312),
            ("200x2000", 312),
            ("768x768", 1521),
            ("1000x1000", 2550),
            ("40x40", 6),
            ("80x40", 10),
            ("39x41", 6),
            ("1023x1", 52),
            ("2048x2048", 2704),
            ("3000x1000", 1900),
            ("1366x768", 1508),
            ("1025x1025", 2704),
        ];
        for (s, n) in processor {
            let (w, h) = size(s);
            assert_eq!(total(w, h), n, "{s}");
        }
    }

    #[test]
    fn openrouter_counts_are_the_processor_count_plus_a_constant() {
        // Prompt tokens OpenRouter added per picture: Inkling on DeepInfra adds 3 more,
        // Inkling-Small on Together adds 2 more.
        let sizes = [
            "64x64",
            "224x224",
            "256x256",
            "448x448",
            "512x512",
            "640x480",
            "800x600",
            "1024x768",
            "1280x720",
            "1920x1080",
            "2000x200",
            "200x2000",
            "768x768",
            "1000x1000",
            "40x40",
            "39x41",
            "2048x2048",
            "3000x1000",
        ];
        let inkling_deepinfra = [
            19, 147, 172, 532, 679, 795, 1233, 2031, 1511, 1511, 315, 315, 1524, 2553, 9, 9, 2707, 1903,
        ];
        let inkling_small_together = [
            18, 146, 171, 531, 678, 794, 1232, 2030, 1510, 1510, 314, 314, 1523, 2552, 8, 8, 2706, 1902,
        ];
        for (i, s) in sizes.iter().enumerate() {
            let (w, h) = size(s);
            assert_eq!(total(w, h) + 3, inkling_deepinfra[i], "inkling {s}");
            assert_eq!(total(w, h) + 2, inkling_small_together[i], "inkling-small {s}");
        }
    }

    #[test]
    fn a_picture_is_one_run_of_the_placeholder_with_no_markers() {
        let params = serde_json::json!({"patch": 40, "frac": 2.0, "cap": 2048});
        // 1920x1080 -> 2048x1152: 29 rows of 52 columns.
        assert_eq!(layout(&params, 1920, 1080).unwrap(), vec![(None, 1508)]);
        assert!(layout(&params, 0, 10).is_err());
    }

    #[test]
    fn the_enlargement_rounds_halves_up() {
        // 3x5 at frac 2.5 gives 7.5 x 12.5, which round up to 8 x 13 (Python round would give
        // 8 x 12): 13 rows of 9 columns at patch 1.
        let params = serde_json::json!({"patch": 1, "frac": 2.5, "cap": 2048});
        assert_eq!(layout(&params, 3, 5).unwrap(), vec![(None, 13 * 9)]);
    }
}
