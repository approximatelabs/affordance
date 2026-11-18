//! Ovis2.5 and Ovis2.6 pictures.

/// Ovis2.5-2B, Ovis2.5-9B and Ovis2.6-30B-A3B. These repositories have no processor class: the
/// model's own Python (`VisualTokenizer.smart_resize` and `Ovis2_5._merge_inputs` in
/// `modeling_ovis2_5.py`, the same code in `modeling_ovis2_6.py`) resizes the picture onto a grid
/// of 32-pixel cells (16-pixel patches merged 2 by 2), one token per cell, and replaces the
/// template's `<image>` text with a start indicator, the cells and an end indicator (ids -301,
/// N x -300, -302 in the Hugging Face code). vLLM's port writes that run as N + 2 copies of
/// `<|image_pad|>`, which is what this returns. Reference counts came from running the model
/// code's `preprocess_inputs` in `lmsysorg/sglang:v0.5.23` with no network; SGLang 0.5.23 itself
/// has no Ovis2.5 or Ovis2.6 model.
///
/// Params: `cell` (32), `min_pixels` (200704), `max_pixels` (2408448 for Ovis2.5-2B, 3211264
/// for the other two), `token` (default `<|image_pad|>`).
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let number = |key: &str| {
        params[key]
            .as_u64()
            .ok_or_else(|| format!("ovis layout needs a whole number `{key}`: {params}"))
    };
    let factor = number("cell")?;
    let (min_pixels, max_pixels) = (number("min_pixels")?, number("max_pixels")?);
    let token = params["token"].as_str().unwrap_or("<|image_pad|>").to_string();
    if factor == 0 || width == 0 || height == 0 {
        return Err(format!("ovis can not lay out a {width}x{height} picture"));
    }
    let (h_bar, w_bar) = smart_resize(height as u64, width as u64, factor, min_pixels, max_pixels);
    if h_bar == 0 || w_bar == 0 {
        return Err(format!("ovis resizes a {width}x{height} picture to nothing"));
    }
    let cells = ((h_bar / factor) * (w_bar / factor)) as usize;
    Ok(vec![(Some(token), cells + 2)])
}

/// Python's `round` on a float: halves go to the even neighbour.
fn py_round(x: f64) -> u64 {
    x.round_ties_even() as u64
}

/// `VisualTokenizer.smart_resize`, step for step, returning (height, width).
fn smart_resize(mut height: u64, mut width: u64, factor: u64, min: u64, max: u64) -> (u64, u64) {
    let f = factor as f64;
    if height < factor || width < factor {
        if height < width {
            width = py_round(f / height as f64 * width as f64);
            height = factor;
        } else {
            height = py_round(f / width as f64 * height as f64);
            width = factor;
        }
    } else if height.max(width) as f64 / height.min(width) as f64 > 200.0 {
        if height > width {
            height = 200 * width;
        } else {
            width = 200 * height;
        }
    }
    let (hf, wf) = (height as f64, width as f64);
    let mut h_bar = py_round(hf / f) * factor;
    let mut w_bar = py_round(wf / f) * factor;
    if h_bar * w_bar > max {
        let beta = (hf * wf / max as f64).sqrt();
        h_bar = (hf / beta / f).floor() as u64 * factor;
        w_bar = (wf / beta / f).floor() as u64 * factor;
    } else if h_bar * w_bar < min {
        let beta = (min as f64 / (hf * wf)).sqrt();
        h_bar = (hf * beta / f).ceil() as u64 * factor;
        w_bar = (wf * beta / f).ceil() as u64 * factor;
    }
    (h_bar, w_bar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn total(params: &serde_json::Value, w: u32, h: u32) -> usize {
        layout(params, w, h).unwrap().iter().map(|(_, n)| n).sum()
    }

    #[test]
    fn an_ovis_picture_is_laid_out_as_its_model_code_lays_it_out() {
        let small = json!({"cell": 32, "min_pixels": 200704, "max_pixels": 2408448});
        let large = json!({"cell": 32, "min_pixels": 200704, "max_pixels": 3211264});
        let shared = [
            (64, 64, 198),
            (224, 224, 198),
            (256, 256, 198),
            (448, 448, 198),
            (512, 512, 258),
            (640, 480, 302),
            (800, 600, 477),
            (1024, 768, 770),
            (1280, 720, 882),
            (1920, 1080, 2042),
            (2000, 200, 374),
            (200, 2000, 374),
            (768, 768, 578),
            (1000, 1000, 963),
            (16, 16, 198),
            (20, 2000, 282),
            (2000, 20, 282),
            (10, 5000, 502),
            (449, 449, 198),
            (100, 30000, 1877),
        ];
        for params in [&small, &large] {
            for (w, h, count) in shared {
                assert_eq!(total(params, w, h), count, "{w}x{h} with {params}");
            }
        }
        assert_eq!(total(&small, 3000, 3000), 2306);
        assert_eq!(total(&large, 3000, 3000), 3138);
        assert_eq!(
            layout(&small, 640, 480).unwrap(),
            vec![(Some("<|image_pad|>".to_string()), 302)]
        );
    }

    #[test]
    fn an_empty_picture_is_refused() {
        let params = json!({"cell": 32, "min_pixels": 200704, "max_pixels": 2408448});
        assert!(layout(&params, 0, 10).is_err());
    }
}
