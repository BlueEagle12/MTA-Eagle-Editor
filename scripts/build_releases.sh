#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MANIFEST="$PROJECT_DIR/Cargo.toml"
OUTPUT="$PROJECT_DIR/release/github-upload"
WINDOWS_TARGET="x86_64-pc-windows-gnu"

if ! rustup target list --installed | grep -qx "$WINDOWS_TARGET"; then
  echo "Missing Rust target $WINDOWS_TARGET. Install it with:" >&2
  echo "  rustup target add $WINDOWS_TARGET" >&2
  exit 1
fi

rm -rf "$OUTPUT"
cargo build --release --locked --manifest-path "$MANIFEST"
python3 "$SCRIPT_DIR/package_release.py" \
  --platform linux-x86_64 \
  --binary "$PROJECT_DIR/target/release/eagle-editor" \
  --output "$OUTPUT"

cargo build --release --locked --manifest-path "$MANIFEST" --target "$WINDOWS_TARGET"
python3 "$SCRIPT_DIR/package_release.py" \
  --platform windows-x86_64 \
  --binary "$PROJECT_DIR/target/$WINDOWS_TARGET/release/eagle-editor.exe" \
  --output "$OUTPUT"

python3 "$SCRIPT_DIR/package_release.py" --output "$OUTPUT" --bundle-only
echo "Release folders and archives are ready in $OUTPUT"
