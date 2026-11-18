//! The `nearest` picture layout.

/// Muse Glimmer 30B (`meta-models__Muse-Glimmer-30B`). Its processor (`MuseGlimmerImageProcessor`,
/// transformers 5.19, `smart_resize`; SGLang's `muse_glimmer.py` calls it unchanged) measures the
/// picture in cells (`ih = H/cell`, `iw = W/cell`, float64), scales both down to `most` cells
/// when over it, tries the grids made of the floor and ceiling of each, keeps those with both
/// sides at least 1 and at most `most` cells, and picks the one whose rows/cols is closest to
/// H/W. Ties go to the grid that comes first when CPython iterates a set of the candidate
/// tuples, so this function computes CPython's tuple hash and 8-slot set insertion to get that
/// order. The template's `<|patch|>` becomes `start` + rows*cols `<|patch|>` + `end`. Reference:
/// the processor in docker (sglang v0.5.23, Python 3.12) on 24 sizes and 409,201 fuzzed sizes,
/// and OpenRouter (Together) on 14 sizes (count + 2).
///
/// Params: `cell` (pixels per cell side), `most` (cell cap), `start` and `end` (marker names).
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let int = |key: &str| {
        params
            .get(key)
            .and_then(|v| v.as_u64())
            .filter(|&v| v > 0)
            .ok_or_else(|| format!("nearest: `{key}` must be a positive integer"))
    };
    let text = |key: &str| {
        params
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| format!("nearest: `{key}` must be a token name"))
    };
    let cell = int("cell")?;
    let most = int("most")?;
    let start = text("start")?;
    let end = text("end")?;
    if width == 0 || height == 0 {
        return Err(format!("nearest: a {width}x{height} picture has no pixels"));
    }
    let (rows, cols) = grid(width, height, cell as f64, most);
    Ok(vec![(Some(start), 1), (None, (rows * cols) as usize), (Some(end), 1)])
}

/// `smart_resize`'s grid choice, in float64 as Python computes it.
fn grid(width: u32, height: u32, cell: f64, most: u64) -> (u64, u64) {
    let (w, h) = (f64::from(width), f64::from(height));
    let mut ih = h / cell;
    let mut iw = w / cell;
    let r = iw / ih;
    let most_f = most as f64;
    if ih * iw > most_f {
        // Python's `** 0.5` is C `pow(x, 0.5)`, which is correctly rounded like `sqrt`.
        ih = (most_f / r).sqrt();
        iw = ih * r;
    }
    let product = [
        (ih.floor() as u64, iw.floor() as u64),
        (ih.floor() as u64, iw.ceil() as u64),
        (ih.ceil() as u64, iw.floor() as u64),
        (ih.ceil() as u64, iw.ceil() as u64),
    ];
    let kept: Vec<(u64, u64)> = cpython_set_order(&product)
        .into_iter()
        .filter(|&(gh, gw)| gh >= 1 && gw >= 1 && gh * gw <= most)
        .collect();
    let candidates = if kept.is_empty() {
        vec![(round_half_even(ih).max(1.0) as u64, round_half_even(iw).max(1.0) as u64)]
    } else {
        kept
    };
    // Python's min() keeps the first of equal keys.
    let target = h / w;
    let mut best = candidates[0];
    let mut best_err = (best.0 as f64 / best.1 as f64 - target).abs();
    for &cand in &candidates[1..] {
        let err = (cand.0 as f64 / cand.1 as f64 - target).abs();
        if err < best_err {
            best = cand;
            best_err = err;
        }
    }
    best
}

/// Python's `round()` on a float: halves go to the even neighbour.
fn round_half_even(x: f64) -> f64 {
    x.round_ties_even()
}

const P1: u64 = 11400714785074694791;
const P2: u64 = 14029467366897019727;
const P5: u64 = 2870177450012600261;

/// CPython (3.8 and later) `hash((a, b))` for small non-negative ints, as an unsigned word.
fn cpython_tuple_hash(t: (u64, u64)) -> u64 {
    let mut acc = P5;
    for x in [t.0, t.1] {
        acc = acc.wrapping_add(x.wrapping_mul(P2));
        acc = acc.rotate_left(31);
        acc = acc.wrapping_mul(P1);
    }
    acc = acc.wrapping_add(2 ^ (P5 ^ 3527539));
    if acc == u64::MAX { 1546275796 } else { acc }
}

/// The order of `list(set(items))` in CPython for at most four tuples: an 8-slot table filled
/// in list order with CPython's probe sequence (no linear probing when the mask is 7).
fn cpython_set_order(items: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let mut table: [Option<(u64, u64)>; 8] = [None; 8];
    for &item in items {
        let h = cpython_tuple_hash(item);
        let mut i = (h & 7) as usize;
        let mut perturb = h;
        loop {
            match table[i] {
                None => {
                    table[i] = Some(item);
                    break;
                }
                Some(existing) if existing == item => break,
                Some(_) => {
                    perturb >>= 5;
                    i = ((i as u64).wrapping_mul(5).wrapping_add(1).wrapping_add(perturb) & 7) as usize;
                }
            }
        }
    }
    table.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn muse() -> serde_json::Value {
        serde_json::json!({
            "cell": 28,
            "most": 4096,
            "start": "<|image_start|>",
            "end": "<|image_end|>",
        })
    }

    fn total(w: u32, h: u32) -> usize {
        layout(&muse(), w, h).unwrap().iter().map(|r| r.1).sum()
    }

    #[test]
    fn a_muse_glimmer_picture_is_laid_out_as_its_processor_lays_it_out() {
        // Processor counts of <|patch|> (docker, transformers 5.19, Python 3.12).
        let processor = [
            ("64x64", 9),
            ("224x224", 64),
            ("256x256", 100),
            ("448x448", 256),
            ("512x512", 361),
            ("640x480", 391),
            ("800x600", 588),
            ("1024x768", 972),
            ("1280x720", 1196),
            ("1920x1080", 2691),
            ("2000x200", 497),
            ("200x2000", 497),
            ("768x768", 784),
            ("1000x1000", 1296),
            ("4000x3000", 4015),
            ("8000x8000", 4096),
            ("3000x3000", 4096),
            ("2500x2501", 4096),
            ("10000x100", 1432),
            ("100x10000", 1432),
            ("30x30", 1),
            ("10x10", 1),
            ("1x1", 1),
            ("5000x1", 179),
        ];
        // OpenRouter (Together) prompt counts: the patches plus the two markers.
        let openrouter = [
            ("64x64", 11),
            ("224x224", 66),
            ("256x256", 102),
            ("448x448", 258),
            ("512x512", 363),
            ("640x480", 393),
            ("800x600", 590),
            ("1024x768", 974),
            ("1280x720", 1198),
            ("1920x1080", 2693),
            ("2000x200", 499),
            ("200x2000", 499),
            ("768x768", 786),
            ("1000x1000", 1298),
        ];
        let size = |s: &str| {
            let (w, h) = s.split_once('x').unwrap();
            (w.parse().unwrap(), h.parse().unwrap())
        };
        for (s, n) in processor {
            let (w, h) = size(s);
            assert_eq!(total(w, h), n + 2, "{s} processor");
        }
        for (s, n) in openrouter {
            let (w, h) = size(s);
            assert_eq!(total(w, h), n, "{s} openrouter");
        }
        assert_eq!(
            layout(&muse(), 64, 64).unwrap(),
            vec![
                (Some("<|image_start|>".to_string()), 1),
                (None, 9),
                (Some("<|image_end|>".to_string()), 1),
            ]
        );
        assert!(layout(&muse(), 0, 10).is_err());
    }

    #[test]
    fn cpython_puts_three_by_three_before_two_by_two_in_a_set() {
        let order = cpython_set_order(&[(2, 2), (2, 3), (3, 2), (3, 3)]);
        let squares: Vec<_> = order.into_iter().filter(|c| c.0 == c.1).collect();
        assert_eq!(squares, vec![(3, 3), (2, 2)]);
    }
}
