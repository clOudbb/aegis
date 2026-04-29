#!/usr/bin/env sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
CHECKSUM_DIR="$ROOT_DIR/dist/checksums"
CHECKSUM_FILE="$CHECKSUM_DIR/SHA256SUMS"

mkdir -p "$CHECKSUM_DIR"
rm -f "$CHECKSUM_FILE"

if command -v shasum >/dev/null 2>&1; then
  hash_cmd='shasum -a 256'
elif command -v sha256sum >/dev/null 2>&1; then
  hash_cmd='sha256sum'
else
  echo "shasum or sha256sum is required." >&2
  exit 1
fi

find "$ROOT_DIR/dist" \
  -type f \
  \( -name '*.zip' -o -name '*.xcframework.zip' \) \
  ! -path "$CHECKSUM_DIR/*" \
  -print | sort | while IFS= read -r artifact; do
    (
      cd "$ROOT_DIR"
      $hash_cmd "${artifact#$ROOT_DIR/}"
    ) >> "$CHECKSUM_FILE"
  done

cat "$CHECKSUM_FILE"
