/// LLaVA-OneVision style "anyres" layout, as used by YannQi__R-4B (`RProcessor` and
/// `RImageProcessor` from the model repository). The processor picks one tile layout from a list
/// of pinpoints with transformers' `select_best_resolution`, turns it into a feature grid of
/// `side` features per `tile` pixels, removes the padding rows or columns with Python's
/// `int(round(x, 7))`, and writes `side * side` features for the whole picture shrunk to one tile,
/// then the grid with one newline feature after each row. Every feature is the template's
/// placeholder token (`<image>`), so the result is a single run. This models a prompt with one
/// picture; when one prompt holds several pictures, the processor makes each of them
/// `side * side + 1` tokens (730 for R-4B) whatever its size, which this function does not model.
/// Reference counts came from the processor itself (transformers 5.19, `trust_remote_code`),
/// whose own `_get_num_multimodal_tokens` agrees.
///
/// Params: `tile` (pixels, default 384), `side` (features per tile edge, default 27),
/// `pinpoints` (list of `[height, width]` in pixels, required).
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    if width == 0 || height == 0 {
        return Err(format!("anyres: picture {width}x{height} has no pixels"));
    }
    let uint = |key: &str, default: u64| -> Result<u64, String> {
        match params.get(key) {
            None => Ok(default),
            Some(v) => v
                .as_u64()
                .filter(|&n| n > 0)
                .ok_or_else(|| format!("anyres: `{key}` must be a positive integer")),
        }
    };
    let tile = uint("tile", 384)?;
    let side = uint("side", 27)?;
    let pinpoints = params
        .get("pinpoints")
        .and_then(|v| v.as_array())
        .ok_or("anyres: `pinpoints` must be a list of [height, width]")?;
    let mut points = Vec::with_capacity(pinpoints.len());
    for p in pinpoints {
        let pair = p.as_array().filter(|a| a.len() == 2);
        let hw = pair.and_then(|a| Some((a[0].as_u64()?, a[1].as_u64()?)));
        match hw {
            Some((h, w)) if h > 0 && w > 0 => points.push((h, w)),
            _ => return Err("anyres: each pinpoint must be [height, width]".into()),
        }
    }
    if points.is_empty() {
        return Err("anyres: `pinpoints` is empty".into());
    }

    // transformers' select_best_resolution, with original_size = (height, width).
    let (ow, oh) = (width as f64, height as f64);
    let original = width as u64 * height as u64;
    let mut best = points[0];
    let mut max_effective = 0u64;
    let mut min_wasted = u64::MAX;
    let mut first = true;
    for &(h, w) in &points {
        let scale = (w as f64 / ow).min(h as f64 / oh);
        let dw = (ow * scale) as u64;
        let dh = (oh * scale) as u64;
        let effective = (dw * dh).min(original);
        let wasted = (w * h) as i128 - effective as i128;
        let wasted = wasted.max(0) as u64;
        if first || effective > max_effective || (effective == max_effective && wasted < min_wasted) {
            first = false;
            max_effective = effective;
            min_wasted = wasted;
            best = (h, w);
        }
    }

    let (bh, bw) = best;
    let mut ch = side * (bh / tile);
    let mut cw = side * (bw / tile);
    if ch == 0 || cw == 0 {
        return Err(format!("anyres: pinpoint {bh}x{bw} is smaller than one {tile} px tile"));
    }
    // Unpad. Python compares floats: W/H > cw/ch.
    if ow / oh > cw as f64 / ch as f64 {
        let nh = round7_trunc(oh * (cw as f64 / ow));
        ch -= ch.saturating_sub(nh) / 2 * 2;
    } else {
        let nw = round7_trunc(ow * (ch as f64 / oh));
        cw -= cw.saturating_sub(nw) / 2 * 2;
    }
    let total = side * side + ch * cw + ch;
    Ok(vec![(None, total as usize)])
}

/// Python's `int(round(x, 7))` for a non-negative float: `round` to 7 decimals is correctly
/// rounded from the exact binary value, as Rust's `{:.7}` formatting is. An exact decimal tie
/// cannot sit next to a whole number, so the tie rule never changes the truncated result.
fn round7_trunc(x: f64) -> u64 {
    let s = format!("{x:.7}");
    s.parse::<f64>().map(|r| r.trunc() as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r4b() -> serde_json::Value {
        serde_json::json!({
            "tile": 384,
            "side": 27,
            "pinpoints": [[384, 768], [768, 384], [768, 768], [1152, 384], [384, 1152]]
        })
    }

    #[test]
    fn an_r4b_picture_is_laid_out_as_its_processor_lays_it_out() {
        let reference = [
            ((64, 64), 1512),
            ((224, 224), 1512),
            ((256, 256), 1512),
            ((448, 448), 3699),
            ((512, 512), 3699),
            ((640, 480), 2929),
            ((800, 600), 2929),
            ((1024, 768), 2929),
            ((1280, 720), 2379),
            ((1920, 1080), 2379),
            ((2000, 200), 1467),
            ((200, 2000), 1539),
            ((768, 768), 3699),
            ((1000, 1000), 3699),
            ((4000, 3000), 2929),
            ((3000, 200), 1139),
            ((1152, 384), 2943),
            ((385, 1000), 2717),
        ];
        let params = r4b();
        for ((w, h), want) in reference {
            let runs = layout(&params, w, h).unwrap();
            let got: usize = runs.iter().map(|r| r.1).sum();
            assert_eq!(got, want, "{w}x{h}");
        }
        // 1920x1080 picks 768x768: grid 54x54, unpadded to 30 rows; one run of <image>.
        assert_eq!(layout(&params, 1920, 1080).unwrap(), vec![(None, 729 + 30 * 54 + 30)]);
    }

    #[test]
    fn a_picture_without_pixels_is_refused() {
        assert!(layout(&r4b(), 0, 10).is_err());
    }
}

#[cfg(test)]
mod granite_tests {
    fn minimax_vl_01() -> serde_json::Value {
        let config = [
            (336, 336),
            (336, 672),
            (336, 1008),
            (336, 1344),
            (336, 1680),
            (336, 2016),
            (672, 336),
            (672, 672),
            (672, 1008),
            (672, 1344),
            (672, 1680),
            (672, 2016),
            (1008, 336),
            (1008, 672),
            (1008, 1008),
            (1008, 1344),
            (1008, 1680),
            (1008, 2016),
            (1344, 336),
            (1344, 672),
            (1344, 1008),
            (1344, 1344),
            (1680, 336),
            (1680, 672),
            (1680, 1008),
            (2016, 336),
            (2016, 672),
            (2016, 1008),
        ];
        let pinpoints: Vec<[u32; 2]> = config.iter().map(|&(w, h)| [h, w]).collect();
        serde_json::json!({"tile": 336, "side": 24, "pinpoints": pinpoints})
    }

    #[test]
    fn a_granite_vision_picture_is_counted_as_its_processor_counts_it() {
        // Granite Vision 4.1 4B's processor (LlavaNext with a downsample of 1/2), run in Docker.
        let mut pinpoints = vec![];
        for n in 1..=10 {
            pinpoints.push([384, 384 * n]);
        }
        for (h, w) in [
            (768, 384),
            (768, 768),
            (768, 1152),
            (768, 1536),
            (768, 1920),
            (1152, 384),
            (1152, 768),
            (1152, 1152),
            (1536, 384),
            (1536, 768),
            (1920, 384),
            (1920, 768),
        ] {
            pinpoints.push([h, w]);
        }
        for n in 6..=10 {
            pinpoints.push([384 * n, 384]);
        }
        let rule = serde_json::json!({"tile": 384, "side": 12, "pinpoints": pinpoints});
        for ((w, h), want) in [
            ((64, 64), 300),
            ((448, 448), 744),
            ((640, 480), 594),
            ((800, 600), 936),
            ((1280, 720), 1176),
            ((2000, 200), 728),
            ((200, 2000), 792),
            ((1000, 1000), 1476),
        ] {
            let got: usize = super::layout(&rule, w, h).unwrap().iter().map(|r| r.1).sum();
            assert_eq!(got, want, "{w}x{h}");
        }
    }

    #[test]
    fn a_minimax_vl_01_picture_is_counted_as_its_processor_counts_it() {
        let rule = minimax_vl_01();
        for ((w, h), want) in [
            ((64, 64), 1176),
            ((224, 224), 1176),
            ((256, 256), 1176),
            ((448, 448), 2928),
            ((512, 512), 2928),
            ((640, 480), 2340),
            ((800, 600), 3696),
            ((1024, 768), 7560),
            ((1280, 720), 5814),
            ((1920, 1080), 9864),
            ((2000, 200), 2606),
            ((200, 2000), 2736),
            ((768, 768), 5832),
            ((1000, 1000), 5832),
            ((336, 336), 1176),
            ((672, 336), 1752),
            ((3000, 1000), 7536),
            ((1000, 3000), 7632),
            ((50, 500), 816),
        ] {
            let runs = super::layout(&rule, w, h).unwrap();
            assert_eq!(runs, vec![(None, want)], "{w}x{h}");
        }
    }
}
