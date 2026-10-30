//! Linux前端和C++测试桩的共享版本必须与Rust主协议一致；此检查在所有平台运行。
use qingjian_platform::protocol::PROTOCOL_VERSION;

// 复用Linux Server的真实协议检查，让Windows也覆盖发送端对常量的引用。
#[path = "../../../apps/linux/server/tests/protocol.rs"]
mod linux_protocol;

#[test]
fn cpp_frontends_and_mocks_use_the_current_rust_protocol() {
    let header = include_str!("../../../apps/linux/protocol_version.h");
    let declaration = header
        .lines()
        .find(|line| {
            line.trim_start()
                .starts_with("inline constexpr std::uint32_t PROTOCOL_VERSION =")
        })
        .expect("shared C++ protocol declaration");
    let version: u32 = declaration
        .split_once('=')
        .unwrap()
        .1
        .trim()
        .trim_end_matches(';')
        .parse()
        .expect("numeric protocol version");
    assert_eq!(
        version, PROTOCOL_VERSION,
        "更新主协议时须同步apps/linux/protocol_version.h"
    );
}
