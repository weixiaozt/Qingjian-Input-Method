//! Rust Server、Fcitx前端及测试桩共用的主协议版本；Rust测试负责与协议定义核对。
#pragma once
#include <cstdint>

namespace qingjian {
inline constexpr std::uint32_t PROTOCOL_VERSION = 13;
}
