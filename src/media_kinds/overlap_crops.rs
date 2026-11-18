/// Picture tokens for allenai Molmo2-8B. The repository's `Molmo2ImageProcessor`
/// (`image_processing_molmo2.py`) picks a tiling of up to `max_crops` overlapping 378 px crops
/// with `select_tiling` on the picture minus the margins, in numpy float32 (a side at or below
/// the margins divides by zero or a negative, as numpy does), then pools the crops' patches
/// 2 by 2, rounding up. `Molmo2Processor.get_image_tokens` writes a low-res block
/// `<low_res_im_start>` + 14x14 `<im_patch>` + `<im_end>` (no column tokens), then
/// `<im_start>` + h rows of (w `<im_patch>` + `<im_col>`) + `<im_end>`, all in place of the
/// template's `<|image|>`. The reference counts come from the repository's code run in Docker
/// (14 sizes) and a rule that matched it token for token on 91 more.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let count = |key: &str, default: u64| -> Result<u64, String> {
        match params.get(key) {
            None => Ok(default),
            Some(v) => v
                .as_u64()
                .ok_or_else(|| format!("overlap_crops: parameter {key} must be a whole number")),
        }
    };
    let name = |key: &str, default: &str| -> Result<String, String> {
        match params.get(key) {
            None => Ok(default.to_string()),
            Some(v) => v
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("overlap_crops: parameter {key} must be a token name")),
        }
    };
    let crop = count("crop", 378)? as i64;
    let patch = count("patch", 14)? as i64;
    let margin_left = count("margin_left", 4)? as i64;
    let margin_right = count("margin_right", 4)? as i64;
    let max_crops = count("max_crops", 8)? as i64;
    let pool = count("pool", 2)? as i64;
    let low_res_start = name("low_res_start", "<low_res_im_start>")?;
    let im_start = name("im_start", "<im_start>")?;
    let im_end = name("im_end", "<im_end>")?;
    let im_patch = name("im_patch", "<im_patch>")?;
    let im_col = name("im_col", "<im_col>")?;

    if width == 0 || height == 0 {
        return Err(format!("overlap_crops: a {width}x{height} picture has no pixels"));
    }
    if patch == 0 || pool == 0 || max_crops == 0 {
        return Err("overlap_crops: patch, pool and max_crops must be above zero".into());
    }
    let crop_patches = crop / patch;
    let margin = margin_left + margin_right;
    let window_patches = crop_patches - margin;
    if window_patches <= 0 {
        return Err("overlap_crops: the margins cover the whole crop".into());
    }
    let window = window_patches * patch;
    let margin_px = margin * patch;

    // select_tiling(H - margin_px, W - margin_px, window, max_crops) in numpy float32: x / 0 is
    // inf and x / a negative side is negative, as numpy gives them.
    let h = (i64::from(height) - margin_px) as f32;
    let w = (i64::from(width) - margin_px) as f32;
    let mut tilings = Vec::new();
    for i in 1..=max_crops {
        for j in 1..=max_crops {
            if i * j <= max_crops {
                tilings.push((i, j));
            }
        }
    }
    tilings.sort_by_key(|&(i, j)| (i * j, i));
    let scales: Vec<f32> = tilings
        .iter()
        .map(|&(i, j)| ((i * window) as f32 / h).min((j * window) as f32 / w))
        .collect();
    // np.argmax / np.argmin: the first index holding the extreme value.
    let first = |vals: &[f32], better: fn(f32, f32) -> bool| {
        let mut ix = 0;
        for (k, &v) in vals.iter().enumerate() {
            if better(v, vals[ix]) {
                ix = k;
            }
        }
        ix
    };
    let ix = if scales.iter().all(|&s| s < 1.0) {
        first(&scales, |a, b| a > b)
    } else {
        let masked: Vec<f32> = scales.iter().map(|&s| if s < 1.0 { 10e9 } else { s }).collect();
        first(&masked, |a, b| a < b)
    };
    let (rows, cols) = tilings[ix];

    // The crops cover rows*window_patches + margin patches a side; pooling pads up to a whole pool.
    let ceil = |n: i64| ((n + pool - 1) / pool) as usize;
    let th = ceil(rows * window_patches + margin);
    let tw = ceil(cols * window_patches + margin);
    let g = ceil(crop_patches);

    let mut runs = vec![
        (Some(low_res_start), 1),
        (Some(im_patch.clone()), g * g),
        (Some(im_end.clone()), 1),
        (Some(im_start), 1),
    ];
    for _ in 0..th {
        runs.push((Some(im_patch.clone()), tw));
        runs.push((Some(im_col.clone()), 1));
    }
    runs.push((Some(im_end), 1));
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::layout;

    fn total(w: u32, h: u32) -> usize {
        layout(&serde_json::json!({}), w, h).unwrap().iter().map(|r| r.1).sum()
    }

    #[test]
    fn a_molmo2_picture_is_laid_out_as_its_processor_lays_it_out() {
        // The processor's own counts, run in Docker.
        let reference: &[(u32, u32, usize)] = &[
            (64, 64, 410),
            (224, 224, 410),
            (256, 256, 410),
            (448, 448, 752),
            (512, 512, 752),
            (640, 480, 752),
            (800, 600, 982),
            (1024, 768, 982),
            (1280, 720, 1189),
            (1920, 1080, 1189),
            (2000, 200, 1334),
            (200, 2000, 1400),
            (768, 768, 752),
            (1000, 1000, 752),
        ];
        // 91 more sizes whose sequences matched the processor's token for token.
        let matched: &[(u32, u32, usize)] = &[
            (232, 376, 410),
            (348, 1479, 1115),
            (693, 2744, 980),
            (1263, 1031, 982),
            (2482, 870, 1189),
            (2486, 147, 1334),
            (2381, 2791, 992),
            (649, 1765, 1208),
            (2616, 1612, 1189),
            (2962, 2086, 982),
            (1524, 2229, 992),
            (1823, 2057, 992),
            (1099, 148, 802),
            (113, 1492, 1115),
            (1905, 1305, 1189),
            (1557, 1736, 992),
            (2154, 674, 1189),
            (2296, 727, 1189),
            (968, 945, 982),
            (98, 724, 410),
            (1332, 712, 1189),
            (560, 2090, 980),
            (2090, 1474, 982),
            (2105, 2763, 992),
            (2294, 745, 1189),
            (1826, 1699, 982),
            (2152, 1492, 982),
            (2431, 1450, 1189),
            (1483, 1826, 992),
            (661, 1638, 1208),
            (2930, 1890, 1189),
            (2683, 2173, 982),
            (1024, 2008, 1208),
            (1144, 2041, 1208),
            (2052, 2112, 992),
            (1450, 2711, 1208),
            (1863, 1889, 992),
            (1437, 2326, 1208),
            (2974, 2284, 982),
            (2965, 1871, 1189),
            (1994, 2699, 992),
            (909, 1330, 1208),
            (2866, 681, 942),
            (2525, 1099, 1189),
            (1966, 1268, 1189),
            (1243, 2893, 1208),
            (2066, 2303, 992),
            (2121, 2079, 982),
            (2669, 2523, 982),
            (2409, 1666, 982),
            (1278, 2994, 1208),
            (852, 2003, 1208),
            (2097, 1502, 982),
            (2803, 2553, 982),
            (309, 1399, 980),
            (2974, 35, 410),
            (784, 435, 982),
            (241, 2353, 1400),
            (2675, 201, 1334),
            (1119, 2424, 1208),
            (929, 2796, 1208),
            (436, 2140, 1265),
            (560, 1089, 1208),
            (1003, 863, 982),
            (248, 1733, 1265),
            (2937, 131, 1334),
            (233, 1485, 1115),
            (1476, 705, 1189),
            (1022, 2756, 1208),
            (97, 340, 410),
            (472, 277, 536),
            (104, 168, 410),
            (2988, 87, 410),
            (1529, 1048, 1189),
            (524, 644, 752),
            (753, 2143, 1208),
            (2833, 8, 410),
            (1580, 2415, 1208),
            (177, 1016, 830),
            (621, 149, 536),
            (1, 1, 410),
            (112, 112, 410),
            (113, 113, 410),
            (111, 500, 410),
            (500, 111, 410),
            (378, 378, 410),
            (379, 379, 752),
            (644, 378, 536),
            (378, 644, 545),
            (3000, 1, 410),
            (1, 3000, 410),
        ];
        assert_eq!(matched.len(), 91);
        for &(w, h, n) in reference.iter().chain(matched) {
            assert_eq!(total(w, h), n, "{w}x{h}");
        }

        // 2000x200 tiles as 1 row by 8 columns: 14 pooled rows of 80 patches, each with a column token.
        let s = |t: &str| Some(t.to_string());
        let mut want = vec![
            (s("<low_res_im_start>"), 1),
            (s("<im_patch>"), 196),
            (s("<im_end>"), 1),
            (s("<im_start>"), 1),
        ];
        for _ in 0..14 {
            want.push((s("<im_patch>"), 80));
            want.push((s("<im_col>"), 1));
        }
        want.push((s("<im_end>"), 1));
        assert_eq!(layout(&serde_json::json!({}), 2000, 200).unwrap(), want);
    }
}
