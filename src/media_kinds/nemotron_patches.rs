/// Nemotron-3-Nano-Omni-30B-A3B-Reasoning and Nemotron-3.5-Super-VL-120B-A12B: the official
/// processor (the repository's `_compute_target_patches`, a port of vLLM's
/// `DynamicResolutionImageTiler`) resizes the picture to one grid of 16 px patches, kept
/// between 1024 and 13312 patches, and every 2x2 block of patches becomes one `<image>` token,
/// so a picture costs 256 to 3328 tokens. The template's `<image>` is replaced by `<img>`, the
/// tokens, and `</img>`. Reference: the official processors run in Docker (transformers 5.19.0),
/// vLLM v0.33.0 on 7464 sizes, and OpenRouter (Super equal, Omni one more for the template's
/// newline). SGLang v0.5.23 uses different arithmetic and is not followed here.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let int = |key: &str, default: u64| -> Result<u64, String> {
        match params.get(key) {
            None => Ok(default),
            Some(v) => v
                .as_u64()
                .filter(|&n| n > 0)
                .ok_or_else(|| format!("nemotron_patches: {key} must be a positive integer")),
        }
    };
    let text = |key: &str, default: &str| -> Result<String, String> {
        match params.get(key) {
            None => Ok(default.to_string()),
            Some(v) => v
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("nemotron_patches: {key} must be a string")),
        }
    };
    let patch = int("patch", 16)? as f64;
    let merge = int("merge", 2)?;
    let min_patches = int("min_patches", 1024)?;
    let max_patches = int("max_patches", 13312)?;
    let start = text("start", "<img>")?;
    let end = text("end", "</img>")?;
    if width == 0 || height == 0 {
        return Err(format!("nemotron_patches: a {width}x{height} picture is empty"));
    }

    // Step 1: Python's round(H/16 + 0.5), halves to even.
    let ph = (height as f64 / patch + 0.5).round_ties_even() as u64;
    let pw = (width as f64 / patch + 0.5).round_ties_even() as u64;
    // Step 2: scale down into the budget.
    let avail = max_patches;
    let f = ((avail as f64) / ((ph * pw) as f64)).sqrt().min(1.0);
    let mut th = (f * ph as f64).floor() as u64;
    let mut tw = (f * pw as f64).floor() as u64;
    // Step 3: scale up to the minimum.
    if avail > min_patches && th * tw < min_patches {
        if th * tw == 0 {
            // The processor divides by zero here (ZeroDivisionError).
            return Err(format!(
                "nemotron_patches: a {width}x{height} picture scales to zero patches"
            ));
        }
        let u = (min_patches as f64 / (th * tw) as f64).sqrt();
        th = (u * th as f64).ceil() as u64;
        tw = (u * tw as f64).ceil() as u64;
    }
    // Step 4: make height, then width, even.
    if th % 2 == 1 {
        if (th + 1) * tw <= avail {
            th += 1;
        } else {
            th = (th - 1).max(2);
        }
    }
    if tw % 2 == 1 {
        if th * (tw + 1) <= avail {
            tw += 1;
        } else {
            tw = (tw - 1).max(2);
        }
    }
    let tokens = (tw * th / (merge * merge)) as usize;
    Ok(vec![(Some(start), 1), (None, tokens), (Some(end), 1)])
}

#[cfg(test)]
mod tests {
    use super::layout;

    fn params() -> serde_json::Value {
        serde_json::json!({
            "patch": 16, "merge": 2, "min_patches": 1024, "max_patches": 13312,
            "start": "<img>", "end": "</img>"
        })
    }

    fn total(w: u32, h: u32) -> usize {
        layout(&params(), w, h).unwrap().iter().map(|r| r.1).sum()
    }

    // From <img> to </img> inclusive; the official processors of both models agree.
    const REFERENCE: &[(u32, u32, usize)] = &[
        (64, 64, 258),
        (224, 224, 258),
        (256, 256, 258),
        (448, 448, 258),
        (512, 512, 258),
        (640, 480, 302),
        (800, 600, 477),
        (1024, 768, 770),
        (1280, 720, 922),
        (1920, 1080, 2042),
        (2000, 200, 443),
        (200, 2000, 443),
        (768, 768, 578),
        (1000, 1000, 1026),
        (100, 37, 277),
        (513, 513, 291),
        (1023, 1023, 1026),
        (3000, 3000, 3308),
        (4000, 500, 2002),
        (17, 900, 260),
        (600, 800, 477),
        (1600, 900, 1452),
        (33, 33, 258),
        (4096, 4096, 3308),
        (1500, 1499, 2211),
    ];

    // OpenRouter prompt tokens with the picture minus without it, for the first 14 sizes.
    const OPENROUTER_SUPER: &[usize] = &[258, 258, 258, 258, 258, 302, 477, 770, 922, 2042, 443, 443, 578, 1026];
    const OPENROUTER_OMNI: &[usize] = &[259, 259, 259, 259, 259, 303, 478, 771, 923, 2043, 444, 444, 579, 1027];

    #[test]
    fn a_nemotron_omni_or_super_picture_is_laid_out_as_its_processor_lays_it_out() {
        for &(w, h, want) in REFERENCE {
            assert_eq!(total(w, h), want, "{w}x{h}");
        }
        assert_eq!(
            layout(&params(), 1280, 720).unwrap(),
            vec![
                (Some("<img>".to_string()), 1),
                (None, 920),
                (Some("</img>".to_string()), 1)
            ]
        );
    }

    #[test]
    fn openrouter_counts_super_exactly_and_omni_one_more_for_the_templates_newline() {
        for (i, &(w, h, _)) in REFERENCE.iter().take(14).enumerate() {
            assert_eq!(total(w, h), OPENROUTER_SUPER[i], "super {w}x{h}");
            assert_eq!(total(w, h) + 1, OPENROUTER_OMNI[i], "omni {w}x{h}");
        }
    }

    #[test]
    fn a_picture_the_processor_cannot_scale_is_refused() {
        assert!(layout(&params(), 0, 10).is_err());
        assert!(layout(&params(), 1, 1_000_000).is_err());
    }
}
