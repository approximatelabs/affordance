#!/usr/bin/env bash
# The tokenizers the integration tests load, for a checkout without the full reference data (CI).
#
#   tools/fetch-test-tokenizers.sh
#
# Downloads one archive from this repository's `test-tokenizers-1` release, checks its SHA-256,
# and puts its files in ref/tokenizers/ (a file already there is left as it is). The archive holds
# five publishers' tokenizer files under their licenses (its README lists them). Tests that need
# other models or the official outputs stay behind the `reference-tests` feature.
set -euo pipefail
cd "$(dirname "$0")/.."
name=affordance-test-tokenizers-1.tar.gz
url=https://github.com/approximatelabs/affordance/releases/download/test-tokenizers-1/$name
sha256=d4959b19479bb48bef95a16dc57b599ae21f07b514d0517d223cd25a5ae0d593

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -fsSL --retry 3 "$url" -o "$work/$name"
if command -v sha256sum >/dev/null; then got=$(sha256sum "$work/$name" | cut -d' ' -f1)
else got=$(shasum -a 256 "$work/$name" | cut -d' ' -f1); fi
[ "$got" = "$sha256" ] || { echo "fetch-test-tokenizers: $name has SHA-256 $got, not $sha256" >&2; exit 1; }
tar -xzf "$work/$name" -C "$work"
mkdir -p ref/tokenizers
cp -n "$work"/tokenizers/* ref/tokenizers/ 2>/dev/null || true
echo "fetch-test-tokenizers: $(ls "$work"/tokenizers | wc -l | tr -d ' ') files in ref/tokenizers"
