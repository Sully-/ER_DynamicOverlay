#ifndef ER_OVERLAY_PLUGIN_H
#define ER_OVERLAY_PLUGIN_H

/*
 * Elden Ring overlay metric-plugin ABI, version 1.
 *
 * A plugin is a Windows x64 DLL that exports the six functions below. It returns numbers.
 * Label, icon, show_max and tile placement belong to the layout file, not to the plugin.
 *
 * The Rust SDK (sdk/er_overlay_plugin_sdk) emits these exports for you. Use this header
 * when writing a plugin in C or C++.
 *
 * Rules:
 * - x64 only. Struct layout is part of the contract; do not reorder or resize fields.
 * - Every call happens on one thread, sequentially. A plugin is not required to be thread-safe
 *   unless it starts its own thread.
 * - Do not let a C++ exception or a Rust panic unwind across these functions.
 * - Do heavy work in er_overlay_plugin_create, never in DllMain.
 * - Do not retain the ErHostInfo pointer. overlay_version, base_dir and log stay valid until
 *   er_overlay_plugin_destroy returns; copy the strings if you need them afterwards.
 * - Pointers you return (metric ids, sample text) stay owned by the plugin. Ids must remain
 *   valid until destroy. Sample text must remain valid until the next poll.
 * - poll() is a ~250 ms heartbeat, not a sampling guarantee. For finer events, run your own
 *   thread and have sample() read the last published result.
 */

#include <stddef.h>
#include <stdint.h>

#define ER_OVERLAY_PLUGIN_ABI_VERSION 1

#define ER_METRIC_COUNT 0   /* integer, rendered "12" or "12/50" */
#define ER_METRIC_TIME_MS 1 /* milliseconds, rendered HH:MM:SS */
#define ER_METRIC_TEXT 2    /* free-form string, e.g. a rank "S" */

#define ER_LOG_ERROR 0
#define ER_LOG_WARN 1
#define ER_LOG_INFO 2
#define ER_LOG_DEBUG 3
#define ER_LOG_TRACE 4

#if defined(_WIN32) && !defined(ER_OVERLAY_PLUGIN_API)
#define ER_OVERLAY_PLUGIN_API __declspec(dllexport)
#endif
#ifndef ER_OVERLAY_PLUGIN_API
#define ER_OVERLAY_PLUGIN_API
#endif

typedef struct ErMetricDesc {
    const char* id; /* "score.total", UTF-8, NUL-terminated, valid until destroy */
    uint32_t kind;
    uint32_t _reserved[3];
} ErMetricDesc;

typedef struct ErMetricSample {
    int64_t value;
    int64_t max;        /* ignored when has_max == 0; may change every tick */
    const char* text;   /* only for ER_METRIC_TEXT; valid until the next poll */
    uint8_t available;  /* 0 -> the tile shows "---" */
    uint8_t has_max;
    uint8_t _pad[2];
    uint32_t _reserved;
} ErMetricSample;

typedef struct ErHostInfo {
    uint32_t abi_version;
    uint32_t _pad;
    const char* overlay_version;
    const char* base_dir; /* overlay DLL directory, UTF-8 */
    void (*log)(uint32_t level, const char* msg); /* may be NULL */
} ErHostInfo;

#ifdef __cplusplus
static_assert(sizeof(ErMetricDesc) == 24, "ErMetricDesc must be 24 bytes");
static_assert(sizeof(ErMetricSample) == 32, "ErMetricSample must be 32 bytes");
static_assert(sizeof(ErHostInfo) == 32, "ErHostInfo must be 32 bytes");
#elif defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
_Static_assert(sizeof(ErMetricDesc) == 24, "ErMetricDesc must be 24 bytes");
_Static_assert(sizeof(ErMetricSample) == 32, "ErMetricSample must be 32 bytes");
_Static_assert(sizeof(ErHostInfo) == 32, "ErHostInfo must be 32 bytes");
#endif

#ifdef __cplusplus
extern "C" {
#endif

ER_OVERLAY_PLUGIN_API uint32_t er_overlay_plugin_abi_version(void);

/* NULL rejects the plugin. Copy what you need from host before returning. */
ER_OVERLAY_PLUGIN_API void* er_overlay_plugin_create(const ErHostInfo* host);

/* Called once after create. out_len is required. */
ER_OVERLAY_PLUGIN_API const ErMetricDesc* er_overlay_plugin_metrics(void* ctx, size_t* out_len);

ER_OVERLAY_PLUGIN_API void er_overlay_plugin_poll(void* ctx, uint64_t tick_ms);

/* index is the position in the metrics() array, not a string lookup. */
ER_OVERLAY_PLUGIN_API ErMetricSample er_overlay_plugin_sample(void* ctx, size_t index);

ER_OVERLAY_PLUGIN_API void er_overlay_plugin_destroy(void* ctx);

#ifdef __cplusplus
}
#endif

#endif /* ER_OVERLAY_PLUGIN_H */
