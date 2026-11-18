//! Picture rules that are one model family's own: each a function from a picture's size to the
//! token runs its processor puts in place of the template's placeholder, checked in its tests
//! against the processor's own counts. `src/media.rs` has the rules several families share.

pub mod aligned;
pub mod anyres;
pub mod cohere2;
pub mod deepseek_ocr;
pub mod hmlp;
pub mod hunyuan;
pub mod marked_grid;
pub mod nearest;
pub mod nemotron_patches;
pub mod overlap_crops;
pub mod ovis;
pub mod padgrid;
pub mod rowcol;
pub mod score_tiles;
pub mod slices;
pub mod step3;

/// The runs a named rule gives a picture of `width` by `height`: `(None, n)` is n placeholder
/// tokens, `(Some(name), n)` n of the token `name`, `(Some("text:..."), 1)` that text.
pub fn layout(
    name: &str,
    params: &serde_json::Value,
    width: u32,
    height: u32,
) -> Result<Vec<(Option<String>, usize)>, String> {
    match name {
        "aligned" => aligned::layout(params, width, height),
        "anyres" => anyres::layout(params, width, height),
        "cohere2" => cohere2::layout(params, width, height),
        "deepseek_ocr" => deepseek_ocr::layout(params, width, height),
        "hmlp" => hmlp::layout(params, width, height),
        "hunyuan" => hunyuan::layout(params, width, height),
        "marked_grid" => marked_grid::layout(params, width, height),
        "nemotron_patches" => nemotron_patches::layout(params, width, height),
        "nearest" => nearest::layout(params, width, height),
        "overlap_crops" => overlap_crops::layout(params, width, height),
        "ovis" => ovis::layout(params, width, height),
        "padgrid" => padgrid::layout(params, width, height),
        "rowcol" => rowcol::layout(params, width, height),
        "score_tiles" => score_tiles::layout(params, width, height),
        "slices" => slices::layout(params, width, height),
        "step3" => step3::layout(params, width, height),
        _ => Err(format!("no picture rule named {name:?}")),
    }
}

/// Whether a rule of this name exists: the same names `layout` takes.
pub fn known(name: &str) -> bool {
    !matches!(layout(name, &serde_json::Value::Null, 1, 1), Err(e) if e == format!("no picture rule named {name:?}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_rule_file_is_a_known_rule() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/media_kinds");
        for entry in std::fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            if let Some(rule) = name.strip_suffix(".rs").filter(|n| *n != "mod") {
                assert!(super::known(rule), "{rule}");
            }
        }
        assert!(!super::known("nothing"));
    }
}
