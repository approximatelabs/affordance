"""Write each vision model's picture metrics into views/index.json.

    uv run python tools/measure_media.py            every registered model that takes pictures
    uv run python tools/measure_media.py ORG/NAME   one model

A model's processor turns a picture into tokens by a rule. Layout needs that rule to give a
picture a size in tokens (src/media.rs): the row `agent.media_metrics.image`. This reads the
rule from the publisher's processor configuration (`preprocessor_config.json`, fetched into
ref/models beside the template) for the processor designs it knows:

- a grid (Qwen2-VL and its descendants, GLM-V): cells of patch x merge pixels, one token each,
  within a least and a most number of pixels. `{"kind": "grid", "cell", "min", "max"}`
- a fixed count (Gemma 3): every picture is the same number of tokens. `{"kind": "fixed"}`

A model whose processor is another design gets no row, and its pictures keep one placeholder
and the file as it is. `tools/check-media.sh` checks a row against a real server: the tokens
laid out must be the tokens the server processed.
"""
import json, os, sys, urllib.error, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FILES = ["preprocessor_config.json", "processor_config.json"]
# Processors that cut a picture into a grid of patch x merge cells after rounding its sides to
# the grid and keeping its area within bounds (transformers' `smart_resize`).
GRID = {"PaddleOCRVLImageProcessor", "Qwen2VLImageProcessor", "Qwen2VLImageProcessorFast", "Qwen2VLImageProcessorPil", "Glm4vImageProcessor", "Glm4vImageProcessorFast", "Glm46VImageProcessor", "Glm46VImageProcessorFast"}
FIXED = {"Gemma3ImageProcessor", "Gemma3ImageProcessorFast"}


def fetch(model, name):
    """The model's file `name`, from ref/models or from its repository (kept for next time)."""
    path = os.path.join(ROOT, "ref", "models", model, name)
    if os.path.exists(path): return json.load(open(path))
    headers = {"User-Agent": "media-metrics"}
    if os.environ.get("HF_TOKEN"): headers["Authorization"] = "Bearer " + os.environ["HF_TOKEN"]
    repo = model.replace("__", "/", 1)
    try:
        with urllib.request.urlopen(urllib.request.Request(f"https://huggingface.co/{repo}/resolve/main/{name}", headers=headers), timeout=30) as response:
            text = response.read().decode()
    except (urllib.error.URLError, TimeoutError):
        return None
    try:
        found = json.loads(text)
    except ValueError:
        return None
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "w").write(text)
    return found


def metrics(model):
    """(the row or None, why)."""
    config = fetch(model, "preprocessor_config.json")
    if config is None:
        both = fetch(model, "processor_config.json") or {}
        config = both.get("image_processor")
    if not isinstance(config, dict): return None, "no processor configuration"
    kind = config.get("image_processor_type") or "?"
    if kind in GRID:
        patch, merge = config.get("patch_size"), config.get("merge_size")
        size = config.get("size") or {}
        least = config.get("min_pixels") or size.get("shortest_edge")
        most = config.get("max_pixels") or size.get("longest_edge")
        if not all(isinstance(n, int) and n > 0 for n in (patch, merge, least, most)): return None, f"{kind}: sizes missing"
        return {"kind": "grid", "cell": patch * merge, "min": least, "max": most}, kind
    both = fetch(model, "processor_config.json") or {}
    if kind in ("PixtralImageProcessor", "PixtralImageProcessorFast") and both.get("image_break_token") and both.get("image_end_token"):
        # Pixtral's rows, measured on a real server with fourteen sizes.
        patch, longest = config.get("patch_size"), (config.get("size") or {}).get("longest_edge")
        if isinstance(patch, int) and isinstance(longest, int):
            # Measured on real servers: Pixtral's own models ([IMG]) write a break after each row
            # and an end; LightOnOCR (<|image_pad|>) writes the cells alone.
            plain = both.get("image_token") != "[IMG]"
            return {"kind": "rows", "cell": patch * (both.get("spatial_merge_size") or 1), "longest": longest,
                    "row_end": "" if plain else both["image_break_token"], "end": "" if plain else both["image_end_token"]}, kind
    if both.get("processor_class") == "Gemma3nProcessor" and isinstance(both.get("image_seq_length"), int):
        # Gemma 3n: the same count for every picture, measured on a real server.
        return {"kind": "fixed", "tokens": both["image_seq_length"]}, kind
    if kind in ("Lfm2VlImageProcessor", "Lfm2VlImageProcessorFast") and config.get("do_image_splitting", True) and config.get("use_thumbnail", True):
        # LFM2-VL's tiles, measured on a real server with fourteen sizes.
        need = ("encoder_patch_size", "downsample_factor", "min_image_tokens", "max_image_tokens", "tile_size", "min_tiles", "max_tiles", "max_num_patches")
        if all(isinstance(config.get(k), int) for k in need):
            cell = config["encoder_patch_size"] * config["downsample_factor"]
            return {"kind": "tiles", "cell": cell, "min": config["min_image_tokens"] * cell * cell, "max": config["max_image_tokens"] * cell * cell,
                    "tile": config["tile_size"], "min_tiles": config["min_tiles"], "max_tiles": config["max_tiles"],
                    "split_over": int(config["max_num_patches"] * config["encoder_patch_size"] ** 2 * config.get("max_pixels_tolerance", 2.0)),
                    "start": "<|image_start|>", "end": "<|image_end|>", "marker": "<|img_row_{r}_col_{c}|>", "thumbnail": "<|img_thumbnail|>"}, kind
    if kind == "?" and isinstance(config.get("in_token_limit"), int) and isinstance(config.get("patch_size"), int) and config.get("pad_input"):
        # Kimi-VL's patches, measured on a real server with fourteen sizes.
        return {"kind": "patches", "patch": config["patch_size"], "limit": config["in_token_limit"]}, "Kimi-VL patches"
    if kind in ("Gemma4ImageProcessor", "Gemma4UnifiedImageProcessor") and isinstance(config.get("max_soft_tokens"), int):
        # Measured on a real server with eleven sizes: the largest grid of the picture's
        # proportions with at most this many cells (a square 256, 4:3 266, 16:9 264).
        return {"kind": "shape", "most": config["max_soft_tokens"]}, kind
    if kind in FIXED:
        # The count is the processor's `image_seq_length`, with pan-and-scan off (its default).
        both = fetch(model, "processor_config.json") or {}
        tokens = both.get("image_seq_length") or config.get("image_seq_length")
        if config.get("do_pan_and_scan") or not isinstance(tokens, int): return None, f"{kind}: no fixed count"
        return {"kind": "fixed", "tokens": tokens}, kind
    return None, kind


def clip_metrics(model):
    """A sound clip's rate, for the processors measured on a real server. Gemma 4: one token
    for each `audio_ms_per_token` milliseconds, rounded up, plus 2 around the clip, and at most
    `audio_seq_length` for one clip (1,000 ms was 27 tokens, 4,305 ms was 110)."""
    config = fetch(model, "processor_config.json") or {}
    extractor = (config.get("feature_extractor") or {}).get("feature_extractor_type")
    ms, most = config.get("audio_ms_per_token"), config.get("audio_seq_length")
    if extractor == "Gemma4AudioFeatureExtractor" and isinstance(ms, int) and isinstance(most, int):
        return {"kind": "rate", "ms": ms, "extra": 2, "most": most}
    return None


# Rules found for processor designs this script does not read, each checked against reference
# counts (the processor's own code run on the CPU, or a hosted model's reported prompt tokens):
# model -> {"image": rule, "audio": rule, "video": rule}. They are taken as they are.
MEASURED = os.path.join(ROOT, "views", "media_rules.json")


def main(only):
    path = os.path.join(ROOT, "views", "index.json")
    index = json.load(open(path))
    measured = json.load(open(MEASURED)) if os.path.exists(MEASURED) else {}
    seen, written = {}, 0
    for model, entry in index.items():
        if model.startswith("_") or not isinstance(entry.get("agent"), dict): continue
        if only and model not in only: continue
        agent = entry["agent"]
        if not ((agent.get("media") or {}).get("image") or (agent.get("media") or {}).get("audio")): continue
        row, why = metrics(model) if (agent.get("media") or {}).get("image") else (None, "sound only")
        seen[why] = seen.get(why, 0) + 1
        vision = (fetch(model, "config.json") or {}).get("vision_config") or {}
        if not row and vision.get("model_type") == "deepseek_v41_vision":
            # DeepSeek V4.1's lines of cells, from its vision settings (SGLang's own sizing code).
            row = {"kind": "lines", "patch": vision.get("patch_size", 14), "downsample": vision.get("downsample_ratio", 3),
                   "max_tokens": vision.get("max_image_tokens", 1024), "min_pixels": vision.get("min_pixels", 295936)}
            if vision.get("max_wh_ratio"): row["max_wh_ratio"] = vision["max_wh_ratio"]
        rows = {"image": row} if row else {}
        for kind, rule in (measured.get(model) or {}).items():
            if kind != "_source": rows[kind] = rule
        sound = clip_metrics(model) if (agent.get("media") or {}).get("audio") else None
        if sound: rows["audio"] = sound
        # A video as timed frames, for the model types whose processor writes one that way
        # (measured on Qwen3.5-4B), with the reference preprocessing's bounds.
        model_type = (fetch(model, "config.json") or {}).get("model_type")
        if row and row.get("kind") == "grid" and (agent.get("media") or {}).get("video") and model_type in ("qwen3_vl", "qwen3_vl_moe", "qwen3_5", "qwen3_5_moe"):
            rows["video"] = {"kind": "frames", "cell": row["cell"], "min": 128 * 28 * 28, "max": 768 * 28 * 28, "total": 90316800,
                             "fps": 2, "min_frames": 4, "max_frames": 768, "open": "<|vision_start|>", "close": "<|vision_end|>"}
        if rows:
            agent["media_metrics"] = rows; written += 1
        else:
            agent.pop("media_metrics", None)
    open(path, "w").write(json.dumps(index, indent=1) + "\n")
    print(f"{written} models have media metrics")
    for why, n in sorted(seen.items(), key=lambda x: -x[1]): print(f"  {n:4} {why}")


if __name__ == "__main__":
    main({a.replace("/", "__", 1) for a in sys.argv[1:]})
