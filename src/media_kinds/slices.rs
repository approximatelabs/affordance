/// MiniCPM-V-4.6, MiniCPM-V-4.7 (1B and 35B-A3B) and MiniCPM-OCR-1.0. The processor
/// (transformers `MiniCPMV4_6ImageProcessor` / `MiniCPMOcrImageProcessor`, SGLang 0.5.23's
/// `minicpmv4_6.py`) turns a picture into one downscaled source block and, when the picture
/// covers more than `scale`^2 pixels, a rows x cols grid of equal slices chosen to match the
/// picture's aspect ratio. Each block is a run of placeholder tokens wrapped in markers, the
/// whole picture is prefixed by `<image_id>k</image_id>` (`index`, the picture's place in the
/// prompt, from 0), and grid rows are separated by one newline token (`Ċ`). The reference counts and
/// token-id sequences came from running those processors in docker on 32 sizes per mode.
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let get = |key: &str| -> Result<u64, String> {
        params
            .get(key)
            .and_then(|v| v.as_u64())
            .filter(|&v| v > 0)
            .ok_or_else(|| format!("slices: missing or invalid parameter `{key}`"))
    };
    let patch = get("patch")?;
    let align = get("align")?;
    let divisor = get("divisor")?;
    let scale = get("scale")?;
    let max_slices = get("max_slices")?;
    let image_id = params
        .get("image_id")
        .map(|v| v.as_bool().ok_or("slices: `image_id` must be a boolean"))
        .transpose()?
        .unwrap_or(true);
    let index = params.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
    if width == 0 || height == 0 {
        return Err(format!("slices: a {width}x{height} picture has no pixels"));
    }
    let (w, h) = (u64::from(width), u64::from(height));

    // Python: max(round(x / d) * d, d), round sending halves to the even neighbour.
    let ensure = |x: u64, d: u64| -> u64 {
        let q = (x as f64 / d as f64).round_ties_even() as u64;
        (q * d).max(d)
    };
    let fit = |h: u64, w: u64, up: bool| -> (u64, u64) {
        let (mut h2, mut w2) = (h, w);
        if h * w > scale * scale || up {
            let r = w as f64 / h as f64;
            let nh = (scale as f64 / r.sqrt()).trunc();
            h2 = nh as u64;
            w2 = (nh * r).trunc() as u64;
        }
        (ensure(h2, align), ensure(w2, align))
    };

    // Grid search.
    let grid = {
        let log_ratio = (w as f64 / h as f64).ln();
        let m = (w * h).div_ceil(scale * scale).min(max_slices);
        if m <= 1 {
            None
        } else {
            let mut best: Option<(u64, u64)> = None;
            let mut best_err = f64::INFINITY;
            for n in [m - 1, m, m + 1] {
                if n == 1 || n > max_slices {
                    continue;
                }
                for rows in 1..=n {
                    if n % rows != 0 {
                        continue;
                    }
                    let cols = n / rows;
                    let err = (log_ratio - (cols as f64 / rows as f64).ln()).abs();
                    if err < best_err || (err == best_err && best.is_some_and(|(r, _)| rows > r)) {
                        best = Some((rows, cols));
                        best_err = err;
                    }
                }
            }
            best
        }
    };

    let (sh, sw) = fit(h, w, grid.is_none());
    let src = (sh / patch) * (sw / patch) / divisor;

    let mut out: Vec<(Option<String>, usize)> = Vec::new();
    let mark = |s: &str| (Some(s.to_string()), 1usize);
    if image_id {
        out.push(mark("<image_id>"));
        out.push((Some(format!("text:{index}")), 1));
        out.push(mark("</image_id>"));
    }
    out.push(mark("<image>"));
    out.push((None, src as usize));
    out.push(mark("</image>"));
    if let Some((rows, cols)) = grid {
        let rh = ensure(h, rows) / rows;
        let rw = ensure(w, cols) / cols;
        let (bh, bw) = fit(rh, rw, true);
        let per_slice = (bh / patch) * (bw / patch) / divisor;
        for row in 0..rows {
            if row > 0 {
                out.push(mark("Ċ"));
            }
            for _ in 0..cols {
                out.push(mark("<slice>"));
                out.push((None, per_slice as usize));
                out.push(mark("</slice>"));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::layout;
    use serde_json::json;

    fn total(p: &serde_json::Value, w: u32, h: u32) -> usize {
        layout(p, w, h).unwrap().iter().map(|(_, n)| n).sum()
    }

    fn check(p: serde_json::Value, table: &[(&str, usize)]) {
        for (size, want) in table {
            let (w, h) = size.split_once('x').unwrap();
            let got = total(&p, w.parse().unwrap(), h.parse().unwrap());
            assert_eq!(got, *want, "{size} with {p}");
        }
    }

    #[test]
    fn a_minicpm_picture_is_laid_out_as_its_processor_lays_it_out() {
        let v16 = json!({"patch": 14, "align": 56, "divisor": 16, "scale": 448, "max_slices": 9});
        let ocr = json!({"patch": 14, "align": 28, "divisor": 4, "scale": 448, "max_slices": 9});
        let v4x56 = json!({"patch": 14, "align": 56, "divisor": 4, "scale": 448, "max_slices": 9});
        check(
            v16.clone(),
            &[
                ("64x64", 69),
                ("224x224", 69),
                ("256x256", 69),
                ("448x448", 69),
                ("512x512", 206),
                ("640x480", 212),
                ("800x600", 329),
                ("1024x768", 329),
                ("1280x720", 462),
                ("1920x1080", 600),
                ("2000x200", 266),
                ("200x2000", 268),
                ("768x768", 334),
                ("1000x1000", 334),
                ("1x1", 69),
                ("13x7", 71),
                ("57x57", 69),
                ("100x3000", 282),
                ("3000x100", 279),
                ("449x449", 206),
                ("896x896", 334),
                ("1344x1344", 665),
                ("4000x3000", 655),
                ("3000x4000", 655),
                ("1500x1500", 665),
                ("333x777", 196),
                ("1024x1024", 442),
                ("2048x1536", 655),
                ("50x900", 73),
                ("900x50", 73),
                ("3840x2160", 600),
                ("10000x100", 480),
            ],
        );
        check(
            ocr,
            &[
                ("64x64", 261),
                ("224x224", 261),
                ("256x256", 261),
                ("448x448", 261),
                ("512x512", 772),
                ("640x480", 781),
                ("800x600", 1274),
                ("1024x768", 1274),
                ("1280x720", 1800),
                ("1920x1080", 2314),
                ("2000x200", 1044),
                ("200x2000", 1051),
                ("768x768", 1294),
                ("1000x1000", 1294),
                ("1x1", 261),
                ("13x7", 269),
                ("57x57", 261),
                ("100x3000", 1042),
                ("3000x100", 1022),
                ("449x449", 772),
                ("896x896", 1294),
                ("1344x1344", 2585),
                ("4000x3000", 2545),
                ("3000x4000", 2545),
                ("1500x1500", 2585),
                ("333x777", 760),
                ("1024x1024", 1834),
                ("2048x1536", 2545),
                ("50x900", 277),
                ("900x50", 277),
                ("3840x2160", 2314),
                ("10000x100", 1891),
            ],
        );
        check(
            v4x56,
            &[
                ("64x64", 261),
                ("224x224", 261),
                ("256x256", 261),
                ("448x448", 261),
                ("512x512", 794),
                ("640x480", 821),
                ("800x600", 1274),
                ("1024x768", 1274),
                ("1280x720", 1794),
                ("1920x1080", 2334),
                ("2000x200", 1031),
                ("200x2000", 1033),
                ("768x768", 1294),
                ("1000x1000", 1294),
            ],
        );

        // 1280x720 in 16x mode: a 66-token source block and a 2x3 grid of 63-token slices.
        let m = |s: &str| (Some(s.to_string()), 1);
        let line = [
            m("<slice>"),
            (None, 63),
            m("</slice>"),
            m("<slice>"),
            (None, 63),
            m("</slice>"),
            m("<slice>"),
            (None, 63),
            m("</slice>"),
        ];
        let mut want = vec![
            m("<image_id>"),
            m("text:0"),
            m("</image_id>"),
            m("<image>"),
            (None, 66),
            m("</image>"),
        ];
        want.extend(line.clone());
        want.push(m("Ċ"));
        want.extend(line);
        assert_eq!(layout(&v16, 1280, 720).unwrap(), want);

        // A picture without a grid is only the prefix and the source block.
        assert_eq!(
            layout(&v16, 448, 448).unwrap(),
            vec![
                m("<image_id>"),
                m("text:0"),
                m("</image_id>"),
                m("<image>"),
                (None, 64),
                m("</image>"),
            ]
        );
        assert!(layout(&v16, 0, 10).is_err());
    }
}
