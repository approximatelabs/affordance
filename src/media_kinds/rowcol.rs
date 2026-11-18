//! The Idefics3 row and column split: SmolVLM2 (`SmolVLMProcessor`) and granite-docling
//! (`Idefics3Processor`), whose processors (transformers 5.19, run in Docker) resize every
//! picture so its longest side is `longest` pixels, round both sides up to whole `tile`-pixel
//! tiles, and write each tile as an around marker, its own `<row_r_col_c>` marker and `seq`
//! placeholders, rows separated by a newline token and the last row by a double-newline token
//! (the tokenizer merges the two newlines), then one global copy of the whole picture.
//! The template writes only the placeholder; everything here replaces it.

/// The runs for a picture of `width` by `height`. Params: `longest`, `tile`, `seq` (numbers),
/// `cap` (number, default 4096), `split` (bool, default true), and the token names `around`,
/// `global`, `row_col` (a pattern with `{r}` and `{c}`), `newline`, `end_newline`.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let number = |key: &str| {
        params[key]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or_else(|| format!("rowcol: `{key}` must be a positive number"))
    };
    let text = |key: &str| {
        params[key]
            .as_str()
            .map(String::from)
            .ok_or_else(|| format!("rowcol: `{key}` must be a token name"))
    };
    if width == 0 || height == 0 {
        return Err("rowcol: a picture needs pixels".into());
    }
    let longest = number("longest")? as i64;
    let tile = number("tile")? as i64;
    let seq = number("seq")? as usize;
    let cap = params.get("cap").map_or(Ok(4096), |_| number("cap"))? as i64;
    let split = params
        .get("split")
        .map_or(Some(true), |v| v.as_bool())
        .ok_or("rowcol: `split` must be true or false")?;
    let around = text("around")?;
    let global = text("global")?;

    // `_resize_output_size_rescale_to_max_len`: the longest side to `longest`, the other side
    // truncated and bumped to even.
    let (mut w, mut h) = (width as i64, height as i64);
    let aspect = w as f64 / h as f64;
    if w >= h {
        w = longest;
        h = (w as f64 / aspect) as i64;
        h += h % 2;
    } else {
        h = longest;
        w = (h as f64 * aspect) as i64;
        w += w % 2;
    }
    (w, h) = (w.max(1), h.max(1));
    // `_resize_output_size_scale_below_upper_bound`: the 4096 cap.
    let aspect = w as f64 / h as f64;
    if w >= h && w > cap {
        w = cap;
        h = (w as f64 / aspect) as i64;
    } else if h > w && h > cap {
        h = cap;
        w = (h as f64 * aspect) as i64;
    }
    (w, h) = (w.max(1), h.max(1));

    let mut runs = Vec::new();
    if split {
        // `resize_for_vision_encoder`: round up to whole tiles along the longer side, then the
        // other side from the aspect ratio.
        let up = |x: f64| (x / tile as f64).ceil() as i64 * tile;
        let aspect = w as f64 / h as f64;
        let (w2, h2) = if w >= h {
            let w2 = up(w as f64);
            (w2, up(((w2 as f64 / aspect) as i64) as f64))
        } else {
            let h2 = up(h as f64);
            (up(((h2 as f64 * aspect) as i64) as f64), h2)
        };
        if h2 > tile || w2 > tile {
            let (rows, cols) = (h2 / tile, w2 / tile);
            let row_col = text("row_col")?;
            for r in 1..=rows {
                for c in 1..=cols {
                    runs.push((Some(around.clone()), 1));
                    let marker = row_col.replace("{r}", &r.to_string()).replace("{c}", &c.to_string());
                    runs.push((Some(marker), 1));
                    runs.push((None, seq));
                }
                let end = if r < rows {
                    text("newline")?
                } else {
                    text("end_newline")?
                };
                runs.push((Some(end), 1));
            }
        }
    }
    runs.push((Some(around.clone()), 1));
    runs.push((Some(global), 1));
    runs.push((None, seq));
    runs.push((Some(around), 1));
    Ok(runs)
}

#[cfg(test)]
mod tests {
    fn rule(longest: u32, tile: u32, seq: u32) -> serde_json::Value {
        serde_json::json!({
            "longest": longest, "tile": tile, "seq": seq,
            "around": "<fake_token_around_image>", "global": "<global-img>",
            "row_col": "<row_{r}_col_{c}>", "newline": "Ċ", "end_newline": "ĊĊ",
        })
    }

    fn total(rule: &serde_json::Value, w: u32, h: u32) -> usize {
        super::layout(rule, w, h).unwrap().iter().map(|r| r.1).sum()
    }

    /// The fourteen common sizes: the processor's counts in the spec.
    const COMMON: [(u32, u32); 14] = [
        (64, 64),
        (224, 224),
        (256, 256),
        (448, 448),
        (512, 512),
        (640, 480),
        (800, 600),
        (1024, 768),
        (1280, 720),
        (1920, 1080),
        (2000, 200),
        (200, 2000),
        (768, 768),
        (1000, 1000),
    ];

    #[test]
    fn a_smolvlm2_picture_is_laid_out_as_its_processor_lays_it_out() {
        let rule = rule(1536, 384, 81);
        let counts = [
            1416, 1416, 1416, 1416, 1416, 1083, 1083, 1083, 1083, 1083, 417, 420, 1416, 1416,
        ];
        for ((w, h), n) in COMMON.into_iter().zip(counts) {
            assert_eq!(total(&rule, w, h), n, "{w}x{h}");
        }
        // The 70 further sizes the spec's script compared token for token with the processor.
        for (w, h, n) in [
            (551, 2332, 420),
            (259, 1045, 420),
            (483, 2030, 420),
            (1842, 1935, 1416),
            (2669, 1555, 1083),
            (860, 385, 750),
            (1999, 117, 417),
            (1597, 1773, 1416),
            (2489, 9, 417),
            (2851, 1825, 1083),
            (1091, 2956, 752),
            (938, 2422, 752),
            (419, 1301, 752),
            (126, 92, 1083),
            (105, 2661, 420),
            (2218, 38, 417),
            (1562, 2812, 1084),
            (888, 1729, 1084),
            (2974, 119, 417),
            (2162, 909, 750),
            (1794, 2031, 1416),
            (2265, 955, 750),
            (1416, 946, 1083),
            (2773, 897, 750),
            (1883, 1187, 1083),
            (89, 1705, 420),
            (2280, 2631, 1416),
            (410, 762, 1084),
            (2578, 2965, 1416),
            (1215, 496, 750),
            (1363, 2956, 752),
            (2914, 2052, 1083),
            (1729, 2080, 1416),
            (2746, 778, 750),
            (1243, 1164, 1416),
            (2407, 2046, 1416),
            (2070, 1612, 1416),
            (2413, 142, 417),
            (1968, 995, 1083),
            (1656, 1698, 1416),
            (2723, 709, 750),
            (1504, 2248, 1084),
            (2880, 2763, 1416),
            (1535, 355, 417),
            (1798, 2719, 1084),
            (2083, 443, 417),
            (671, 2134, 752),
            (1611, 1518, 1416),
            (2006, 122, 417),
            (1923, 179, 417),
            (1264, 2882, 752),
            (2519, 2430, 1416),
            (2369, 1613, 1083),
            (2651, 698, 750),
            (691, 2058, 752),
            (930, 51, 417),
            (818, 2211, 752),
            (2246, 951, 750),
            (1657, 2105, 1416),
            (1409, 2367, 1084),
            (1536, 385, 750),
            (1537, 384, 417),
            (385, 1536, 752),
            (1, 1, 1416),
            (3000, 1, 417),
            (1, 3000, 420),
            (1536, 1152, 1083),
            (1153, 1536, 1416),
            (3, 2, 1083),
            (2, 3, 1084),
        ] {
            assert_eq!(total(&rule, w, h), n, "{w}x{h}");
        }
        // 640x480 is 3 rows of 4 tiles, then the global picture.
        let mut whole: Vec<(Option<String>, usize)> = Vec::new();
        let named = |s: &str| (Some(s.to_string()), 1);
        for r in 1..=3 {
            for c in 1..=4 {
                whole.extend([
                    named("<fake_token_around_image>"),
                    named(&format!("<row_{r}_col_{c}>")),
                    (None, 81),
                ]);
            }
            whole.push(named(if r < 3 { "Ċ" } else { "ĊĊ" }));
        }
        whole.extend([
            named("<fake_token_around_image>"),
            named("<global-img>"),
            (None, 81),
            named("<fake_token_around_image>"),
        ]);
        assert_eq!(super::layout(&rule, 640, 480).unwrap(), whole);
        // With splitting off, every picture is the global block alone.
        let mut unsplit = rule.clone();
        unsplit["split"] = serde_json::json!(false);
        assert_eq!(total(&unsplit, 1920, 1080), 84);
    }

    #[test]
    fn a_granite_docling_picture_is_laid_out_as_its_processor_lays_it_out() {
        let rule = rule(2048, 512, 64);
        let counts = [
            1127, 1127, 1127, 1127, 1127, 862, 862, 862, 862, 862, 332, 335, 1127, 1127,
        ];
        for ((w, h), n) in COMMON.into_iter().zip(counts) {
            assert_eq!(total(&rule, w, h), n, "{w}x{h}");
        }
        // The 189 further sizes the spec's script compared with the processor.
        for (w, h, n) in [
            (551, 2332, 335),
            (259, 1045, 335),
            (483, 2030, 335),
            (1842, 1935, 1127),
            (2669, 1555, 862),
            (860, 385, 597),
            (1999, 117, 332),
            (1597, 1773, 1127),
            (2489, 9, 332),
            (2851, 1825, 862),
            (1091, 2956, 599),
            (938, 2422, 599),
            (419, 1301, 599),
            (126, 92, 862),
            (105, 2661, 335),
            (2218, 38, 332),
            (1562, 2812, 863),
            (888, 1729, 863),
            (2974, 119, 332),
            (2162, 909, 597),
            (1794, 2031, 1127),
            (2265, 955, 597),
            (1416, 946, 862),
            (2773, 897, 597),
            (1883, 1187, 862),
            (89, 1705, 335),
            (2280, 2631, 1127),
            (410, 762, 863),
            (2578, 2965, 1127),
            (1215, 496, 597),
            (1363, 2956, 599),
            (2914, 2052, 862),
            (1729, 2080, 1127),
            (2746, 778, 597),
            (1243, 1164, 1127),
            (2407, 2046, 1127),
            (2070, 1612, 1127),
            (2413, 142, 332),
            (1968, 995, 862),
            (1656, 1698, 1127),
            (2723, 709, 597),
            (1504, 2248, 863),
            (2880, 2763, 1127),
            (1535, 355, 332),
            (1798, 2719, 863),
            (2083, 443, 332),
            (671, 2134, 599),
            (1611, 1518, 1127),
            (2006, 122, 332),
            (1923, 179, 332),
            (1264, 2882, 599),
            (2519, 2430, 1127),
            (2369, 1613, 862),
            (2651, 698, 597),
            (691, 2058, 599),
            (930, 51, 332),
            (818, 2211, 599),
            (2246, 951, 597),
            (1657, 2105, 1127),
            (1409, 2367, 863),
            (1448, 1881, 1127),
            (1103, 2701, 599),
            (2245, 2495, 1127),
            (2988, 24, 332),
            (1572, 2100, 863),
            (530, 2125, 335),
            (2300, 842, 597),
            (1746, 230, 332),
            (1971, 1494, 1127),
            (2335, 2271, 1127),
            (819, 2068, 599),
            (1694, 1987, 1127),
            (1462, 1698, 1127),
            (1418, 7, 332),
            (2206, 2213, 1127),
            (2554, 2509, 1127),
            (1357, 1877, 863),
            (2458, 115, 332),
            (941, 2603, 599),
            (726, 2256, 599),
            (2394, 741, 597),
            (376, 2258, 335),
            (1046, 133, 332),
            (2758, 289, 332),
            (341, 69, 332),
            (1856, 60, 332),
            (1152, 1023, 1127),
            (1101, 449, 597),
            (2560, 757, 597),
            (1411, 1190, 1127),
            (285, 686, 599),
            (654, 1046, 863),
            (2161, 689, 597),
            (2690, 1118, 597),
            (2656, 2915, 1127),
            (1207, 1863, 863),
            (2878, 1319, 597),
            (2034, 1941, 1127),
            (468, 97, 332),
            (1278, 1584, 1127),
            (1407, 1725, 1127),
            (771, 1059, 863),
            (446, 1039, 599),
            (2991, 2090, 862),
            (857, 2481, 599),
            (1769, 86, 332),
            (924, 74, 332),
            (1628, 600, 597),
            (145, 2945, 335),
            (657, 1826, 599),
            (2887, 2074, 862),
            (2778, 1748, 862),
            (2232, 904, 597),
            (2584, 2847, 1127),
            (2116, 1847, 1127),
            (915, 2146, 599),
            (2657, 126, 332),
            (1618, 2765, 863),
            (2359, 1316, 862),
            (2703, 2585, 1127),
            (1747, 241, 332),
            (1224, 515, 597),
            (869, 195, 332),
            (1255, 290, 332),
            (314, 1272, 335),
            (1221, 649, 862),
            (1705, 2314, 863),
            (1034, 535, 862),
            (35, 2297, 335),
            (156, 2420, 335),
            (892, 2336, 599),
            (1888, 703, 597),
            (2884, 2552, 1127),
            (2085, 154, 332),
            (1549, 821, 862),
            (1422, 406, 597),
            (843, 2349, 599),
            (2762, 1774, 862),
            (2423, 796, 597),
            (2017, 428, 332),
            (2728, 1598, 862),
            (1213, 2065, 863),
            (2048, 71, 332),
            (1333, 2508, 863),
            (1648, 1153, 862),
            (75, 643, 335),
            (823, 1343, 863),
            (2308, 554, 332),
            (1389, 1759, 1127),
            (873, 1092, 1127),
            (50, 3432, 335),
            (195, 3819, 335),
            (281, 1409, 335),
            (274, 1985, 335),
            (273, 962, 599),
            (34, 2972, 335),
            (21, 347, 335),
            (69, 696, 335),
            (86, 3732, 335),
            (276, 873, 599),
            (138, 3110, 335),
            (171, 2459, 335),
            (260, 3446, 335),
            (131, 1508, 335),
            (174, 1394, 335),
            (59, 1193, 335),
            (121, 3553, 335),
            (251, 555, 599),
            (297, 2258, 335),
            (54, 1314, 335),
            (21, 1666, 335),
            (38, 1558, 335),
            (76, 3394, 335),
            (65, 1397, 335),
            (59, 2520, 335),
            (194, 314, 863),
            (293, 2254, 335),
            (115, 2319, 335),
            (42, 3901, 335),
            (137, 1495, 335),
            (5000, 10, 332),
            (10, 5000, 335),
            (1, 1, 1127),
            (383, 385, 1127),
            (384, 384, 1127),
            (385, 384, 1127),
            (1536, 2048, 863),
            (511, 2048, 335),
            (513, 2048, 599),
        ] {
            assert_eq!(total(&rule, w, h), n, "{w}x{h}");
        }
        // 2000x200 is one row of 4 tiles, closed by the double newline, then the global picture.
        let named = |s: &str| (Some(s.to_string()), 1);
        let mut whole: Vec<(Option<String>, usize)> = Vec::new();
        for c in 1..=4 {
            whole.extend([
                named("<fake_token_around_image>"),
                named(&format!("<row_1_col_{c}>")),
                (None, 64),
            ]);
        }
        whole.extend([
            named("ĊĊ"),
            named("<fake_token_around_image>"),
            named("<global-img>"),
            (None, 64),
            named("<fake_token_around_image>"),
        ]);
        assert_eq!(super::layout(&rule, 2000, 200).unwrap(), whole);
    }
}
