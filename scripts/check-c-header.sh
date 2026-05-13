#!/usr/bin/env sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT_DIR"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
case "$TARGET_DIR" in
  /*) ;;
  *) TARGET_DIR="$ROOT_DIR/$TARGET_DIR" ;;
esac

BUILD_DIR="$TARGET_DIR/header-smoke"
SOURCE_FILE="$BUILD_DIR/header_smoke.c"
BINARY_FILE="$BUILD_DIR/header_smoke"

mkdir -p "$BUILD_DIR"

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
TARGET_TRIPLE="${CARGO_BUILD_TARGET:-$HOST_TRIPLE}"
if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
  cargo build -p aegis-ffi --release --target "$CARGO_BUILD_TARGET"
  LIB_DIR="$TARGET_DIR/$CARGO_BUILD_TARGET/release"
else
  cargo build -p aegis-ffi --release
  LIB_DIR="$TARGET_DIR/release"
fi

DEFAULT_CFLAGS=""
case "$HOST_TRIPLE" in
  aarch64-apple-darwin) DEFAULT_CFLAGS="-arch arm64" ;;
  x86_64-apple-darwin) DEFAULT_CFLAGS="-arch x86_64" ;;
esac

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

static uint32_t host_echo(
    AegisExecutionContextHandle *context,
    size_t argc,
    const AegisStringView *argv,
    void *userdata
) {
    (void)argc;
    (void)argv;
    if (userdata == 0) {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }
    return aegis_context_write_text(context, aegis_sv("hello from c host"));
}

int main(void) {
    uint32_t error_codes =
        AEGIS_ERROR_PARSE |
        AEGIS_ERROR_REGISTRY |
        AEGIS_ERROR_COMMAND_NOT_FOUND |
        AEGIS_ERROR_INVALID_ARGUMENT |
        AEGIS_ERROR_PERMISSION_DENIED |
        AEGIS_ERROR_CANCELLED |
        AEGIS_ERROR_TIMEOUT |
        AEGIS_ERROR_SCRIPT |
        AEGIS_ERROR_PLUGIN |
        AEGIS_ERROR_INTERNAL |
        AEGIS_ERROR_FFI |
        AEGIS_ERROR_PANIC;
    uint32_t output_codes =
        AEGIS_OUTPUT_CHANNEL_UNKNOWN |
        AEGIS_OUTPUT_CHANNEL_MAIN |
        AEGIS_OUTPUT_CHANNEL_DEBUG |
        AEGIS_OUTPUT_CHANNEL_SYSTEM |
        AEGIS_OUTPUT_KIND_UNKNOWN |
        AEGIS_OUTPUT_KIND_TEXT |
        AEGIS_OUTPUT_KIND_JSON |
        AEGIS_OUTPUT_KIND_TABLE |
        AEGIS_OUTPUT_KIND_LOG |
        AEGIS_OUTPUT_KIND_WARNING |
        AEGIS_OUTPUT_KIND_ERROR |
        AEGIS_OUTPUT_KIND_PROGRESS |
        AEGIS_OUTPUT_KIND_STATE_CHANGED |
        AEGIS_OUTPUT_KIND_DIAGNOSTIC |
        AEGIS_EXECUTION_STATUS_UNKNOWN |
        AEGIS_EXECUTION_STATUS_SUCCESS |
        AEGIS_EXECUTION_STATUS_FAILED |
        AEGIS_EXECUTION_STATUS_BLOCKED;
    if (error_codes == 0 || output_codes == 0) {
        return 1;
    }

    AegisAbiVersion version = aegis_abi_version();
    if (version.size != sizeof(AegisAbiVersion)) {
        return 2;
    }

    AegisCoreHandle *core = aegis_core_create();
    if (core == 0) {
        return 3;
    }

    AegisPluginHandle *plugin = 0;
    uint32_t code = aegis_register_plugin(
        core,
        aegis_sv("c.host"),
        aegis_sv("C Host"),
        aegis_sv("0.1.0"),
        &plugin
    );
    if (code != AEGIS_OK || plugin == 0) {
        aegis_core_release(core);
        return 4;
    }

    code = aegis_register_cvar(
        plugin,
        aegis_sv("c_mode"),
        aegis_sv("normal"),
        0u,
        aegis_sv("C mode")
    );
    if (code != AEGIS_OK) {
        aegis_plugin_release(plugin);
        aegis_core_release(core);
        return 5;
    }

    int userdata = 1;
    code = aegis_register_command(
        plugin,
        aegis_sv("c_echo"),
        0u,
        aegis_sv("C echo"),
        host_echo,
        &userdata
    );
    if (code != AEGIS_OK) {
        aegis_plugin_release(plugin);
        aegis_core_release(core);
        return 6;
    }

    AegisExecutionResultHandle *result = 0;
    code = aegis_execute_line(core, aegis_sv("c_echo alpha"), &result);
    if (code != AEGIS_OK || result == 0) {
        aegis_plugin_release(plugin);
        aegis_core_release(core);
        return 7;
    }

    uint32_t status = aegis_result_status_code(result);
    uint32_t error = aegis_result_error_code(result);
    size_t count = aegis_result_output_count(result);
    uint64_t command_id = aegis_result_output_command_id_at(result, 0);
    uint64_t sequence = aegis_result_output_sequence_at(result, 0);
    uint32_t channel = aegis_result_output_channel_at(result, 0);
    uint32_t kind = aegis_result_output_kind_at(result, 0);
    const char *expected_payload = "hello from c host";
    AegisStringView payload = aegis_result_output_payload_at(result, 0);
    int payload_matches =
        payload.ptr != 0 &&
        payload.len == strlen(expected_payload) &&
        memcmp(payload.ptr, expected_payload, payload.len) == 0;
    aegis_result_release(result);
    if (
        status != AEGIS_EXECUTION_STATUS_SUCCESS ||
        error != AEGIS_OK ||
        count != 1 ||
        command_id == 0 ||
        sequence == 0 ||
        channel != AEGIS_OUTPUT_CHANNEL_MAIN ||
        kind != AEGIS_OUTPUT_KIND_TEXT ||
        !payload_matches
    ) {
        aegis_plugin_release(plugin);
        aegis_core_release(core);
        return 8;
    }

    result = 0;
    code = aegis_execute_script(
        core,
        aegis_sv("smoke.cfg"),
        aegis_sv("echo script"),
        0,
        &result
    );
    if (code != AEGIS_OK || result == 0) {
        aegis_plugin_release(plugin);
        aegis_core_release(core);
        return 9;
    }
    aegis_result_release(result);

    aegis_plugin_release(plugin);
    aegis_core_release(core);

    return 0;
}
C_EOF

if [ "$TARGET_TRIPLE" != "$HOST_TRIPLE" ]; then
  ${CC:-cc} ${CFLAGS:-$DEFAULT_CFLAGS} \
    -I "$ROOT_DIR/include" \
    -c "$SOURCE_FILE" \
    -o "$BUILD_DIR/header_smoke.o"
  printf '%s\n' "Skipping C header link/run smoke for non-host target $TARGET_TRIPLE (host: $HOST_TRIPLE)."
  exit 0
fi

${CC:-cc} ${CFLAGS:-$DEFAULT_CFLAGS} \
  -I "$ROOT_DIR/include" \
  "$SOURCE_FILE" \
  "$LIB_DIR/libaegis_ffi.a" \
  -o "$BINARY_FILE"

"$BINARY_FILE"
