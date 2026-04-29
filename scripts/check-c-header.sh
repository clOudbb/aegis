#!/usr/bin/env sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
BUILD_DIR="$ROOT_DIR/target/header-smoke"
SOURCE_FILE="$BUILD_DIR/header_smoke.c"
BINARY_FILE="$BUILD_DIR/header_smoke"

mkdir -p "$BUILD_DIR"

cargo build -p aegis-ffi --release

cat > "$SOURCE_FILE" <<'C_EOF'
#include "aegis.h"

#include <stdint.h>
#include <string.h>

static AegisStringView aegis_sv(const char *text) {
    AegisStringView view;
    view.ptr = (const uint8_t *)text;
    view.len = strlen(text);
    return view;
}

int main(void) {
    AegisAbiVersion version = aegis_abi_version();
    if (version.size != sizeof(AegisAbiVersion)) {
        return 1;
    }

    AegisCoreHandle *core = aegis_core_create();
    if (core == 0) {
        return 2;
    }

    AegisExecutionResultHandle *result = 0;
    uint32_t code = aegis_execute_line(core, aegis_sv("echo hello"), &result);
    if (code != AEGIS_OK || result == 0) {
        aegis_core_release(core);
        return 3;
    }

    uint32_t status = aegis_result_status_code(result);
    aegis_result_release(result);
    aegis_core_release(core);

    return status == AEGIS_EXECUTION_STATUS_SUCCESS ? 0 : 4;
}
C_EOF

cc \
  -I "$ROOT_DIR/include" \
  "$SOURCE_FILE" \
  "$ROOT_DIR/target/release/libaegis_ffi.a" \
  -o "$BINARY_FILE"

"$BINARY_FILE"
