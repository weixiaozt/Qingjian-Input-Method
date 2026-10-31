# 本仓库的定制开发

本仓库的展示名称为“Windows 双语整句输入实验项目”，由 weixiaozt 维护定制修改，与上游作者及团队无隶属关系，不代表其发布或背书。来源、修改日期、许可及尚未替换的程序品牌标识见 [衍生项目说明](DERIVATIVE_NOTICE.md)。

本仓库基于 [qingjian-team/qingjian](https://github.com/qingjian-team/qingjian) 的源码快照 `a1f1cdb80b1ea4d11e0f4583e01c418e39b209e4` 开发，保留上游许可证、署名与原有文档。当前是本地定制测试版本，不是上游官方发行版。

## 包含的功能

- Windows 拼音候选自动英语翻译：使用本机 Ollama/TranslateGemma，短按空格中文，长按空格双语。
- 五笔连续编码整句转换：简码与全码混输、整句候选排序、候选自动翻译；不在四码处提前提交。
- 候选只在输入法窗口显示，支持微信等应用的行内编码兼容设置。
- 保留此前的快捷句末翻译、功能键设置与诊断改动。
- [独立五笔实验台](experiments/wubi-sentence-lab/README.md)：自动切分展示、候选纠错、锁定前词、模拟上屏与反馈。

## 源码入口

| 目录 | 内容 |
|---|---|
| `crates/qingjian-core/src/code_sentence/` | 平台无关的五笔整句解码 |
| `crates/qingjian-core/src/engine/auto_translation/` | 候选翻译和空格确认生命周期 |
| `apps/windows/server/` | Windows 候选、配置与事件接入 |
| `apps/windows/tsf/` | 输入法按键、编辑会话及焦点校验 |
| `apps/settings/` | 设置页面 |
| `assets/schemes/wubi86-sentence/` | 单独的五笔整句测试码表 |
| `experiments/wubi-sentence-lab/` | 独立实验程序及浏览器界面 |

## Windows 构建与验证

使用 `rust-toolchain.toml` 指定的工具链及 Windows C++ 构建环境。完整依赖、数据准备和部署流程见 [Windows 文档](apps/windows/README.md)。

```powershell
cargo build --release --locked -p qingjian-windows-server -p qingjian-windows-tsf -p qingjian-settings
cargo test --locked -p qingjian-core -p qingjian-platform -p qingjian-windows-server -p qingjian-settings
```

Windows 输入法使用协议 13。部署时须保证 DLL 与 Server 的功能兼容；已打开的软件可能仍加载旧 DLL。请先备份已有程序和配置，保存正在输入的内容，再依照 Windows 部署流程更新。此仓库不包含个人安装目录或自动覆盖本机程序的脚本。

五笔测试码表目录也需部署到青简的 `assets/schemes/wubi86-sentence/`。在设置中关闭拼音、选择该形码方案，开启“五笔连续整句”和“候选自动翻译”；行内编码兼容设置使用“只在候选窗口”。详见[五笔整句文档](docs/user/input/wubi-sentence.md)。

关键配置示例（合并到自己的配置，不要直接覆盖完整配置）：

```toml
[general]
scheme = "none"
code = "wubi86-sentence"
preedit = "window"

[quick_translation]
enabled = false
candidate_enabled = true
wubi_sentence = true
hold_ms = 350
candidate_debounce_ms = 250
base_url = "http://127.0.0.1:11434/v1"
model = "translategemma:4b"
```

`Enter` / 短按空格确认中文，长按空格确认双语；输入法不会自动发送聊天消息。切回拼音时关闭形码、选择全拼。关闭“五笔连续整句”可恢复普通形码规则。

## 已做的验证与限制

本地相关回归曾通过 926 项，另有 CLI 首选核验、真实本机模型联调和设置页面渲染检查。实验台已做浏览器中的短按、长按、取消等待和候选纠错验证。统计对应开发时的测试集合，不代替其他系统或应用的兼容性验收。

拼音版已由用户在实际窗口测试；五笔集成版仍处于微信等应用试用阶段。macOS/Linux 尚未接入这些 Windows 设置与确认方式。实验台的点击分词锁定界面尚未接入系统候选窗。

仓库只包含源码、码表、测试与文档；不上传模型权重、安装包、个人输入日志、反馈、配置、密钥及运行时缓存。

## 长按切换输入方案

Windows 可在快捷键设置中启用长按右 Ctrl 切换全拼/五笔整句，默认600毫秒，松开后执行；组合键及鼠标操作不触发。配置为 `[shortcut] right_control_scheme_switch = true` 和 `right_control_scheme_hold_ms = 600`。已有未确认内容时拒绝切换并保留文字。
