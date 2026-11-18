"""Fetch the three pinned official NVIDIA files used by CPU reference tests.

Run with --destination ref/middle_models/nvidia__NVIDIA-Nemotron-3-Nano-30B-A3B-BF16 (or fetch
elsewhere and copy that folder there).
"""
import argparse
import hashlib
import json
from pathlib import Path
from urllib.request import urlopen

parser = argparse.ArgumentParser()
parser.add_argument("--destination", type=Path, required=True)
args = parser.parse_args()
manifest = json.loads((Path(__file__).resolve().parent.parent / "ref/middle_out/nemotron_source.json").read_text())
args.destination.mkdir(parents=True, exist_ok=True)
for name, expected in manifest["files"].items():
    path = args.destination / name
    if path.exists() and hashlib.sha256(path.read_bytes()).hexdigest() == expected:
        continue
    url = f'https://huggingface.co/{manifest["model"]}/resolve/{manifest["revision"]}/{name}'
    with urlopen(url, timeout=60) as response:
        data = response.read()
    if hashlib.sha256(data).hexdigest() != expected:
        raise RuntimeError(f"official source hash mismatch: {name}")
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_bytes(data)
    temporary.replace(path)
    print(f"{name}: {len(data)} bytes")
