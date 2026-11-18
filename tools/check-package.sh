#!/usr/bin/env bash
# The distributed framework contains no research project or Python runtime.
set -euo pipefail
cd "$(dirname "$0")/.."
package_list=$(mktemp)
trap 'rm -f "$package_list"' EXIT
cargo package --list --allow-dirty --offline > "$package_list"
if rg '^(CLAUDE\.md|TICKET_|docs/design/)' "$package_list"; then
  echo "Private project files entered the framework package." >&2
  exit 1
fi
if rg '^(phase2|results|reports|models|datasets|eval|engine|adapters)/|\.py$' "$package_list"; then
  echo "Research or Python files entered the framework package." >&2
  exit 1
fi
while IFS= read -r file; do
  if [[ -f "$file" ]] && head -n 1 "$file" | rg -q '^#!.*python'; then
    echo "Python executable entered the framework package: $file" >&2
    exit 1
  fi
done < "$package_list"
