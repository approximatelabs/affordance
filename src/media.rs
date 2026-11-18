//! Media as layout: how many tokens a picture takes, and the file at that size.
//!
//! A model's processor turns a picture into tokens by a rule that is the model's (its
//! [`Metrics`], like a font's). Layout gives the picture a box in tokens (`max-tokens`,
//! `min-tokens`, `object-fit`), [`fit`] finds the size in pixels that takes that many, and
//! [`resized`] is the file at that size. A picture sent at a size the processor leaves alone
//! takes exactly the tokens laid out for it.
//!
//! Everything here is a pure function of the file's bytes and the box: integer arithmetic, no
//! clock, the same bytes on every machine.

use serde_json::Value as J;

/// How a model's processor turns a picture into tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Metrics {
    /// A rule that is one family's own (`src/media_kinds`), with its parameters.
    Named { name: String, params: J },
    /// A canvas of tiles (Llama 4's design): the grid of at most `max_tiles` tiles of `tile`
    /// pixels that fits the picture with the least scaling (or, when none fits, the most), each
    /// tile `per_tile` `patch` tokens, tiles in a row joined by `col_sep` and each row ended by
    /// `row_sep`; then `global` and one more tile's patches for the whole picture; all between
    /// `start` and `end`. One tile is the global view alone.
    Canvas {
        tile: u32,
        max_tiles: u32,
        per_tile: usize,
        start: String,
        end: String,
        patch: String,
        global: String,
        col_sep: String,
        row_sep: String,
    },
    /// Dynamic tiles (InternVL's design): the layout of `min_tiles..=max_tiles` tiles of
    /// `tile` pixels nearest the picture's shape (on a tie, more tiles when the picture is over
    /// half their area, or with `tie_more` always more), `per_tile` cells a tile, one more tile
    /// of cells as a thumbnail when there is more than one, all between `start` and `end`.
    DynTiles {
        tile: u32,
        per_tile: usize,
        min_tiles: u32,
        max_tiles: u32,
        tie_more: bool,
        start: String,
        end: String,
    },
    /// Ovis's grid: like `Grid`, but a side under one cell is raised to a cell first (the other
    /// scaled, Python's round), a picture over 200:1 is cut to 200:1, a picture under `min` is
    /// scaled up with both sides rounded up, and `wrap` more placeholder tokens go around it.
    OvisGrid { cell: u32, min: u64, max: u64, wrap: usize },
    /// Lines of cells (DeepSeek V4.1's design). The picture is brought up to `min_pixels` and
    /// its sides up to whole `patch`-pixel patches; every `downsample` by `downsample` patches
    /// is a cell; the prompt holds a start, each row of cells and a line end, and an end, all as
    /// the placeholder token: `rows * (cols + 1) + 2`. Over `max_tokens` the picture is shrunk
    /// until it fits.
    Lines {
        patch: u32,
        downsample: u32,
        max_tokens: usize,
        min_pixels: u64,
        max_wh_ratio: Option<u32>,
    },
    /// Rows of cells (Pixtral's design). A picture whose longest side is over `longest` pixels
    /// is scaled down to it (sides cut to whole pixels); it is then `cell`-pixel cells, rounded
    /// up, each row followed by `row_end` and the last by `end`.
    Rows {
        cell: u32,
        longest: u32,
        row_end: String,
        end: String,
    },
    /// Tiles (LFM2-VL's design). A picture up to `split_over` pixels is one block on a grid of
    /// `cell` pixels within `min..=max` pixels. A larger one is resized to a grid of tiles of
    /// `tile` pixels (`min_tiles` to `max_tiles` of them, the layout nearest its proportions),
    /// each `marker` (`{r}`, `{c}` filled) and its cells, then `thumbnail` and the picture as
    /// one block. All of it between `start` and `end`; every cell is the placeholder token.
    Tiles {
        cell: u32,
        min: u64,
        max: u64,
        tile: u32,
        min_tiles: u32,
        max_tiles: u32,
        split_over: u64,
        start: String,
        end: String,
        marker: String,
        thumbnail: String,
    },
    /// The picture is cut into cells of `cell` by `cell` pixels, one token each. A picture
    /// whose sides are multiples of `cell` and whose area is within `min..=max` pixels passes
    /// through the processor unchanged.
    Grid { cell: u32, min: u64, max: u64 },
    /// Every picture takes `tokens`, whatever its size.
    Fixed { tokens: usize },
    /// Patches of `patch` pixels, at most `limit` of them (a larger picture is scaled down to
    /// fit), each side padded up to two patches, one token for each 2 by 2 patches (Kimi-VL's
    /// MoonViT: 224 by 224 is 64 tokens, 1280 by 720 is 1,075).
    Patches { patch: u32, limit: u64, side: Option<u32> },
    /// A picture's tokens follow its shape and not its size: the processor resizes it to the
    /// largest grid of its proportions with at most `most` cells (Gemma 4: a square is 16 by
    /// 16, a 4:3 picture 19 by 14).
    Shape { most: u64 },
    /// A sound clip: one token for each `ms` milliseconds (the last one for what is left),
    /// plus `extra` tokens the processor puts around a clip, and at most `most` for one clip.
    Rate { ms: u32, extra: usize, most: Option<usize> },
    /// A video as timed frames (Qwen3-VL's design): the processor takes `fps` frames a second
    /// (at least `min_frames`, at most `max_frames`, an even number), resizes each onto a grid
    /// of `cell` pixels within `min..=max` pixels a frame and `total` pixels for the video, and
    /// writes every two frames as their time (`<1.4 seconds>`), `open`, one token a cell, and
    /// `close`.
    Frames {
        cell: u32,
        min: u64,
        max: u64,
        total: u64,
        fps: u32,
        min_frames: u64,
        max_frames: u64,
        open: String,
        close: String,
    },
}

impl Metrics {
    /// From a model's row (`{"kind": "grid", "cell": 32, "min": 65536, "max": 16777216}`).
    pub fn parse(v: &J) -> Result<Metrics, String> {
        let number = |key: &str| {
            v[key]
                .as_u64()
                .ok_or_else(|| format!("media metrics need a whole number `{key}`: {v}"))
        };
        match v["kind"].as_str() {
            Some("grid") => {
                let (cell, min, max) = (number("cell")?, number("min")?, number("max")?);
                if cell == 0 || cell > 4096 || max < min || max < cell * cell {
                    return Err(format!("media metrics are not a grid a picture can fit: {v}"));
                }
                Ok(Metrics::Grid {
                    cell: cell as u32,
                    min: min.max(cell * cell),
                    max,
                })
            }
            Some("fixed") => Ok(Metrics::Fixed {
                tokens: number("tokens")? as usize,
            }),
            Some("shape") => Ok(Metrics::Shape {
                most: number("most")?.max(1),
            }),
            Some("canvas") => {
                let text = |key: &str| {
                    v[key]
                        .as_str()
                        .map(String::from)
                        .ok_or_else(|| format!("media metrics need `{key}`: {v}"))
                };
                let tile = number("tile")?;
                if tile == 0 {
                    return Err(format!("media metrics need a tile above zero: {v}"));
                }
                Ok(Metrics::Canvas {
                    tile: tile as u32,
                    max_tiles: number("max_tiles")?.max(1) as u32,
                    per_tile: number("per_tile")? as usize,
                    start: text("start")?,
                    end: text("end")?,
                    patch: text("patch")?,
                    global: text("global")?,
                    col_sep: text("col_sep")?,
                    row_sep: text("row_sep")?,
                })
            }
            Some("dyntiles") => {
                let text = |key: &str| v[key].as_str().unwrap_or_default().to_string();
                let tile = number("tile")?;
                if tile == 0 {
                    return Err(format!("media metrics need a tile above zero: {v}"));
                }
                Ok(Metrics::DynTiles {
                    tile: tile as u32,
                    per_tile: number("per_tile")? as usize,
                    min_tiles: number("min_tiles")?.max(1) as u32,
                    max_tiles: number("max_tiles")?.max(1) as u32,
                    tie_more: v["tie"] == "more",
                    start: text("start"),
                    end: text("end"),
                })
            }
            Some("ovis_grid") => {
                let cell = number("cell")?;
                if cell == 0 {
                    return Err(format!("media metrics need a cell above zero: {v}"));
                }
                Ok(Metrics::OvisGrid {
                    cell: cell as u32,
                    min: number("min")?,
                    max: number("max")?,
                    wrap: v["wrap"].as_u64().unwrap_or(0) as usize,
                })
            }
            Some("lines") => {
                let (patch, downsample) = (number("patch")?, number("downsample")?);
                if patch == 0 || downsample == 0 {
                    return Err(format!("media metrics need a patch and a downsample above zero: {v}"));
                }
                Ok(Metrics::Lines {
                    patch: patch as u32,
                    downsample: downsample as u32,
                    max_tokens: number("max_tokens")?.max(3) as usize,
                    min_pixels: number("min_pixels")?,
                    max_wh_ratio: v["max_wh_ratio"].as_u64().map(|r| r as u32),
                })
            }
            Some("rows") => {
                let (cell, longest) = (number("cell")?, number("longest")?);
                if cell == 0 || longest < cell {
                    return Err(format!(
                        "media metrics need a cell above zero within the longest side: {v}"
                    ));
                }
                Ok(Metrics::Rows {
                    cell: cell as u32,
                    longest: longest as u32,
                    row_end: v["row_end"].as_str().unwrap_or_default().into(),
                    end: v["end"].as_str().unwrap_or_default().into(),
                })
            }
            Some("tiles") => {
                let text = |key: &str| {
                    v[key]
                        .as_str()
                        .map(String::from)
                        .ok_or_else(|| format!("media metrics need `{key}`: {v}"))
                };
                let cell = number("cell")?;
                if cell == 0 || number("tile")? == 0 {
                    return Err(format!("media metrics need a cell and a tile above zero: {v}"));
                }
                Ok(Metrics::Tiles {
                    cell: cell as u32,
                    min: number("min")?,
                    max: number("max")?,
                    tile: number("tile")? as u32,
                    min_tiles: number("min_tiles")? as u32,
                    max_tiles: number("max_tiles")? as u32,
                    split_over: number("split_over")?,
                    start: text("start")?,
                    end: text("end")?,
                    marker: text("marker")?,
                    thumbnail: text("thumbnail")?,
                })
            }
            Some("patches") => {
                let patch = number("patch")?;
                if patch == 0 || patch > 1024 {
                    return Err(format!("media metrics need a patch size above zero: {v}"));
                }
                Ok(Metrics::Patches {
                    patch: patch as u32,
                    limit: number("limit")?.max(4),
                    side: v["side"].as_u64().map(|n| n as u32),
                })
            }
            Some("rate") => {
                let ms = number("ms")?;
                if ms == 0 {
                    return Err(format!("media metrics need a rate above zero: {v}"));
                }
                Ok(Metrics::Rate {
                    ms: ms as u32,
                    extra: v["extra"].as_u64().unwrap_or(0) as usize,
                    most: v["most"].as_u64().map(|n| n as usize),
                })
            }
            Some("frames") => {
                let text = |key: &str| {
                    v[key]
                        .as_str()
                        .map(String::from)
                        .ok_or_else(|| format!("media metrics need `{key}`: {v}"))
                };
                let cell = number("cell")?;
                if cell == 0 || number("fps")? == 0 {
                    return Err(format!("media metrics need a cell and a frame rate above zero: {v}"));
                }
                Ok(Metrics::Frames {
                    cell: cell as u32,
                    min: number("min")?,
                    max: number("max")?,
                    total: number("total")?,
                    fps: number("fps")? as u32,
                    min_frames: number("min_frames")?,
                    max_frames: number("max_frames")?,
                    open: text("open")?,
                    close: text("close")?,
                })
            }
            Some(name) if crate::media_kinds::known(name) => Ok(Metrics::Named {
                name: name.to_string(),
                params: v.clone(),
            }),
            _ => Err(format!(
                "media metrics need kind \"grid\", \"fixed\", \"rate\" or \"frames\": {v}"
            )),
        }
    }
}

/// A model's size rules for the files it takes: part of its tokenizer, as its vocabulary is.
/// Text cannot be laid out without a tokenizer; a picture, a clip or a video cannot be laid
/// out without the rule for its kind. A kind with no rule here is not laid out at a guess: the
/// element's alt text stands in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rules {
    pub image: Option<Metrics>,
    pub audio: Option<Metrics>,
    pub video: Option<Metrics>,
}

impl Rules {
    /// The rule for an element (`img`, `audio`, `video`).
    pub fn of(&self, element: &str) -> Option<Metrics> {
        match element {
            "img" => self.image.clone(),
            "audio" => self.audio.clone(),
            "video" => self.video.clone(),
            _ => None,
        }
    }

    /// The rules of the model whose `tokenizer.json` is at `tokenizer`: read from the
    /// processor configuration its publisher ships beside it (`preprocessor_config.json`,
    /// `processor_config.json`), for the processor designs this knows. A tokenizer file on its
    /// own, or a processor of another design, has none.
    pub fn beside(tokenizer: &std::path::Path) -> Rules {
        let read = |name: &str| -> J {
            tokenizer
                .parent()
                .and_then(|dir| std::fs::read_to_string(dir.join(name)).ok())
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(J::Null)
        };
        let (pre, both) = (read("preprocessor_config.json"), read("processor_config.json"));
        let image = if pre.is_object() {
            &pre
        } else {
            &both["image_processor"]
        };
        let mut rules = Rules::default();
        match image["image_processor_type"].as_str().unwrap_or("") {
            // A grid of patch x merge cells, after the picture's sides are rounded to the grid
            // and its area kept within bounds (Qwen2-VL's processor and its descendants).
            "Qwen2VLImageProcessor"
            | "Qwen2VLImageProcessorFast"
            | "Qwen2VLImageProcessorPil"
            | "PaddleOCRVLImageProcessor"
            | "Glm4vImageProcessor"
            | "Glm4vImageProcessorFast"
            | "Glm46VImageProcessor"
            | "Glm46VImageProcessorFast" => {
                let least = image["min_pixels"].as_u64().or(image["size"]["shortest_edge"].as_u64());
                let most = image["max_pixels"].as_u64().or(image["size"]["longest_edge"].as_u64());
                if let (Some(patch), Some(merge), Some(min), Some(max)) =
                    (image["patch_size"].as_u64(), image["merge_size"].as_u64(), least, most)
                {
                    rules.image = Metrics::parse(
                        &serde_json::json!({"kind": "grid", "cell": patch * merge, "min": min, "max": max}),
                    )
                    .ok();
                }
            }
            // A count that follows the picture's shape (measured on a real server, 11 sizes).
            "Gemma4ImageProcessor" | "Gemma4UnifiedImageProcessor" => {
                rules.image = image["max_soft_tokens"].as_u64().map(|most| Metrics::Shape { most });
            }
            // Pixtral's rows (measured on a real server with fourteen sizes).
            "PixtralImageProcessor" | "PixtralImageProcessorFast"
                if both["image_break_token"].is_string() && both["image_end_token"].is_string() =>
            {
                let merge = both["spatial_merge_size"].as_u64().unwrap_or(1);
                if let (Some(patch), Some(longest)) =
                    (image["patch_size"].as_u64(), image["size"]["longest_edge"].as_u64())
                {
                    // Measured on real servers: Pixtral's own models ([IMG]) write a break after
                    // each row and an end; LightOnOCR (<|image_pad|>) writes the cells alone.
                    let plain = both["image_token"] != "[IMG]";
                    rules.image = Some(Metrics::Rows {
                        cell: (patch * merge) as u32,
                        longest: longest as u32,
                        row_end: if plain {
                            String::new()
                        } else {
                            both["image_break_token"].as_str().unwrap_or_default().into()
                        },
                        end: if plain {
                            String::new()
                        } else {
                            both["image_end_token"].as_str().unwrap_or_default().into()
                        },
                    });
                }
            }
            // LFM2-VL's tiles (measured on a real server with fourteen sizes).
            "Lfm2VlImageProcessor" | "Lfm2VlImageProcessorFast"
                if image["do_image_splitting"] != false && image["use_thumbnail"] != false =>
            {
                let n = |key: &str| image[key].as_u64();
                if let (
                    Some(patch),
                    Some(down),
                    Some(least),
                    Some(most),
                    Some(tile),
                    Some(min_tiles),
                    Some(max_tiles),
                    Some(patches),
                ) = (
                    n("encoder_patch_size"),
                    n("downsample_factor"),
                    n("min_image_tokens"),
                    n("max_image_tokens"),
                    n("tile_size"),
                    n("min_tiles"),
                    n("max_tiles"),
                    n("max_num_patches"),
                ) {
                    let cell = patch * down;
                    let tolerance = image["max_pixels_tolerance"].as_f64().unwrap_or(2.0);
                    rules.image = Some(Metrics::Tiles {
                        cell: cell as u32,
                        min: least * cell * cell,
                        max: most * cell * cell,
                        tile: tile as u32,
                        min_tiles: min_tiles as u32,
                        max_tiles: max_tiles as u32,
                        split_over: (patches as f64 * (patch * patch) as f64 * tolerance) as u64,
                        start: "<|image_start|>".into(),
                        end: "<|image_end|>".into(),
                        marker: "<|img_row_{r}_col_{c}|>".into(),
                        thumbnail: "<|img_thumbnail|>".into(),
                    });
                }
            }
            // Kimi-VL's patches (no processor type named; these keys are its).
            "" if image["in_token_limit"].is_u64() && image["patch_size"].is_u64() && image["pad_input"] == true => {
                rules.image = Some(Metrics::Patches {
                    patch: image["patch_size"].as_u64().unwrap_or(14) as u32,
                    limit: image["in_token_limit"].as_u64().unwrap_or(4096),
                    side: None,
                });
            }
            // Gemma 3n: the same count for every picture (measured on a real server).
            _ if both["processor_class"] == "Gemma3nProcessor" && both["image_seq_length"].is_u64() => {
                rules.image = both["image_seq_length"].as_u64().map(|tokens| Metrics::Fixed {
                    tokens: tokens as usize,
                });
            }
            // The same count for every picture (pan-and-scan off, its default).
            "Gemma3ImageProcessor" | "Gemma3ImageProcessorFast" if image["do_pan_and_scan"] != true => {
                let tokens = both["image_seq_length"].as_u64().or(image["image_seq_length"].as_u64());
                rules.image = tokens.map(|tokens| Metrics::Fixed {
                    tokens: tokens as usize,
                });
            }
            _ => {}
        }
        // A video as timed frames, for the model types whose processor writes one that way,
        // with the reference preprocessing's bounds (qwen-vl-utils: 2 frames a second, 4 to
        // 768 of them, 128 to 768 cells of 28 pixels a frame, 115,200 for the video).
        let config = read("config.json");
        if rules.image.is_some()
            && matches!(
                config["model_type"].as_str(),
                Some("qwen3_vl" | "qwen3_vl_moe" | "qwen3_5" | "qwen3_5_moe")
            )
            && let Some(Metrics::Grid { cell, .. }) = rules.image
        {
            rules.video = Some(Metrics::Frames {
                cell,
                min: 128 * 28 * 28,
                max: 768 * 28 * 28,
                total: 90_316_800,
                fps: 2,
                min_frames: 4,
                max_frames: 768,
                open: "<|vision_start|>".into(),
                close: "<|vision_end|>".into(),
            });
        }
        // DeepSeek V4.1's lines of cells, from its vision settings (SGLang's own sizing code).
        let vision = &config["vision_config"];
        if vision["model_type"] == "deepseek_v41_vision" {
            let n = |key: &str| vision[key].as_u64();
            rules.image = Some(Metrics::Lines {
                patch: n("patch_size").unwrap_or(14) as u32,
                downsample: n("downsample_ratio").unwrap_or(3) as u32,
                max_tokens: n("max_image_tokens").unwrap_or(1024) as usize,
                min_pixels: n("min_pixels").unwrap_or(295_936),
                max_wh_ratio: n("max_wh_ratio").map(|r| r as u32),
            });
        }
        // Gemma 4's sound: measured on a real server (40 ms a token, rounded up, 2 around).
        if both["feature_extractor"]["feature_extractor_type"] == "Gemma4AudioFeatureExtractor"
            && let (Some(ms), Some(most)) = (both["audio_ms_per_token"].as_u64(), both["audio_seq_length"].as_u64())
        {
            rules.audio = Some(Metrics::Rate {
                ms: ms as u32,
                extra: 2,
                most: Some(most as usize),
            });
        }
        rules
    }
}

/// How a picture meets its box (`object-fit`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ObjectFit {
    /// Shrink to fit the cap; never enlarge past the natural size.
    #[default]
    ScaleDown,
    /// Shrink or enlarge to the largest size that fits the cap.
    Contain,
    /// The natural size, or not shown when that is over the cap.
    None,
}

/// A picture's place in layout: the tokens it takes and the size it is sent at. `size` is
/// `None` when the file is sent as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub tokens: usize,
    pub size: Option<(u32, u32)>,
}

/// The size a picture of `natural` pixels is sent at and the tokens it takes, under a cap and
/// a floor in tokens. `None`: it cannot be shown within the cap (its `alt` stands in).
pub fn fit(
    metrics: &Metrics,
    natural: (u32, u32),
    max_tokens: Option<usize>,
    min_tokens: Option<usize>,
    object_fit: ObjectFit,
) -> Option<Placed> {
    let (cell, min, max) = match *metrics {
        Metrics::Fixed { tokens } => {
            return max_tokens
                .is_none_or(|cap| tokens <= cap)
                .then_some(Placed { tokens, size: None });
        }
        // The largest whole grid of the picture's proportions within `most` cells: each side
        // is the square root of `most` times that side over the other, rounded down.
        Metrics::Shape { most } => {
            // The processor's own arithmetic (checked against it on 544,137 sizes): the picture
            // scaled to `most` cells of 48 by 48 pixels, each side floored in floating point; a
            // picture too thin for one cell across is one cell by its proportion, at most `most`.
            let (w, h) = (natural.0.max(1) as u64, natural.1.max(1) as u64);
            let f = ((most * 48 * 48) as f64 / (w * h) as f64).sqrt();
            let (mut rows, mut cols) = (
                (f * h as f64 / 48.0).floor() as u64,
                (f * w as f64 / 48.0).floor() as u64,
            );
            if rows == 0 {
                (rows, cols) = (1, (w / h).min(most));
            } else if cols == 0 {
                (cols, rows) = (1, (h / w).min(most));
            }
            let tokens = (rows * cols) as usize;
            return max_tokens
                .is_none_or(|cap| tokens <= cap)
                .then_some(Placed { tokens, size: None });
        }
        // The processor's own count; the file goes as it is. Over a cap it is not shown.
        Metrics::Lines { .. } => {
            let tokens = lines_tokens(metrics, natural);
            return max_tokens
                .is_none_or(|cap| tokens <= cap)
                .then_some(Placed { tokens, size: None });
        }
        // As it is: the processor's own rule. Under a cap: sent on the two-patch grid, inside
        // the patch limit, where the processor leaves it as it is.
        Metrics::Patches { patch, limit, side } => {
            let natural_tokens = patches_tokens(patch, limit, side, natural);
            if max_tokens.is_none_or(|cap| natural_tokens <= cap)
                && min_tokens.is_none_or(|floor| natural_tokens >= floor)
            {
                return Some(Placed {
                    tokens: natural_tokens,
                    size: None,
                });
            }
            let cell = 2 * patch as u64;
            let grid = Metrics::Grid {
                cell: cell as u32,
                min: cell * cell,
                max: limit * patch as u64 * patch as u64,
            };
            let placed = fit(&grid, natural, max_tokens, min_tokens, object_fit)?;
            return Some(Placed {
                tokens: placed.tokens,
                size: Some(placed.size.unwrap_or(natural)),
            });
        }
        Metrics::Grid { cell, min, max } => (cell as u64, min, max),
        // A rate sizes a clip and frames a video; tiles are laid out by `tiles_layout`.
        Metrics::Named { .. }
        | Metrics::Rate { .. }
        | Metrics::Frames { .. }
        | Metrics::Tiles { .. }
        | Metrics::Rows { .. }
        | Metrics::DynTiles { .. }
        | Metrics::Canvas { .. } => {
            return None;
        }
        Metrics::OvisGrid { .. } => {
            let tokens = ovis_tokens(metrics, natural);
            return max_tokens
                .is_none_or(|cap| tokens <= cap)
                .then_some(Placed { tokens, size: None });
        }
    };
    let (w0, h0) = (natural.0.max(1) as u64, natural.1.max(1) as u64);
    let cells = |w: u64, h: u64| (w / cell) * (h / cell);
    // Python's round(): a half goes to the even neighbour (720 / 32 = 22.5 is 22 cells).
    let multiple = |n: u64| {
        let (whole, rest) = (n / cell, n % cell);
        let up = 2 * rest > cell || (2 * rest == cell && whole % 2 == 1);
        (whole + u64::from(up)).max(1) * cell
    };
    // The largest size of this picture's shape, on the grid, with at most `area` pixels:
    // both sides scaled by sqrt(area / natural area) and rounded down, at least one cell.
    let within = |area: u64| -> (u64, u64) {
        let scale = |side: u64, other: u64| {
            // side * sqrt(area / (side * other)) = sqrt(side * area / other)
            let exact = isqrt(side as u128 * area as u128 / other as u128);
            (exact / cell).max(1) * cell
        };
        let (mut w, mut h) = (scale(w0, h0), scale(h0, w0));
        while w * h > area && (w > cell || h > cell) {
            if w >= h && w > cell { w -= cell } else { h -= cell }
        }
        (w, h)
    };
    // The natural size: the processor's own smart_resize, in its floating point. Each side to
    // the nearest cell (Python's round, at least one cell); over `max` pixels, both scaled down
    // and floored to the grid; under `min`, both scaled up and rounded up (nothing taken back).
    let (mut w, mut h) = (multiple(w0), multiple(h0));
    let c = cell as f64;
    if w * h > max {
        let beta = ((w0 * h0) as f64 / max as f64).sqrt();
        (w, h) = (
            (((w0 as f64 / beta) / c).floor() as u64).max(1) * cell,
            (((h0 as f64 / beta) / c).floor() as u64).max(1) * cell,
        );
    } else if w * h < min {
        let beta = (min as f64 / (w0 * h0) as f64).sqrt();
        (w, h) = (
            (((w0 as f64 * beta) / c).ceil() as u64) * cell,
            (((h0 as f64 * beta) / c).ceil() as u64) * cell,
        );
    }
    let smallest = at_least(w0.min(cell), h0.min(cell), min, cell);
    let smallest = if w * h <= smallest.0 * smallest.1 {
        (w, h)
    } else {
        smallest
    };
    if let Some(cap) = max_tokens.map(|c| c as u64) {
        if cells(w, h) > cap {
            if object_fit == ObjectFit::None || cells(smallest.0, smallest.1) > cap {
                return None;
            }
            (w, h) = within((cap * cell * cell).min(max));
            if w * h < min {
                // The shape cannot be kept inside both the cap and the bounds: the smallest.
                (w, h) = smallest;
            }
            if cells(w, h) > cap {
                return None;
            }
        } else if object_fit == ObjectFit::Contain {
            let (cw, ch) = within((cap * cell * cell).min(max));
            if cells(cw, ch) > cells(w, h) {
                (w, h) = (cw, ch);
            }
        }
    }
    if let Some(floor) = min_tokens.map(|f| f as u64)
        && cells(w, h) < floor
    {
        let most = max_tokens.map_or(max, |cap| (cap as u64 * cell * cell).min(max));
        let (fw, fh) = at_least(w0, h0, (floor * cell * cell).min(most), cell);
        if fw * fh <= most {
            (w, h) = (fw, fh);
        }
    }
    let size = (w as u32, h as u32);
    Some(Placed {
        tokens: cells(w, h) as usize,
        size: (size != natural).then_some(size),
    })
}

/// A clip's place in layout: the tokens it takes, and the milliseconds of it that are sent
/// (`None`: all of it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedClip {
    pub tokens: usize,
    pub keep_ms: Option<u64>,
}

/// The tokens a clip of `length_ms` takes under a cap, and how much of it is kept: a clip
/// over its cap is trimmed to the longest piece that fits. `None`: not even one token's
/// worth fits (its `alt` stands in).
pub fn fit_clip(metrics: &Metrics, length_ms: u64, max_tokens: Option<usize>) -> Option<PlacedClip> {
    let Metrics::Rate { ms, extra, most } = *metrics else {
        return None;
    };
    let natural = length_ms.div_ceil(ms as u64).max(1) as usize;
    // The processor itself keeps no more than `most` of a clip.
    let allowed = [most, max_tokens.map(|cap| cap.saturating_sub(extra))]
        .into_iter()
        .flatten()
        .min();
    match allowed {
        Some(0) => None,
        Some(allowed) if natural > allowed => Some(PlacedClip {
            tokens: allowed + extra,
            keep_ms: Some(allowed as u64 * ms as u64),
        }),
        _ => Some(PlacedClip {
            tokens: natural + extra,
            keep_ms: None,
        }),
    }
}

/// What a video file says about itself: its frames, its frames a second, and a frame's size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Video {
    pub frames: u64,
    pub fps: f64,
    pub width: u32,
    pub height: u32,
}

/// A video's place in layout: for every two frames the processor takes, the time written
/// before them; and the tokens one such pair takes.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedVideo {
    pub stamps: Vec<String>,
    pub cells: usize,
}

/// How the processor lays a video out: which frames it takes, the size it gives them, and the
/// time it writes before each pair. `None`: a video it would refuse.
pub fn fit_video(metrics: &Metrics, video: Video) -> Option<PlacedVideo> {
    let Metrics::Frames {
        cell,
        min,
        max,
        total,
        fps,
        min_frames,
        max_frames,
        ..
    } = metrics
    else {
        return None;
    };
    let (cell, frames) = (*cell as f64, video.frames);
    if frames < 2 || video.fps <= 0.0 || video.width == 0 || video.height == 0 {
        return None;
    }
    // Python's round(): halves go to the even neighbour.
    let round = |x: f64| {
        let (down, part) = (x.floor(), x - x.floor());
        if part > 0.5 || (part == 0.5 && down % 2.0 != 0.0) {
            down + 1.0
        } else {
            down
        }
    };
    // How many frames: `fps` a second, within the least and the most, an even number.
    let least = (*min_frames).div_ceil(2) * 2;
    let most = (*max_frames).min(frames) / 2 * 2;
    let wanted = frames as f64 / video.fps * *fps as f64;
    let count = ((wanted.max(least as f64).min(most as f64).min(frames as f64) / 2.0).floor() * 2.0) as u64;
    if count < 2 || count > frames {
        return None;
    }
    // Which frames: evenly from the first to the last (numpy's linspace, cut to whole numbers,
    // each once).
    let step = (frames - 1) as f64 / (count - 1) as f64;
    let mut taken: Vec<u64> = (0..count)
        .map(|i| {
            if i == count - 1 {
                frames - 1
            } else {
                (i as f64 * step) as u64
            }
        })
        .collect();
    taken.dedup();
    // The size of a frame: its sides on the grid, its area within the least and the most (the
    // most is also the video's share for each pair of frames).
    let (h, w) = (video.height as f64, video.width as f64);
    if h.max(w) / h.min(w) > 200.0 {
        return None;
    }
    let most_pixels = (*max as f64)
        .min(*total as f64 / taken.len() as f64 * 2.0)
        .max((*min as f64 * 1.05).floor());
    let (mut hb, mut wb) = (cell.max(round(h / cell) * cell), cell.max(round(w / cell) * cell));
    if hb * wb > most_pixels {
        let beta = (h * w / most_pixels).sqrt();
        (hb, wb) = ((h / beta / cell).floor() * cell, (w / beta / cell).floor() * cell);
    } else if hb * wb < *min as f64 {
        let beta = (*min as f64 / (h * w)).sqrt();
        (hb, wb) = ((h * beta / cell).ceil() * cell, (w * beta / cell).ceil() * cell);
    }
    // The time before each pair: the middle of its two frames' times (a last frame alone is
    // paired with itself), to a tenth of a second.
    let times: Vec<f64> = taken.iter().map(|&i| i as f64 / video.fps).collect();
    let stamps = times
        .chunks(2)
        .map(|pair| format!("<{:.1} seconds>", (pair[0] + pair[pair.len() - 1]) / 2.0))
        .collect();
    Some(PlacedVideo {
        stamps,
        cells: ((hb / cell) * (wb / cell)) as usize,
    })
}

/// A video's frames, frame rate and size, read from an MP4 or MOV file's own tables.
pub fn video_info(src: &str) -> Result<Video, String> {
    let data = bytes(src)?;
    let bad = || "a video laid out by its frames must be an MP4 or MOV file whose tables can be read".to_string();
    // The boxes directly inside `data[from..to]`: (type, payload start, payload end).
    let boxes = |from: usize, to: usize| -> Vec<([u8; 4], usize, usize)> {
        let (mut at, mut found) = (from, vec![]);
        while at + 8 <= to {
            let size = u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize;
            let kind = [data[at + 4], data[at + 5], data[at + 6], data[at + 7]];
            let (head, size) = match size {
                1 if at + 16 <= to => (
                    16,
                    u64::from_be_bytes(data[at + 8..at + 16].try_into().unwrap()) as usize,
                ),
                0 => (8, to - at),
                n => (8, n),
            };
            if size < head || at + size > to {
                break;
            }
            found.push((kind, at + head, at + size));
            at += size;
        }
        found
    };
    let inside =
        |from: usize, to: usize, kind: &[u8; 4]| boxes(from, to).into_iter().find(|b| &b.0 == kind).map(|b| (b.1, b.2));
    let be32 = |at: usize| {
        data.get(at..at + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    let (moov_from, moov_to) = inside(0, data.len(), b"moov").ok_or_else(bad)?;
    for (kind, from, to) in boxes(moov_from, moov_to) {
        if &kind != b"trak" {
            continue;
        }
        let Some((mdia_from, mdia_to)) = inside(from, to, b"mdia") else {
            continue;
        };
        let is_video =
            inside(mdia_from, mdia_to, b"hdlr").is_some_and(|(at, _)| data.get(at + 8..at + 12) == Some(b"vide"));
        if !is_video {
            continue;
        }
        // The track's clock (ticks a second) and its length in ticks.
        let (mdhd, _) = inside(mdia_from, mdia_to, b"mdhd").ok_or_else(bad)?;
        let (timescale, duration) = if data.get(mdhd) == Some(&1) {
            let ticks = data.get(mdhd + 24..mdhd + 32).ok_or_else(bad)?;
            (
                be32(mdhd + 20).ok_or_else(bad)?,
                u64::from_be_bytes(ticks.try_into().unwrap()),
            )
        } else {
            (
                be32(mdhd + 12).ok_or_else(bad)?,
                be32(mdhd + 16).ok_or_else(bad)? as u64,
            )
        };
        // Its frames: the sample-size table's count.
        let (minf_from, minf_to) = inside(mdia_from, mdia_to, b"minf").ok_or_else(bad)?;
        let (stbl_from, stbl_to) = inside(minf_from, minf_to, b"stbl").ok_or_else(bad)?;
        let (stsz, _) = inside(stbl_from, stbl_to, b"stsz").ok_or_else(bad)?;
        let frames = be32(stsz + 8).ok_or_else(bad)? as u64;
        // A frame's size: the track header's last two fields, in 16.16 fixed point.
        let (_, tkhd_to) = inside(from, to, b"tkhd").ok_or_else(bad)?;
        let (width, height) = (
            be32(tkhd_to - 8).ok_or_else(bad)? >> 16,
            be32(tkhd_to - 4).ok_or_else(bad)? >> 16,
        );
        if frames == 0 || duration == 0 || timescale == 0 {
            return Err(bad());
        }
        return Ok(Video {
            frames,
            fps: frames as f64 * timescale as f64 / duration as f64,
            width,
            height,
        });
    }
    Err(bad())
}

/// A PCM WAV file's layout: samples a second, bytes in one frame (all channels), and where
/// its samples are.
struct Wav {
    rate: u32,
    frame: usize,
    channels: u16,
    bits: u16,
    at: usize,
    len: usize,
}

fn wav(data: &[u8]) -> Result<Wav, String> {
    let bad = || "a sound clip sized by layout must be a PCM WAV file".to_string();
    if !(data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WAVE")) {
        return Err(bad());
    }
    let (mut at, mut format) = (12, None);
    while let Some(head) = data.get(at..at + 8) {
        let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
        let body = at + 8;
        match &head[..4] {
            b"fmt " => {
                let f = data.get(body..body + 16).ok_or_else(bad)?;
                let le16 = |i: usize| u16::from_le_bytes([f[i], f[i + 1]]);
                // 1: PCM. 0xFFFE: the extensible form of it.
                if !matches!(le16(0), 1 | 0xFFFE) {
                    return Err(bad());
                }
                format = Some((le16(2), u32::from_le_bytes([f[4], f[5], f[6], f[7]]), le16(14)));
            }
            b"data" => {
                let (channels, rate, bits) = format.ok_or_else(bad)?;
                let frame = channels as usize * (bits as usize / 8);
                if frame == 0 || rate == 0 {
                    return Err(bad());
                }
                return Ok(Wav {
                    rate,
                    frame,
                    channels,
                    bits,
                    at: body,
                    len: len.min(data.len().saturating_sub(body)),
                });
            }
            _ => {}
        }
        at = body + len + (len & 1);
    }
    Err(bad())
}

/// A WAV clip's length in milliseconds, rounded up.
pub fn clip_length_ms(src: &str) -> Result<u64, String> {
    let data = bytes(src)?;
    let w = wav(&data)?;
    Ok(((w.len / w.frame) as u64 * 1000).div_ceil(w.rate as u64))
}

/// `keep_ms` of a WAV clip, from its start or (`from_end`) its end, as a WAV data URL.
pub fn clipped(src: &str, keep_ms: u64, from_end: bool) -> Result<String, String> {
    let data = bytes(src)?;
    let w = wav(&data)?;
    let frames = (w.len / w.frame).min((keep_ms * w.rate as u64 / 1000) as usize);
    let start = if from_end {
        w.at + w.len / w.frame * w.frame - frames * w.frame
    } else {
        w.at
    };
    let samples = &data[start..start + frames * w.frame];
    let mut out = Vec::with_capacity(44 + samples.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + samples.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&w.channels.to_le_bytes());
    out.extend_from_slice(&w.rate.to_le_bytes());
    out.extend_from_slice(&(w.rate * w.frame as u32).to_le_bytes());
    out.extend_from_slice(&(w.frame as u16).to_le_bytes());
    out.extend_from_slice(&w.bits.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    out.extend_from_slice(samples);
    Ok(format!("data:audio/wav;base64,{}", base64_encode(&out)))
}

/// One piece of a tiled picture's layout: a named marker token, or a run of cells (each the
/// placeholder token).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Marker(String),
    Cells(usize),
    /// A run of a named token other than the placeholder (Llama 4's `<|patch|>`).
    Run(String, usize),
}

/// A tiled picture's layout (LFM2-VL's processor, measured on a real server) and the size it
/// is sent at (`None`: as it is). Under a cap it fits in, it is laid out as the processor lays
/// out the file. Over its cap it is sent as one block at a size that fits (a picture that small
/// is never cut into tiles). `None`: it cannot fit.
pub fn tiles_layout(
    metrics: &Metrics,
    natural: (u32, u32),
    max_tokens: Option<usize>,
) -> Option<(Vec<Piece>, Option<(u32, u32)>)> {
    let Metrics::Tiles {
        cell,
        min,
        max,
        tile,
        min_tiles,
        max_tiles,
        split_over,
        start,
        end,
        marker,
        thumbnail,
    } = metrics
    else {
        return None;
    };
    let block = Metrics::Grid {
        cell: *cell,
        min: *min,
        max: *max,
    };
    let tokens = |pieces: &[Piece]| pieces.iter().map(piece_tokens).sum::<usize>();
    let (w, h) = (natural.0.max(1) as u64, natural.1.max(1) as u64);
    let mut pieces = vec![Piece::Marker(start.clone())];
    if w * h > *split_over {
        // The tile layout nearest the picture's proportions, among those with min..=max tiles,
        // fewer tiles first; on a tie, more tiles when the picture is over half their area.
        let mut layouts: Vec<(u64, u64)> = vec![];
        for n in *min_tiles as u64..=*max_tiles as u64 {
            for cols in 1..=n {
                for rows in 1..=n {
                    let count = cols * rows;
                    if count >= *min_tiles as u64 && count <= *max_tiles as u64 && !layouts.contains(&(cols, rows)) {
                        layouts.push((cols, rows));
                    }
                }
            }
        }
        layouts.sort_by_key(|&(c, r)| c * r);
        let aspect = w as f64 / h as f64;
        let side = *tile as u64;
        let (mut best, mut best_diff) = ((1u64, 1u64), f64::INFINITY);
        for &(cols, rows) in &layouts {
            let diff = (aspect - cols as f64 / rows as f64).abs();
            if diff < best_diff {
                (best, best_diff) = ((cols, rows), diff);
            } else if diff == best_diff && (w * h) as f64 > 0.5 * (side * side * cols * rows) as f64 {
                best = (cols, rows);
            }
        }
        let per_tile = ((side / *cell as u64) * (side / *cell as u64)) as usize;
        for r in 1..=best.1 {
            for c in 1..=best.0 {
                pieces.push(Piece::Marker(
                    marker.replace("{r}", &r.to_string()).replace("{c}", &c.to_string()),
                ));
                pieces.push(Piece::Cells(per_tile));
            }
        }
        pieces.push(Piece::Marker(thumbnail.clone()));
        pieces.push(Piece::Cells(
            fit(&block, natural, None, None, ObjectFit::ScaleDown)?.tokens,
        ));
        pieces.push(Piece::Marker(end.clone()));
        if max_tokens.is_none_or(|cap| tokens(&pieces) <= cap) {
            return Some((pieces, None));
        }
        pieces.truncate(1);
    }
    // One block: as it is, or, over a cap, shrunk on the grid (2 tokens are the markers).
    let cap = max_tokens.map(|cap| cap.saturating_sub(2));
    if cap == Some(0) {
        return None;
    }
    let placed = fit(&block, natural, cap, None, ObjectFit::ScaleDown)?;
    let mut size = placed.size;
    // A picture shrunk from a tiled one is sent at its block size, so it is not tiled again.
    if w * h > *split_over && size.is_none() {
        size = Some(natural);
    }
    pieces.push(Piece::Cells(placed.tokens));
    pieces.push(Piece::Marker(end.clone()));
    Some((pieces, size))
}

/// A picture in rows of cells (Pixtral's processor, measured on a real server): its layout and
/// the size it is sent at (`None`: as it is). Over a cap it is sent smaller, on the cell grid,
/// at the largest size that fits. `None`: not even one cell fits.
pub fn rows_layout(
    metrics: &Metrics,
    natural: (u32, u32),
    max_tokens: Option<usize>,
) -> Option<(Vec<Piece>, Option<(u32, u32)>)> {
    let Metrics::Rows {
        cell,
        longest,
        row_end,
        end,
    } = metrics
    else {
        return None;
    };
    let cell = *cell as u64;
    let layout = |w: u64, h: u64| -> Vec<Piece> {
        let (cols, rows) = (w.div_ceil(cell).max(1), h.div_ceil(cell).max(1));
        let mut pieces = vec![];
        for r in 0..rows {
            pieces.push(Piece::Cells(cols as usize));
            // A processor that writes no break or end token (LightOnOCR's) has them empty.
            let marker = if r + 1 == rows { end } else { row_end };
            if !marker.is_empty() {
                pieces.push(Piece::Marker(marker.clone()));
            }
        }
        pieces
    };
    let tokens = |pieces: &[Piece]| pieces.iter().map(piece_tokens).sum::<usize>();
    // As the processor takes it: its longest side brought down to `longest`, cut to whole
    // pixels.
    let (w0, h0) = (natural.0.max(1) as u64, natural.1.max(1) as u64);
    let ratio = w0.max(h0) as f64 / *longest as f64;
    let (w, h) = if ratio > 1.0 {
        (((w0 as f64) / ratio) as u64, ((h0 as f64) / ratio) as u64)
    } else {
        (w0, h0)
    };
    let pieces = layout(w.max(1), h.max(1));
    if max_tokens.is_none_or(|cap| tokens(&pieces) <= cap) {
        return Some((pieces, None));
    }
    let cap = max_tokens.unwrap_or(usize::MAX);
    // Smaller, on the grid: the largest whole number of cells across that fits, keeping the
    // shape as closely as the grid allows.
    let mut best = None;
    let mut cols = w.div_ceil(cell);
    while cols >= 1 {
        let rows = ((h as f64 * cols as f64 / w as f64).round() as u64).max(1);
        let pieces = layout(cols * cell, rows * cell);
        if tokens(&pieces) <= cap {
            best = Some((pieces, Some(((cols * cell) as u32, (rows * cell) as u32))));
            break;
        }
        cols -= 1;
    }
    best
}

/// A picture on a canvas of tiles (Llama 4's processor, checked against its code and two hosted
/// servers): its layout. `None` over a cap: it is not shown.
pub fn canvas_layout(
    metrics: &Metrics,
    natural: (u32, u32),
    max_tokens: Option<usize>,
) -> Option<(Vec<Piece>, Option<(u32, u32)>)> {
    let Metrics::Canvas {
        tile,
        max_tiles,
        per_tile,
        start,
        end,
        patch,
        global,
        col_sep,
        row_sep,
    } = metrics
    else {
        return None;
    };
    let (w, h, t) = (natural.0.max(1) as f64, natural.1.max(1) as f64, *tile as f64);
    // Every grid's scale; the smallest that is at least 1, else the largest; on a tie, fewer tiles.
    let mut grids: Vec<(f64, u32, u32)> = vec![];
    for rows in 1..=*max_tiles {
        for cols in 1..=*max_tiles {
            if rows * cols <= *max_tiles {
                grids.push(((cols as f64 * t / w).min(rows as f64 * t / h), rows, cols));
            }
        }
    }
    let fitting: Vec<_> = grids.iter().filter(|g| g.0 >= 1.0).copied().collect();
    let pick = |list: &[(f64, u32, u32)], smallest: bool| {
        let target = list
            .iter()
            .map(|g| g.0)
            .fold(if smallest { f64::INFINITY } else { f64::NEG_INFINITY }, |a, b| {
                if smallest { a.min(b) } else { a.max(b) }
            });
        list.iter().filter(|g| g.0 == target).min_by_key(|g| g.1 * g.2).copied()
    };
    let (_, rows, cols) = if fitting.is_empty() {
        pick(&grids, false)?
    } else {
        pick(&fitting, true)?
    };
    let mut pieces = vec![Piece::Marker(start.clone())];
    if rows * cols > 1 {
        for _ in 0..rows {
            for c in 0..cols {
                if c > 0 {
                    pieces.push(Piece::Marker(col_sep.clone()));
                }
                pieces.push(Piece::Run(patch.clone(), *per_tile));
            }
            pieces.push(Piece::Marker(row_sep.clone()));
        }
    }
    pieces.push(Piece::Marker(global.clone()));
    pieces.push(Piece::Run(patch.clone(), *per_tile));
    pieces.push(Piece::Marker(end.clone()));
    let total: usize = pieces.iter().map(piece_tokens).sum();
    max_tokens.is_none_or(|cap| total <= cap).then_some((pieces, None))
}

/// A picture under a named rule (`src/media_kinds`): its layout, as pieces. `None` over a cap:
/// it is not shown.
pub fn named_layout(
    metrics: &Metrics,
    natural: (u32, u32),
    max_tokens: Option<usize>,
) -> Result<Option<Vec<Piece>>, String> {
    let Metrics::Named { name, params } = metrics else {
        return Ok(None);
    };
    let runs = crate::media_kinds::layout(name, params, natural.0.max(1), natural.1.max(1))?;
    let pieces: Vec<Piece> = runs
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .map(|(token, n)| match token {
            None => Piece::Cells(n),
            Some(name) if n == 1 => Piece::Marker(name),
            Some(name) => Piece::Run(name, n),
        })
        .collect();
    let total: usize = pieces.iter().map(piece_tokens).sum();
    Ok(max_tokens.is_none_or(|cap| total <= cap).then_some(pieces))
}

/// How many tokens a piece of a layout is.
pub fn piece_tokens(piece: &Piece) -> usize {
    match piece {
        Piece::Marker(_) => 1,
        Piece::Cells(n) | Piece::Run(_, n) => *n,
    }
}

/// Python's round() of `x`: a half goes to the even neighbour.
fn round_even(x: f64) -> f64 {
    let down = x.floor();
    let part = x - down;
    if part > 0.5 || (part == 0.5 && down % 2.0 != 0.0) {
        down + 1.0
    } else {
        down
    }
}

/// The tokens Ovis's processor gives a picture (its smart_resize, checked against its code),
/// with the wrapping tokens.
pub fn ovis_tokens(metrics: &Metrics, size: (u32, u32)) -> usize {
    let Metrics::OvisGrid { cell, min, max, wrap } = *metrics else {
        return 0;
    };
    let c = cell as f64;
    let (mut w, mut h) = (size.0.max(1) as f64, size.1.max(1) as f64);
    if w < c || h < c {
        if w < h {
            (h, w) = (round_even(c / w * h), c);
        } else {
            (w, h) = (round_even(c / h * w), c);
        }
    } else if w.max(h) / w.min(h) > 200.0 {
        if w > h { w = 200.0 * h } else { h = 200.0 * w }
    }
    let (mut gw, mut gh) = (round_even(w / c) * c, round_even(h / c) * c);
    if gw * gh > max as f64 {
        let beta = (w * h / max as f64).sqrt();
        (gw, gh) = ((w / beta / c).floor() * c, (h / beta / c).floor() * c);
    } else if gw * gh < min as f64 {
        let beta = (min as f64 / (w * h)).sqrt();
        (gw, gh) = ((w * beta / c).ceil() * c, (h * beta / c).ceil() * c);
    }
    ((gw / c) * (gh / c)) as usize + wrap
}

/// A picture in dynamic tiles (InternVL's processor, checked against its code): its layout.
/// `None` over a cap: it is not shown.
pub fn dyntiles_layout(
    metrics: &Metrics,
    natural: (u32, u32),
    max_tokens: Option<usize>,
) -> Option<(Vec<Piece>, Option<(u32, u32)>)> {
    let Metrics::DynTiles {
        tile,
        per_tile,
        min_tiles,
        max_tiles,
        tie_more,
        start,
        end,
    } = metrics
    else {
        return None;
    };
    let (w, h) = (natural.0.max(1) as u64, natural.1.max(1) as u64);
    let mut layouts: Vec<(u64, u64)> = vec![];
    for n in *min_tiles as u64..=*max_tiles as u64 {
        for cols in 1..=n {
            for rows in 1..=n {
                let count = cols * rows;
                if count >= *min_tiles as u64 && count <= *max_tiles as u64 && !layouts.contains(&(cols, rows)) {
                    layouts.push((cols, rows));
                }
            }
        }
    }
    layouts.sort_by_key(|&(c, r)| c * r);
    let aspect = w as f64 / h as f64;
    let side = *tile as u64;
    let (mut best, mut best_diff) = ((1u64, 1u64), f64::INFINITY);
    for &(cols, rows) in &layouts {
        let diff = (aspect - cols as f64 / rows as f64).abs();
        if diff < best_diff {
            (best, best_diff) = ((cols, rows), diff);
        } else if diff == best_diff {
            let more = if *tie_more {
                cols * rows > best.0 * best.1
            } else {
                (w * h) as f64 > 0.5 * (side * side * cols * rows) as f64
            };
            if more {
                best = (cols, rows);
            }
        }
    }
    let n = best.0 * best.1;
    let pieces_count = n + u64::from(n > 1);
    let mut pieces = vec![];
    if !start.is_empty() {
        pieces.push(Piece::Marker(start.clone()));
    }
    pieces.push(Piece::Cells(*per_tile * pieces_count as usize));
    if !end.is_empty() {
        pieces.push(Piece::Marker(end.clone()));
    }
    let total: usize = pieces.iter().map(piece_tokens).sum();
    max_tokens.is_none_or(|cap| total <= cap).then_some((pieces, None))
}

/// The tokens DeepSeek V4.1's processor gives a picture, as SGLang's `plan_image_grid` and
/// `num_image_tokens` compute them.
pub fn lines_tokens(metrics: &Metrics, size: (u32, u32)) -> usize {
    let Metrics::Lines {
        patch,
        downsample,
        max_tokens,
        min_pixels,
        max_wh_ratio,
    } = *metrics
    else {
        return 0;
    };
    let (p, d) = (patch as f64, downsample as f64);
    let (mut w, mut h) = (size.0.max(1) as f64, size.1.max(1) as f64);
    if let Some(r) = max_wh_ratio
        && w > h * r as f64
    {
        w = h * r as f64;
    }
    if w * h > 0.0 && w * h < min_pixels as f64 {
        let ratio = (min_pixels as f64 / (w * h)).sqrt();
        (w, h) = ((w * ratio).trunc(), (h * ratio).trunc());
    }
    let grid = |best_h: f64, best_w: f64| (((best_h / p).floor() / d).ceil(), ((best_w / p).floor() / d).ceil());
    let count = |(rows, cols): (f64, f64)| (rows * (cols + 1.0) + 2.0) as usize;
    let (mut best_w, mut best_h) = ((w / p).ceil() * p, (h / p).ceil() * p);
    if count(grid(best_h, best_w)) > max_tokens {
        // The largest picture of its shape whose grid fits (SGLang's `solve_resize_ratio`).
        let ratio = h / w;
        let most = max_tokens as f64;
        let max_w = ((most - 2.0) / ratio + 0.25).sqrt() - 0.5;
        let max_h = max_w * ratio;
        let cell = p * d;
        (best_h, best_w) = if max_w < 1.0 {
            (((max_tokens - 2) / 2) as f64 * cell, cell)
        } else if max_h < 1.0 {
            (cell, (max_tokens - 3) as f64 * cell)
        } else {
            let beta = (max_w.floor() * cell / w).min(max_h.floor() * cell / h);
            ((h * beta / p).floor() * p, (w * beta / p).floor() * p)
        };
    }
    count(grid(best_h, best_w))
}

/// The tokens a patch processor (Kimi-VL's) gives a picture of `size`, as its reference code
/// computes them: patches counted by whole divisions, the picture scaled down by the square
/// root of the excess when there are too many (sides cut to whole pixels), each side padded up
/// to two patches, and one token for each 2 by 2 patches.
pub fn patches_tokens(patch: u32, limit: u64, side: Option<u32>, size: (u32, u32)) -> usize {
    let p = patch as u64;
    let (w0, h0) = (size.0.max(1) as u64, size.1.max(1) as u64);
    let count = (w0 / p).max(1) * (h0 / p).max(1);
    let mut scale: f64 = if count > limit {
        (limit as f64 / count as f64).sqrt()
    } else {
        1.0
    };
    // Kimi K2.x also caps each side at `side` patches.
    if let Some(side) = side {
        let most = (side as u64 * p) as f64;
        scale = scale.min(most / w0 as f64).min(most / h0 as f64);
    }
    let (mut w, mut h) = (w0, h0);
    if scale < 1.0 {
        (w, h) = (
            (((w0 as f64) * scale) as u64).max(1),
            (((h0 as f64) * scale) as u64).max(1),
        );
    }
    if let Some(side) = side {
        (w, h) = (w.min(side as u64 * p), h.min(side as u64 * p));
    }
    let cell = 2 * p;
    (w.div_ceil(cell) * h.div_ceil(cell)) as usize
}

/// The smallest size of the shape `w0` by `h0`, on the grid, with at least `area` pixels.
fn at_least(w0: u64, h0: u64, area: u64, cell: u64) -> (u64, u64) {
    let scale = |side: u64, other: u64| {
        let exact = isqrt((side as u128 * area as u128).div_ceil(other as u128));
        exact.div_ceil(cell).max(1) * cell
    };
    let (mut w, mut h) = (scale(w0, h0), scale(h0, w0));
    // Rounding both sides up can overshoot by a row or a column: take back what is spare.
    loop {
        if w >= h && w > cell && (w - cell) * h >= area {
            w -= cell;
        } else if h > cell && w * (h - cell) >= area {
            h -= cell;
        } else if w > cell && (w - cell) * h >= area {
            w -= cell;
        } else {
            return (w, h);
        }
    }
}

/// The whole-number square root, rounded down.
fn isqrt(n: u128) -> u64 {
    if n < 2 {
        return n as u64;
    }
    let mut x = (n as f64).sqrt() as u128;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x as u64
}

/// A source's width and height in pixels. Read once for a source: a history's pictures are
/// laid out again on every render.
pub fn natural_size(src: &str) -> Result<(u32, u32), String> {
    thread_local! {
        static SIZES: std::cell::RefCell<std::collections::HashMap<(usize, u64), (u32, u32)>> = Default::default();
    }
    let key = (src.len(), fingerprint(src));
    if let Some(size) = SIZES.with(|sizes| sizes.borrow().get(&key).copied()) {
        return Ok(size);
    }
    let size = dimensions(&bytes(src)?)?;
    SIZES.with(|sizes| {
        let mut sizes = sizes.borrow_mut();
        if sizes.len() >= 4096 {
            sizes.clear();
        }
        sizes.insert(key, size);
    });
    Ok(size)
}

/// The source at `size`, as a PNG data URL. Kept for the last sources asked for: a history's
/// pictures go out again with every request.
pub fn at_size(src: &str, size: (u32, u32)) -> Result<String, String> {
    type Key = (usize, u64, u32, u32);
    thread_local! {
        static RESIZED: std::cell::RefCell<std::collections::VecDeque<(Key, std::rc::Rc<str>)>> = Default::default();
    }
    let key = (src.len(), fingerprint(src), size.0, size.1);
    if let Some(found) = RESIZED.with(|kept| kept.borrow().iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())) {
        return Ok(found.to_string());
    }
    let made = resized(&bytes(src)?, size.0, size.1)?;
    RESIZED.with(|kept| {
        let mut kept = kept.borrow_mut();
        if kept.len() >= 64 {
            kept.pop_front();
        }
        kept.push_back((key, std::rc::Rc::from(made.as_str())));
    });
    Ok(made)
}

/// A source's identity for the caches above (FNV-1a over its text; the length is kept with it).
fn fingerprint(src: &str) -> u64 {
    src.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The bytes a source holds: a `data:` URL's payload, or bare base64.
pub fn bytes(src: &str) -> Result<Vec<u8>, String> {
    let payload = match src.strip_prefix("data:") {
        Some(rest) => {
            rest.split_once(";base64,")
                .ok_or("a media source must be a base64 data URL")?
                .1
        }
        None => src,
    };
    base64_decode(payload)
}

/// A picture's width and height, read from its header (PNG, JPEG, GIF or WebP).
pub fn dimensions(data: &[u8]) -> Result<(u32, u32), String> {
    let be32 = |at: usize| {
        data.get(at..at + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    let le16 = |at: usize| data.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as u32);
    let le24 = |at: usize| {
        data.get(at..at + 3)
            .map(|b| b[0] as u32 | (b[1] as u32) << 8 | (b[2] as u32) << 16)
    };
    let cut = || "the picture's header is cut short".to_string();
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok((be32(16).ok_or_else(cut)?, be32(20).ok_or_else(cut)?));
    }
    if data.starts_with(b"GIF8") {
        return Ok((le16(6).ok_or_else(cut)?, le16(8).ok_or_else(cut)?));
    }
    if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        return match data.get(12..16) {
            Some(b"VP8X") => Ok((le24(24).ok_or_else(cut)? + 1, le24(27).ok_or_else(cut)? + 1)),
            Some(b"VP8 ") => Ok((le16(26).ok_or_else(cut)? & 0x3fff, le16(28).ok_or_else(cut)? & 0x3fff)),
            Some(b"VP8L") => {
                let b = data.get(21..25).ok_or_else(cut)?;
                let bits = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                Ok(((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1))
            }
            _ => Err("a WebP picture of a kind this does not read".into()),
        };
    }
    if data.starts_with(&[0xFF, 0xD8]) {
        // JPEG: segments until a start-of-frame, which holds the height and the width.
        let mut at = 2;
        while let Some(&[0xFF, marker]) = data.get(at..at + 2).map(|b| <&[u8; 2]>::try_from(b).unwrap()) {
            if marker == 0xFF {
                at += 1;
                continue;
            }
            if matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
                let b = data.get(at + 5..at + 9).ok_or_else(cut)?;
                return Ok((
                    u16::from_be_bytes([b[2], b[3]]) as u32,
                    u16::from_be_bytes([b[0], b[1]]) as u32,
                ));
            }
            if matches!(marker, 0xD0..=0xD9 | 0x01) {
                at += 2;
                continue;
            }
            let len = data.get(at + 2..at + 4).ok_or_else(cut)?;
            at += 2 + u16::from_be_bytes([len[0], len[1]]) as usize;
        }
        return Err("a JPEG picture with no frame header".into());
    }
    Err("a picture must be a PNG, JPEG, GIF or WebP".into())
}

/// The picture resampled to `width` by `height`, as a PNG data URL. PNG and JPEG are read;
/// transparency is laid over white. The same bytes for the same input, on every machine.
pub fn resized(data: &[u8], width: u32, height: u32) -> Result<String, String> {
    // A file's header can claim any size: decoding is what allocates. Nothing larger than 64
    // megapixels is read (a 4K screenshot is 8).
    let (claimed_w, claimed_h) = dimensions(data)?;
    if claimed_w as u64 * claimed_h as u64 > 64_000_000 || width as u64 * height as u64 > 64_000_000 {
        return Err(format!(
            "a picture of {claimed_w} by {claimed_h} pixels is too large to resize (the most is 64 megapixels)"
        ));
    }
    let (rgb, w, h) = decode(data)?;
    let out = resample(&rgb, w as usize, h as usize, width as usize, height as usize);
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("writing the resized picture: {e}"))?;
        writer
            .write_image_data(&out)
            .map_err(|e| format!("writing the resized picture: {e}"))?;
    }
    Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
}

/// A PNG or JPEG as 8-bit RGB rows, with its width and height.
fn decode(data: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    if data.starts_with(b"\x89PNG") {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().map_err(|e| format!("reading the PNG: {e}"))?;
        let size = reader.output_buffer_size().ok_or("the PNG is too large to read")?;
        let mut buffer = vec![0; size];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|e| format!("reading the PNG: {e}"))?;
        buffer.truncate(info.buffer_size());
        // Over white: a transparent pixel is paper, as a page shows it.
        let over = |c: u8, a: u8| ((c as u32 * a as u32 + 255 * (255 - a as u32) + 127) / 255) as u8;
        let rgb: Vec<u8> = match info.color_type {
            png::ColorType::Rgb => buffer,
            png::ColorType::Rgba => buffer
                .chunks_exact(4)
                .flat_map(|p| [over(p[0], p[3]), over(p[1], p[3]), over(p[2], p[3])])
                .collect(),
            png::ColorType::Grayscale => buffer.iter().flat_map(|&g| [g, g, g]).collect(),
            png::ColorType::GrayscaleAlpha => buffer
                .chunks_exact(2)
                .flat_map(|p| {
                    let g = over(p[0], p[1]);
                    [g, g, g]
                })
                .collect(),
            png::ColorType::Indexed => return Err("reading the PNG: its palette was not expanded".into()),
        };
        return Ok((rgb, info.width, info.height));
    }
    if data.starts_with(&[0xFF, 0xD8]) {
        let options = zune_jpeg::zune_core::options::DecoderOptions::default()
            .jpeg_set_out_colorspace(zune_jpeg::zune_core::colorspace::ColorSpace::RGB);
        let mut decoder =
            zune_jpeg::JpegDecoder::new_with_options(zune_jpeg::zune_core::bytestream::ZCursor::new(data), options);
        let rgb = decoder.decode().map_err(|e| format!("reading the JPEG: {e}"))?;
        let (w, h) = decoder.dimensions().ok_or("reading the JPEG: no size")?;
        return Ok((rgb, w as u32, h as u32));
    }
    Err(
        "a picture that must be resized has to be a PNG or a JPEG (pass one of those, or one already at the size)"
            .into(),
    )
}

/// RGB rows resampled from `w` by `h` to `tw` by `th`. Each output pixel is the average of
/// the source area it covers, in whole numbers (weights in 1/256 of a pixel): an area average
/// when shrinking, a blend of the nearest pixels when enlarging.
fn resample(src: &[u8], w: usize, h: usize, tw: usize, th: usize) -> Vec<u8> {
    // One axis: for each output position, the source positions it covers and their weights.
    let spans = |from: usize, to: usize| -> Vec<Vec<(usize, u64)>> {
        (0..to)
            .map(|i| {
                // The output pixel covers [i, i + 1) * from / to of the source, in 1/256ths.
                // Enlarging, that is less than a pixel wide: widen it to one pixel about its
                // centre so neighbours blend.
                let (mut lo, mut hi) = ((i * from * 256 / to) as i64, ((i + 1) * from * 256).div_ceil(to) as i64);
                if hi - lo < 256 {
                    let centre = (lo + hi) / 2;
                    (lo, hi) = (centre - 128, centre + 128);
                }
                let (lo, hi) = (lo.max(0), hi.min(from as i64 * 256));
                let (first, last) = ((lo / 256) as usize, ((hi - 1) / 256) as usize);
                (first..=last.min(from - 1))
                    .map(|p| {
                        let (start, end) = ((p as i64 * 256).max(lo), ((p as i64 + 1) * 256).min(hi));
                        (p, (end - start).max(0) as u64)
                    })
                    .filter(|&(_, weight)| weight > 0)
                    .collect()
            })
            .collect()
    };
    let (xs, ys) = (spans(w, tw), spans(h, th));
    let mut out = Vec::with_capacity(tw * th * 3);
    for row in &ys {
        for column in &xs {
            let (mut sum, mut total) = ([0u64; 3], 0u64);
            for &(y, wy) in row {
                for &(x, wx) in column {
                    let at = (y * w + x) * 3;
                    let weight = wx * wy;
                    for c in 0..3 {
                        sum[c] += src[at + c] as u64 * weight;
                    }
                    total += weight;
                }
            }
            for value in sum {
                out.push(((value + total / 2) / total.max(1)) as u8);
            }
        }
    }
    out
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n =
            (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn base64_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut bits, mut have) = (0u32, 0u8);
    for c in text.bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return Err("a media source is not base64".into()),
        };
        bits = bits << 6 | value as u32;
        have += 6;
        if have >= 8 {
            have -= 8;
            out.push((bits >> have) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRID: Metrics = Metrics::Grid {
        cell: 32,
        min: 65_536,
        max: 16_777_216,
    };

    fn tokens(
        natural: (u32, u32),
        max: Option<usize>,
        min: Option<usize>,
        how: ObjectFit,
    ) -> Option<(usize, Option<(u32, u32)>)> {
        fit(&GRID, natural, max, min, how).map(|p| (p.tokens, p.size))
    }

    #[test]
    fn a_small_picture_is_scaled_up_as_the_processors_smart_resize_does() {
        // LFM3's processor (Qwen2-VL's smart_resize, cell 32, at least 65,536 pixels), run in
        // Docker: both sides rounded up, nothing taken back.
        let lfm3 = Metrics::Grid {
            cell: 32,
            min: 65_536,
            max: 16_777_216,
        };
        assert_eq!(
            fit(&lfm3, (33, 17), None, None, ObjectFit::ScaleDown).map(|p| p.tokens),
            Some(72)
        );
    }

    #[test]
    fn a_side_on_a_half_cell_rounds_to_the_even_cell_as_the_processor_does() {
        // Qwen's smart_resize uses Python's round(): 720 / 32 = 22.5 rounds to 22.
        assert_eq!(
            tokens((1280, 720), None, None, ObjectFit::ScaleDown),
            Some((880, Some((1280, 704))))
        );
        // 23.5 rounds to 24.
        assert_eq!(
            tokens((752, 752), None, None, ObjectFit::ScaleDown),
            Some((576, Some((768, 768))))
        );
    }

    #[test]
    fn a_picture_on_the_grid_is_sent_as_it_is() {
        assert_eq!(tokens((640, 480), None, None, ObjectFit::ScaleDown), Some((300, None)));
    }

    #[test]
    fn a_picture_off_the_grid_goes_to_the_nearest_cells() {
        assert_eq!(
            tokens((650, 470), None, None, ObjectFit::ScaleDown),
            Some((300, Some((640, 480))))
        );
    }

    #[test]
    fn a_cap_shrinks_a_picture_and_keeps_its_shape() {
        let (n, size) = tokens((1920, 1080), Some(256), None, ObjectFit::ScaleDown).unwrap();
        let (w, h) = size.unwrap();
        assert!((200..=256).contains(&n), "{n} tokens at {w}x{h}");
        assert_eq!(n, (w / 32 * (h / 32)) as usize);
        assert!(w > h && (w as f64 / h as f64 - 16.0 / 9.0).abs() < 0.25, "{w}x{h}");
    }

    #[test]
    fn every_cap_is_kept_for_every_shape() {
        for (w, h) in [
            (1, 1),
            (33, 4000),
            (4000, 33),
            (224, 224),
            (1919, 1079),
            (5000, 5000),
            (10_000, 20),
        ] {
            for cap in [1usize, 2, 7, 64, 65, 300, 1000, 20_000] {
                for how in [ObjectFit::ScaleDown, ObjectFit::Contain, ObjectFit::None] {
                    if let Some(p) = fit(&GRID, (w, h), Some(cap), None, how) {
                        let (pw, ph) = p.size.unwrap_or((w, h));
                        assert!(p.tokens <= cap, "{w}x{h} cap {cap}: {} tokens", p.tokens);
                        assert_eq!(p.tokens as u32, pw / 32 * (ph / 32));
                        assert!(pw % 32 == 0 && ph % 32 == 0 && (pw as u64 * ph as u64) <= 16_777_216);
                        assert!(
                            pw as u64 * ph as u64 >= 65_536,
                            "{w}x{h} cap {cap}: {pw}x{ph} is under the processor's least"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_picture_too_large_for_its_cap_is_not_shown() {
        // The processor's least is 65,536 pixels: 64 cells. A cap under that cannot be met.
        assert_eq!(tokens((640, 480), Some(32), None, ObjectFit::ScaleDown), None);
        assert_eq!(tokens((640, 480), Some(100), None, ObjectFit::None), None);
    }

    #[test]
    fn contain_enlarges_and_scale_down_does_not() {
        assert_eq!(
            tokens((320, 320), Some(400), None, ObjectFit::ScaleDown),
            Some((100, None))
        );
        assert_eq!(
            tokens((320, 320), Some(400), None, ObjectFit::Contain),
            Some((400, Some((640, 640))))
        );
        assert_eq!(
            tokens((320, 320), None, Some(400), ObjectFit::ScaleDown),
            Some((400, Some((640, 640))))
        );
    }

    #[test]
    fn a_picture_sized_by_its_shape_is_counted_as_a_real_server_counted_it() {
        // SGLang 0.5.23 serving Gemma 4 E2B, asked for the tokens it processed.
        let shape = Metrics::Shape { most: 280 };
        for (size, tokens) in [
            ((64, 64), 256),
            ((224, 224), 256),
            ((896, 896), 256),
            ((640, 480), 266),
            ((800, 600), 266),
            ((1280, 720), 264),
            ((1920, 1080), 264),
            ((2000, 200), 260),
            ((200, 2000), 260),
        ] {
            assert_eq!(
                fit(&shape, size, None, None, ObjectFit::ScaleDown).map(|p| (p.tokens, p.size)),
                Some((tokens, None)),
                "{size:?}"
            );
        }
        assert_eq!(fit(&shape, (640, 480), Some(256), None, ObjectFit::ScaleDown), None);
        // Edge cases the processor's own function gives (its floating point, and pictures too
        // thin for one cell across).
        for (size, tokens) in [
            ((10_000, 10), 280),
            ((154, 55), 252),
            ((595, 34), 207),
            ((3000, 100), 273),
            ((5000, 40), 187),
        ] {
            assert_eq!(
                fit(&shape, size, None, None, ObjectFit::ScaleDown).map(|p| p.tokens),
                Some(tokens),
                "{size:?}"
            );
        }
    }

    #[test]
    fn a_patch_processors_picture_is_counted_as_a_real_server_counted_it() {
        // SGLang 0.5.23 serving Kimi-VL-A3B-Instruct: its count less the 4 tokens around a
        // picture, for fourteen sizes.
        let rule = Metrics::Patches {
            patch: 14,
            limit: 4096,
            side: None,
        };
        for (size, tokens) in [
            ((64, 64), 9),
            ((224, 224), 64),
            ((256, 256), 100),
            ((448, 448), 256),
            ((512, 512), 361),
            ((640, 480), 414),
            ((800, 600), 638),
            ((1024, 768), 1036),
            ((1280, 720), 1075),
            ((1920, 1080), 1032),
            ((2000, 200), 576),
            ((200, 2000), 576),
            ((768, 768), 784),
            ((1000, 1000), 1089),
        ] {
            assert_eq!(
                fit(&rule, size, None, None, ObjectFit::ScaleDown).map(|p| (p.tokens, p.size)),
                Some((tokens, None)),
                "{size:?}"
            );
        }
        // Under a cap it is sent on the 28-pixel grid, where the processor leaves it alone.
        let capped = fit(&rule, (1920, 1080), Some(256), None, ObjectFit::ScaleDown).unwrap();
        let (w, h) = capped.size.unwrap();
        assert!(capped.tokens <= 256 && w % 28 == 0 && h % 28 == 0);
        assert_eq!(patches_tokens(14, 4096, None, (w, h)), capped.tokens);
        // Kimi K2.x: a larger limit and a cap of 512 patches a side (the processor's own counts).
        assert_eq!(patches_tokens(14, 16384, Some(512), (8000, 100)), 1024);
        assert_eq!(patches_tokens(14, 16384, Some(512), (20_000, 20)), 256);
        assert_eq!(patches_tokens(14, 16384, Some(512), (224, 224)), 64);
    }

    #[test]
    fn a_tiled_picture_is_laid_out_as_a_real_server_laid_it_out() {
        // SGLang 0.5.23 serving LFM2.5-VL-1.6B, asked for the tokens it processed.
        let rule = Metrics::Tiles {
            cell: 32,
            min: 64 * 1024,
            max: 256 * 1024,
            tile: 512,
            min_tiles: 2,
            max_tiles: 10,
            split_over: 524_288,
            start: "<|image_start|>".into(),
            end: "<|image_end|>".into(),
            marker: "<|img_row_{r}_col_{c}|>".into(),
            thumbnail: "<|img_thumbnail|>".into(),
        };
        let shape = |size| {
            // A block is sent at the size the processor would give it; tiles as they are.
            let (pieces, _sent) = tiles_layout(&rule, size, None).unwrap();
            let tiles = pieces
                .iter()
                .filter(|p| matches!(p, Piece::Marker(m) if m.starts_with("<|img_row")))
                .count();
            let total: usize = pieces.iter().map(piece_tokens).sum();
            (tiles, total)
        };
        // The server's count, with the two markers it adds around every picture.
        assert_eq!(shape((448, 448)), (0, 198));
        assert_eq!(shape((640, 480)), (0, 236));
        assert_eq!(shape((2000, 200)), (0, 252));
        assert_eq!(shape((64, 64)), (0, 66));
        assert_eq!(shape((1280, 720)), (2, 769));
        assert_eq!(shape((1024, 768)), (6, 1779));
        assert_eq!(shape((768, 768)), (4, 1287));
        assert_eq!(shape((1920, 1080)), (8, 2311));
        let (pieces, _) = tiles_layout(&rule, (1024, 768), None).unwrap();
        assert_eq!(pieces[1], Piece::Marker("<|img_row_1_col_1|>".into()));
        assert_eq!(pieces[11], Piece::Marker("<|img_row_2_col_3|>".into()));
        // Under a cap of 256 a large picture is sent as one block that fits.
        let (pieces, sent) = tiles_layout(&rule, (1920, 1080), Some(256)).unwrap();
        assert_eq!(pieces.len(), 3);
        let (w, h) = sent.unwrap();
        assert!((w as u64 * h as u64) <= 256 * 1024 && w % 32 == 0 && h % 32 == 0);
    }

    #[test]
    fn a_picture_in_rows_is_laid_out_as_a_real_server_laid_it_out() {
        // SGLang 0.5.23 serving Ministral-3-8B: its count less the 8 tokens of the template's
        // own text around a picture, for fourteen sizes.
        let rule = Metrics::Rows {
            cell: 28,
            longest: 1540,
            row_end: "[IMG_BREAK]".into(),
            end: "[IMG_END]".into(),
        };
        let count = |size| {
            let (pieces, sent) = rows_layout(&rule, size, None).unwrap();
            assert_eq!(sent, None);
            pieces.iter().map(piece_tokens).sum::<usize>()
        };
        for (size, tokens) in [
            ((64, 64), 12),
            ((224, 224), 72),
            ((256, 256), 110),
            ((448, 448), 272),
            ((512, 512), 380),
            ((640, 480), 432),
            ((800, 600), 660),
            ((1024, 768), 1064),
            ((1280, 720), 1222),
            ((1920, 1080), 1736),
            ((2000, 200), 336),
            ((200, 2000), 385),
            ((768, 768), 812),
            ((1000, 1000), 1332),
        ] {
            assert_eq!(count(size), tokens, "{size:?}");
        }
        // LightOnOCR: the cells alone (224 by 224 is 64 tokens, 640 by 480 is 414).
        let plain = Metrics::Rows {
            cell: 28,
            longest: 1540,
            row_end: String::new(),
            end: String::new(),
        };
        for (size, tokens) in [((224, 224), 64), ((640, 480), 414), ((2000, 200), 330)] {
            let (pieces, _) = rows_layout(&plain, size, None).unwrap();
            assert_eq!(pieces.iter().map(piece_tokens).sum::<usize>(), tokens);
        }
        let (pieces, sent) = rows_layout(&rule, (1920, 1080), Some(256)).unwrap();
        let total: usize = pieces.iter().map(piece_tokens).sum();
        let (w, h) = sent.unwrap();
        assert!(total <= 256 && w % 28 == 0 && h % 28 == 0, "{total} at {w}x{h}");
        assert_eq!(pieces.last(), Some(&Piece::Marker("[IMG_END]".into())));
    }

    #[test]
    fn a_deepseek_v41_picture_is_counted_as_sglangs_own_code_counts_it() {
        // SGLang 0.5.23's plan_image_grid and num_image_tokens, run on these sizes.
        let rule = Metrics::Lines {
            patch: 14,
            downsample: 3,
            max_tokens: 1024,
            min_pixels: 295_936,
            max_wh_ratio: None,
        };
        for (size, tokens) in [
            ((64, 64), 184),
            ((224, 224), 184),
            ((512, 512), 184),
            ((543, 544), 184),
            ((640, 480), 206),
            ((800, 600), 317),
            ((1024, 768), 496),
            ((1280, 720), 578),
            ((1920, 1080), 968),
            ((2000, 200), 247),
            ((200, 2000), 290),
            ((768, 768), 382),
            ((1000, 1000), 602),
            ((4000, 3000), 1001),
            ((3000, 100), 221),
            ((100, 3000), 290),
        ] {
            assert_eq!(lines_tokens(&rule, size), tokens, "{size:?}");
            assert_eq!(
                fit(&rule, size, None, None, ObjectFit::ScaleDown).map(|p| (p.tokens, p.size)),
                Some((tokens, None))
            );
        }
        assert_eq!(fit(&rule, (1920, 1080), Some(500), None, ObjectFit::ScaleDown), None);
    }

    #[test]
    fn ovis_and_intern_pictures_are_counted_as_their_processors_count_them() {
        // Ovis2.5-2B's processor code, run in Docker (N + 2).
        let ovis = Metrics::OvisGrid {
            cell: 32,
            min: 448 * 448,
            max: 1344 * 1792,
            wrap: 2,
        };
        for (size, tokens) in [
            ((224, 224), 198),
            ((640, 480), 302),
            ((1280, 720), 882),
            ((1920, 1080), 2042),
            ((2000, 200), 374),
            ((32, 43), 223),
        ] {
            assert_eq!(ovis_tokens(&ovis, size), tokens, "{size:?}");
        }
        // Intern-S1's processor (transformers), run in Docker: 2 + 256 x pieces.
        let intern = Metrics::DynTiles {
            tile: 448,
            per_tile: 256,
            min_tiles: 1,
            max_tiles: 12,
            tie_more: false,
            start: "<img>".into(),
            end: "</img>".into(),
        };
        let total = |size| {
            let (pieces, _) = dyntiles_layout(&intern, size, None).unwrap();
            pieces.iter().map(piece_tokens).sum::<usize>()
        };
        for (size, tokens) in [
            ((64, 64), 258),
            ((512, 512), 258),
            ((640, 480), 3330),
            ((1280, 720), 2306),
            ((2000, 200), 2818),
            ((768, 768), 1282),
            ((1000, 1000), 2562),
        ] {
            assert_eq!(total(size), tokens, "{size:?}");
        }
    }

    #[test]
    fn a_llama4_picture_is_laid_out_as_its_processor_lays_it_out() {
        let rule = Metrics::Canvas {
            tile: 336,
            max_tiles: 16,
            per_tile: 144,
            start: "<|image_start|>".into(),
            end: "<|image_end|>".into(),
            patch: "<|patch|>".into(),
            global: "<|image|>".into(),
            col_sep: "<|tile_x_separator|>".into(),
            row_sep: "<|tile_y_separator|>".into(),
        };
        let total = |size| {
            canvas_layout(&rule, size, None)
                .unwrap()
                .0
                .iter()
                .map(piece_tokens)
                .sum::<usize>()
        };
        // One tile is 147; r x c tiles are 147 + 145 r c (the processor, and two hosted servers).
        assert_eq!(total((224, 224)), 147);
        assert_eq!(total((336, 336)), 147);
        let (pieces, _) = canvas_layout(&rule, (672, 336), None).unwrap();
        assert_eq!(pieces.iter().map(piece_tokens).sum::<usize>(), 147 + 145 * 2);
        assert_eq!(pieces[2], Piece::Marker("<|tile_x_separator|>".into()));
    }

    #[test]
    fn a_fixed_picture_is_shown_or_not() {
        let fixed = Metrics::Fixed { tokens: 256 };
        assert_eq!(
            fit(&fixed, (10, 10), Some(256), None, ObjectFit::ScaleDown).map(|p| p.tokens),
            Some(256)
        );
        assert_eq!(fit(&fixed, (10, 10), Some(255), None, ObjectFit::ScaleDown), None);
    }

    /// A 4 by 2 PNG, half black and half white, written by the encoder this module uses.
    fn sample() -> Vec<u8> {
        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, 4, 2);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let row: Vec<u8> = [0u8, 0, 255, 255].iter().flat_map(|&v| [v, v, v]).collect();
        writer.write_image_data(&[row.clone(), row].concat()).unwrap();
        drop(writer);
        png
    }

    #[test]
    fn a_resized_picture_has_the_size_asked_and_the_same_bytes_each_time() {
        let png = sample();
        assert_eq!(dimensions(&png), Ok((4, 2)));
        let small = resized(&png, 2, 1).unwrap();
        assert_eq!(small, resized(&png, 2, 1).unwrap());
        let data = bytes(&small).unwrap();
        assert_eq!(dimensions(&data), Ok((2, 1)));
        let (rgb, w, h) = decode(&data).unwrap();
        assert_eq!((w, h, rgb), (2, 1, vec![0, 0, 0, 255, 255, 255]));
        // Enlarged: the edge between black and white stays in the middle.
        let (rgb, ..) = decode(&bytes(&resized(&png, 8, 4).unwrap()).unwrap()).unwrap();
        assert_eq!((rgb[0], rgb[7 * 3]), (0, 255));
    }

    /// One second of silence at 16 kHz, 16-bit mono, with an extra chunk before the samples.
    fn second() -> String {
        let samples = vec![0u8; 32_000];
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&(4 + 24 + 12 + 8 + 32_000u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        for field in [1u16, 1] {
            wav.extend_from_slice(&field.to_le_bytes());
        }
        wav.extend_from_slice(&16_000u32.to_le_bytes());
        wav.extend_from_slice(&32_000u32.to_le_bytes());
        for field in [2u16, 16] {
            wav.extend_from_slice(&field.to_le_bytes());
        }
        wav.extend_from_slice(b"LIST\x04\x00\x00\x00INFO");
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&32_000u32.to_le_bytes());
        wav.extend_from_slice(&samples);
        format!("data:audio/wav;base64,{}", base64_encode(&wav))
    }

    #[test]
    fn a_clip_takes_a_token_for_each_part_of_a_second_and_a_cap_trims_it() {
        // Measured on Gemma 4: 40 ms a token, rounded up, and 2 around the clip.
        let rate = Metrics::Rate {
            ms: 40,
            extra: 2,
            most: Some(750),
        };
        let tokens = |ms, cap| fit_clip(&rate, ms, cap).map(|p| (p.tokens, p.keep_ms));
        assert_eq!(tokens(1000, None), Some((27, None)));
        assert_eq!(tokens(1020, None), Some((28, None)));
        assert_eq!(tokens(4305, None), Some((110, None)));
        assert_eq!(tokens(4305, Some(52)), Some((52, Some(2000))));
        assert_eq!(tokens(60_000, None), Some((752, Some(30_000))));
        assert_eq!(tokens(1000, Some(2)), None);
        let clip = second();
        assert_eq!(clip_length_ms(&clip), Ok(1000));
        let start = clipped(&clip, 250, false).unwrap();
        assert_eq!(clip_length_ms(&start), Ok(250));
        assert_eq!(start, clipped(&clip, 250, false).unwrap());
        assert_eq!(clip_length_ms(&clipped(&clip, 250, true).unwrap()), Ok(250));
        assert!(clip_length_ms("data:audio/mpeg;base64,SUQz").is_err());
    }

    #[test]
    fn a_video_is_laid_out_as_a_real_server_was_measured_to() {
        // SGLang 0.5.23 serving Qwen3.5-4B, asked for the tokens it processed.
        let rule = Metrics::Frames {
            cell: 32,
            min: 128 * 28 * 28,
            max: 768 * 28 * 28,
            total: 90_316_800,
            fps: 2,
            min_frames: 4,
            max_frames: 768,
            open: "<|vision_start|>".into(),
            close: "<|vision_end|>".into(),
        };
        let placed = |frames, fps, width, height| {
            fit_video(
                &rule,
                Video {
                    frames,
                    fps,
                    width,
                    height,
                },
            )
            .map(|p| (p.stamps, p.cells))
        };
        let stamps = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // 224 by 224, 4 frames a second, 2 seconds.
        assert_eq!(
            placed(8, 4.0, 224, 224),
            Some((stamps(&["<0.2 seconds>", "<1.4 seconds>"]), 100))
        );
        // 640 by 360, 30 frames a second, 3 seconds.
        assert_eq!(
            placed(90, 30.0, 640, 360),
            Some((stamps(&["<0.3 seconds>", "<1.5 seconds>", "<2.7 seconds>"]), 220))
        );
        // 320 by 240, 10 frames a second, 1 second.
        assert_eq!(
            placed(10, 10.0, 320, 240),
            Some((stamps(&["<0.1 seconds>", "<0.8 seconds>"]), 108))
        );
        assert_eq!(placed(1, 10.0, 320, 240), None);
    }

    #[test]
    fn base64_round_trips() {
        for data in [&b""[..], b"a", b"ab", b"abc", b"\x00\xff\x10\x80"] {
            assert_eq!(base64_decode(&base64_encode(data)).unwrap(), data);
        }
    }
}
