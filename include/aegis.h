#ifndef AEGIS_H
#define AEGIS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifndef AEGIS_API
#if defined(_WIN32) && !defined(AEGIS_STATIC)
#define AEGIS_API __declspec(dllimport)
#else
#define AEGIS_API
#endif
#endif

#define AEGIS_OK 0u
#define AEGIS_ERROR_PARSE 100u
#define AEGIS_ERROR_REGISTRY 200u
#define AEGIS_ERROR_COMMAND_NOT_FOUND 300u
#define AEGIS_ERROR_INVALID_ARGUMENT 400u
#define AEGIS_ERROR_PERMISSION_DENIED 500u
#define AEGIS_ERROR_CANCELLED 600u
#define AEGIS_ERROR_TIMEOUT 700u
#define AEGIS_ERROR_SCRIPT 800u
#define AEGIS_ERROR_PLUGIN 900u
#define AEGIS_ERROR_INTERNAL 1000u
#define AEGIS_ERROR_FFI 1100u
#define AEGIS_ERROR_PANIC 10000u

#define AEGIS_OUTPUT_CHANNEL_UNKNOWN 0u
#define AEGIS_OUTPUT_CHANNEL_MAIN 1u
#define AEGIS_OUTPUT_CHANNEL_DEBUG 2u
#define AEGIS_OUTPUT_CHANNEL_SYSTEM 3u

#define AEGIS_OUTPUT_KIND_UNKNOWN 0u
#define AEGIS_OUTPUT_KIND_TEXT 1u
#define AEGIS_OUTPUT_KIND_JSON 2u
#define AEGIS_OUTPUT_KIND_TABLE 3u
#define AEGIS_OUTPUT_KIND_LOG 4u
#define AEGIS_OUTPUT_KIND_WARNING 5u
#define AEGIS_OUTPUT_KIND_ERROR 6u
#define AEGIS_OUTPUT_KIND_PROGRESS 7u
#define AEGIS_OUTPUT_KIND_STATE_CHANGED 8u
#define AEGIS_OUTPUT_KIND_DIAGNOSTIC 9u

#define AEGIS_EXECUTION_STATUS_UNKNOWN 0u
#define AEGIS_EXECUTION_STATUS_SUCCESS 1u
#define AEGIS_EXECUTION_STATUS_FAILED 2u
#define AEGIS_EXECUTION_STATUS_BLOCKED 3u

typedef struct AegisCoreHandle AegisCoreHandle;
typedef struct AegisPluginHandle AegisPluginHandle;
typedef struct AegisExecutionContextHandle AegisExecutionContextHandle;
typedef struct AegisExecutionResultHandle AegisExecutionResultHandle;

/*
 * Borrowed UTF-8 bytes.
 *
 * The caller owns ptr and must keep it valid for len bytes for the duration of
 * the ABI call. Strings do not need to be NUL-terminated. Use {0, 0} for an
 * empty string view; a NULL pointer with a non-zero len is invalid.
 */
typedef struct AegisStringView {
    const uint8_t *ptr;
    size_t len;
} AegisStringView;

typedef struct AegisAbiVersion {
    size_t size;
    uint16_t major;
    uint16_t minor;
    uint16_t patch;
    uint32_t core_api_version;
} AegisAbiVersion;

typedef uint32_t (*AegisCommandCallback)(
    AegisExecutionContextHandle *context,
    size_t argc,
    const AegisStringView *argv,
    void *userdata
);

AEGIS_API AegisAbiVersion aegis_abi_version(void);

/*
 * Create and release a core handle.
 *
 * aegis_core_release accepts NULL. Releasing the same non-NULL handle more
 * than once, or using any handle after release, is undefined behavior.
 */
AEGIS_API AegisCoreHandle *aegis_core_create(void);
AEGIS_API void aegis_core_release(AegisCoreHandle *handle);

/*
 * Execute one command line.
 *
 * out_result must be a writable pointer. On every non-NULL out_result call,
 * Aegis initializes *out_result to NULL before validation. When the return code
 * is AEGIS_OK and *out_result is non-NULL, the caller owns the result handle
 * and must release it with aegis_result_release.
 */
AEGIS_API uint32_t aegis_execute_line(
    AegisCoreHandle *core,
    AegisStringView input,
    AegisExecutionResultHandle **out_result
);

/*
 * Execute one script with default options.
 *
 * options_ptr is reserved and must be NULL in this ABI version. Result
 * ownership follows aegis_execute_line.
 */
AEGIS_API uint32_t aegis_execute_script(
    AegisCoreHandle *core,
    AegisStringView source_name,
    AegisStringView script,
    const void *options_ptr,
    AegisExecutionResultHandle **out_result
);

/*
 * Result accessors tolerate NULL result handles by returning zero or empty
 * views. Payload views are borrowed from the result and become invalid when
 * aegis_result_release is called.
 */
AEGIS_API void aegis_result_release(AegisExecutionResultHandle *result);
AEGIS_API uint32_t aegis_result_status_code(const AegisExecutionResultHandle *result);
AEGIS_API uint32_t aegis_result_error_code(const AegisExecutionResultHandle *result);
AEGIS_API size_t aegis_result_output_count(const AegisExecutionResultHandle *result);
AEGIS_API uint64_t aegis_result_output_command_id_at(const AegisExecutionResultHandle *result, size_t index);
AEGIS_API uint64_t aegis_result_output_sequence_at(const AegisExecutionResultHandle *result, size_t index);
AEGIS_API uint32_t aegis_result_output_channel_at(const AegisExecutionResultHandle *result, size_t index);
AEGIS_API uint32_t aegis_result_output_kind_at(const AegisExecutionResultHandle *result, size_t index);
AEGIS_API AegisStringView aegis_result_output_payload_at(const AegisExecutionResultHandle *result, size_t index);

/*
 * Register a plugin and plugin-owned capabilities.
 *
 * out_plugin must be writable and is initialized to NULL before validation.
 * The returned plugin handle does not unregister capabilities when released.
 * It remains valid until aegis_plugin_release or until its owning core is
 * released. Aegis serializes FFI calls per core handle, but hosts must not
 * release a handle concurrently with calls that use the same handle.
 */
AEGIS_API uint32_t aegis_register_plugin(
    AegisCoreHandle *core,
    AegisStringView id,
    AegisStringView name,
    AegisStringView version,
    AegisPluginHandle **out_plugin
);

AEGIS_API void aegis_plugin_release(AegisPluginHandle *plugin);

/*
 * Register a cvar owned by plugin. String views are copied before return.
 */
AEGIS_API uint32_t aegis_register_cvar(
    AegisPluginHandle *plugin,
    AegisStringView name,
    AegisStringView default_value,
    uint32_t flags,
    AegisStringView description
);

/*
 * Register a command owned by plugin.
 *
 * callback must be non-NULL and must remain valid until the owning core is
 * released. userdata is caller-owned and is passed back unchanged. Callbacks
 * must not reenter the same core handle while executing.
 */
AEGIS_API uint32_t aegis_register_command(
    AegisPluginHandle *plugin,
    AegisStringView name,
    uint32_t flags,
    AegisStringView description,
    AegisCommandCallback callback,
    void *userdata
);

/*
 * Write text through a callback context. The context is valid only during the
 * callback invocation that received it.
 */
AEGIS_API uint32_t aegis_context_write_text(
    AegisExecutionContextHandle *context,
    AegisStringView text
);

#ifdef __cplusplus
}
#endif

#endif
