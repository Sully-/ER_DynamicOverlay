#include "er_overlay_plugin.h"

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>

namespace {

struct DemoPlugin {
    uint64_t tick_ms = 0;
    ErMetricDesc descs[3]{};
    std::string text;
    std::string overlay_version;
    std::string base_dir;
    void (*log)(uint32_t level, const char* msg) = nullptr;

    int64_t progress() const { return static_cast<int64_t>((tick_ms / 1000) % 61); }

    const char* rank() const {
        const int64_t p = progress();
        if (p <= 14) {
            return "C";
        }
        if (p <= 29) {
            return "B";
        }
        if (p <= 44) {
            return "A";
        }
        return "S";
    }

    void host_log(uint32_t level, const char* msg) const {
        if (log != nullptr) {
            log(level, msg);
        }
    }
};

ErMetricSample unavailable() {
    ErMetricSample sample{};
    sample.available = 0;
    return sample;
}

}  // namespace

extern "C" {

ER_OVERLAY_PLUGIN_API uint32_t er_overlay_plugin_abi_version(void) {
    return ER_OVERLAY_PLUGIN_ABI_VERSION;
}

ER_OVERLAY_PLUGIN_API void* er_overlay_plugin_create(const ErHostInfo* host) {
    try {
        auto* plugin = new DemoPlugin();
        if (host != nullptr) {
            if (host->overlay_version != nullptr) {
                plugin->overlay_version = host->overlay_version;
            }
            if (host->base_dir != nullptr) {
                plugin->base_dir = host->base_dir;
            }
            plugin->log = host->log;
        }

        plugin->descs[0].id = "demo_cpp.uptime";
        plugin->descs[0].kind = ER_METRIC_TIME_MS;
        plugin->descs[1].id = "demo_cpp.progress";
        plugin->descs[1].kind = ER_METRIC_COUNT;
        plugin->descs[2].id = "demo_cpp.rank";
        plugin->descs[2].kind = ER_METRIC_TEXT;

        char msg[512];
        std::snprintf(
            msg,
            sizeof(msg),
            "demo_cpp: overlay %s @ %s",
            plugin->overlay_version.c_str(),
            plugin->base_dir.c_str());
        plugin->host_log(ER_LOG_INFO, msg);
        return plugin;
    } catch (...) {
        return nullptr;
    }
}

ER_OVERLAY_PLUGIN_API const ErMetricDesc* er_overlay_plugin_metrics(void* ctx, size_t* out_len) {
    try {
        if (out_len == nullptr) {
            return nullptr;
        }
        auto* plugin = static_cast<DemoPlugin*>(ctx);
        if (plugin == nullptr) {
            *out_len = 0;
            return nullptr;
        }
        *out_len = 3;
        return plugin->descs;
    } catch (...) {
        if (out_len != nullptr) {
            *out_len = 0;
        }
        return nullptr;
    }
}

ER_OVERLAY_PLUGIN_API void er_overlay_plugin_poll(void* ctx, uint64_t tick_ms) {
    try {
        auto* plugin = static_cast<DemoPlugin*>(ctx);
        if (plugin == nullptr) {
            return;
        }
        plugin->tick_ms = tick_ms;
        plugin->text.clear();
    } catch (...) {
    }
}

ER_OVERLAY_PLUGIN_API ErMetricSample er_overlay_plugin_sample(void* ctx, size_t index) {
    try {
        auto* plugin = static_cast<DemoPlugin*>(ctx);
        if (plugin == nullptr) {
            return unavailable();
        }

        ErMetricSample sample{};
        sample.available = 1;
        switch (index) {
            case 0:
                sample.value = static_cast<int64_t>(plugin->tick_ms);
                return sample;
            case 1:
                sample.value = plugin->progress();
                sample.max = 60;
                sample.has_max = 1;
                return sample;
            case 2:
                plugin->text = plugin->rank();
                sample.text = plugin->text.c_str();
                return sample;
            default:
                return unavailable();
        }
    } catch (...) {
        return unavailable();
    }
}

ER_OVERLAY_PLUGIN_API void er_overlay_plugin_destroy(void* ctx) {
    try {
        delete static_cast<DemoPlugin*>(ctx);
    } catch (...) {
    }
}

}  // extern "C"
