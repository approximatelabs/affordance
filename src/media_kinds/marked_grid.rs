//! A Qwen2-VL grid with its own start and end tokens around the cells: MiniMax M3 and M3.1
//! Flash, whose processor (the same code in both repositories, run in Docker, and matched by
//! the hosted servers that run it) replaces the template's one picture token with
//! `start`, the cells, and `end`. The cells are counted by the shared grid rule
//! (`crate::media::fit`, Qwen's `smart_resize`).

use crate::media::{Metrics, ObjectFit, fit};

/// The runs for a picture of `width` by `height`: `start`, the grid's cells, `end`.
/// Params: `cell`, `min`, `max` (pixels, as the grid kind), `start`, `end` (token names).
pub fn layout(params: &serde_json::Value, width: u32, height: u32) -> Result<Vec<(Option<String>, usize)>, String> {
    let number = |key: &str| {
        params[key]
            .as_u64()
            .ok_or_else(|| format!("marked_grid: `{key}` must be a number"))
    };
    let text = |key: &str| {
        params[key]
            .as_str()
            .map(String::from)
            .ok_or_else(|| format!("marked_grid: `{key}` must be a token name"))
    };
    if width == 0 || height == 0 {
        return Err("marked_grid: a picture needs pixels".into());
    }
    let grid = Metrics::Grid {
        cell: number("cell")? as u32,
        min: number("min")?,
        max: number("max")?,
    };
    let placed =
        fit(&grid, (width, height), None, None, ObjectFit::ScaleDown).ok_or("marked_grid: the picture does not fit")?;
    Ok(vec![
        (Some(text("start")?), 1),
        (None, placed.tokens),
        (Some(text("end")?), 1),
    ])
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_minimax_m3_picture_is_laid_out_as_its_processor_lays_it_out() {
        let rule = serde_json::json!({"cell": 28, "min": 3136, "max": 451584, "start": "]<]start of image[>[", "end": "]<]end of image[>["});
        let runs = super::layout(&rule, 1920, 1080).unwrap();
        assert_eq!(runs.first().unwrap().0.as_deref(), Some("]<]start of image[>["));
        assert_eq!(runs.last().unwrap().0.as_deref(), Some("]<]end of image[>["));
        let total = |w, h| super::layout(&rule, w, h).unwrap().iter().map(|r| r.1).sum::<usize>();
        // N cells (the processor's counts, run in Docker) and the two markers.
        for ((w, h), n) in [
            ((64, 64), 4),
            ((224, 224), 64),
            ((256, 256), 81),
            ((448, 448), 256),
            ((512, 512), 324),
            ((640, 480), 391),
            ((800, 600), 540),
            ((1024, 768), 540),
            ((1280, 720), 576),
            ((1920, 1080), 576),
            ((2000, 200), 497),
            ((200, 2000), 497),
            ((768, 768), 576),
            ((1000, 1000), 576),
        ] {
            assert_eq!(total(w, h), n + 2, "{w}x{h}");
        }
    }
}
