#!/usr/bin/env python3
"""Audit the lockfile, allowing only reviewed, version- and callsite-bound exceptions."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent


def check():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", default=os.environ.get("AFFORDANCE_RUSTSEC_DB"), help="local RustSec database")
    parser.add_argument("--no-fetch", action="store_true", default=os.environ.get("AFFORDANCE_SECURITY_OFFLINE") == "1")
    args = parser.parse_args()
    command = [os.environ.get("CARGO_AUDIT", "cargo-audit"), "audit", "--json"]
    if args.db:
        command += ["--db", str(Path(args.db).resolve())]
    if args.no_fetch:
        command += ["--no-fetch"]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if result.returncode not in (0, 1) or not result.stdout.strip():
        raise RuntimeError(result.stderr or "cargo-audit produced no report")
    report = json.loads(result.stdout)
    if report.get("error"):
        raise RuntimeError(str(report["error"]))
    if not report.get("database", {}).get("advisory-count") or not report.get("lockfile", {}).get("dependency-count") or "vulnerabilities" not in report:
        raise RuntimeError("incomplete audit report or empty advisory database")
    records = report["vulnerabilities"]["list"]
    records += [item for group in report.get("warnings", {}).values() for item in group]
    policy = json.loads((ROOT / "tools/security-exceptions.json").read_text())
    packages = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
    errors, accepted = [], []
    for record in records:
        advisory, package = record["advisory"], record["package"]
        rule = policy["advisories"].get(advisory["id"])
        label = f'{advisory["id"]}: {package["name"]} {package["version"]}'
        if not rule or rule["package"] != package["name"] or rule["version"] != package["version"]:
            errors.append(label)
            continue
        if datetime.date.today() > datetime.date.fromisoformat(policy["review_by"]):
            errors.append(f"{label}: exception needs review")
            continue
        accepted.append(label)
    # The reviewed boundaries hold whether or not an advisory currently matches.
    for name, pin in policy["packages"].items():
        matches = [p for p in packages if p["name"] == name]
        if len(matches) != 1 or any(matches[0].get(k) != v for k, v in pin.items()):
            errors.append(f"dependency boundary changed: {name}")
    for path, digest in policy["callsites"].items():
        if hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != digest:
            errors.append(f"reviewed callsite changed: {path}")
    # Fail on newly introduced access to the affected APIs, even if dependency versions stay put.
    forbidden = re.compile(r"\b(?:zip|quick_xml|inferno)::|\b(?:read_npz|read_pth|from_pth|from_npz)\b|candle_core::(?:pickle|npy)")
    for directory in (ROOT / "src", ROOT / "tools/bench", ROOT / "examples"):
        for path in directory.rglob("*.rs"):
            source = path.read_text()
            if forbidden.search(source) or ("pprof::" in source and str(path.relative_to(ROOT)) not in policy["callsites"]):
                errors.append(f"review dependency exposure: {path.relative_to(ROOT)}")
    for item in sorted(set(accepted)):
        print(f"Reviewed exception: {item} (docs/SECURITY.md)")
    if errors:
        raise RuntimeError("\n".join(sorted(set(errors))))
    print("Dependency audit passed; every remaining match has a checked, documented exception.")


if __name__ == "__main__":
    try:
        check()
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        print(f"Security check failed: {error}", file=sys.stderr)
        sys.exit(1)
