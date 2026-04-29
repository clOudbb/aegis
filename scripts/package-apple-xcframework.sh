#!/usr/bin/env sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist/apple"
HEADER_DIR="$DIST_DIR/Headers"
XCFRAMEWORK="$DIST_DIR/AegisFFI.xcframework"
ZIP_FILE="$DIST_DIR/AegisFFI.xcframework.zip"

targets="
aarch64-apple-darwin
aarch64-apple-ios
aarch64-apple-ios-sim
"

case "$(uname -s)" in
  Darwin) ;;
  *)
    echo "Apple packaging must run on macOS." >&2
    exit 1
    ;;
esac

command -v xcodebuild >/dev/null 2>&1 || {
  echo "xcodebuild is required." >&2
  exit 1
}

rm -rf "$DIST_DIR"
mkdir -p "$HEADER_DIR"
cp "$ROOT_DIR/include/aegis.h" "$HEADER_DIR/aegis.h"
cp "$ROOT_DIR/include/module.modulemap" "$HEADER_DIR/module.modulemap"

for target in $targets; do
  if ! rustup target list --installed | grep -qx "$target"; then
    echo "Missing Rust target: $target" >&2
    echo "Install it with: rustup target add $target" >&2
    exit 1
  fi
  cargo build -p aegis-ffi --release --target "$target"
done

rm -rf "$XCFRAMEWORK"
xcodebuild -create-xcframework \
  -library "$ROOT_DIR/target/aarch64-apple-darwin/release/libaegis_ffi.a" -headers "$HEADER_DIR" \
  -library "$ROOT_DIR/target/aarch64-apple-ios/release/libaegis_ffi.a" -headers "$HEADER_DIR" \
  -library "$ROOT_DIR/target/aarch64-apple-ios-sim/release/libaegis_ffi.a" -headers "$HEADER_DIR" \
  -output "$XCFRAMEWORK"

rm -f "$ZIP_FILE"
(
  cd "$DIST_DIR"
  zip -qry "AegisFFI.xcframework.zip" "AegisFFI.xcframework"
)

echo "$XCFRAMEWORK"
echo "$ZIP_FILE"
