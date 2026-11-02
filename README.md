# Windows 双语整句输入实验项目

**基于 [青简 Qingjian](https://github.com/qingjian-team/qingjian) 开发的非官方衍生项目，由 [weixiaozt](https://github.com/weixiaozt) 维护定制改动。与上游作者及团队无隶属关系，不代表上游发布或背书。**

原输入法及其基础能力由青简项目作者和贡献者开发。本仓库主要探索 Windows 下的拼音、五笔整句输入与本地英语翻译，属于定制测试版本。来源、修改日期及许可说明见 [衍生项目说明](DERIVATIVE_NOTICE.md)。

[English](docs/i18n/README.en.md) · [繁體中文](docs/i18n/README.zh-Hant.md) · [日本語](docs/i18n/README.ja.md) · [Español](docs/i18n/README.es.md) · [Tiếng Việt](docs/i18n/README.vi.md) · [Deutsch](docs/i18n/README.de.md)

## 本仓库的定制功能

- **拼音整句自动翻译**：当前高亮的中文词语或句子自动生成英文译文。
- **五笔连续整句**：连续输入简码和全码，由候选窗展示整句；保留字词编码提示，不拼接显示整句编码。
- **空格确认**：短按空格上屏中文；长按默认350毫秒，上屏“中文＋换行＋英文”，光标回到中文末尾，不自动发送消息。
- **长按右 Ctrl 切换**：启用后，单独按住默认600毫秒并松开，在全拼与五笔整句之间切换。
- **本地翻译**：连接本机 Ollama 的 TranslateGemma 4B 模型。
- **最近选择优先**：选中词语或整句后，下次输入相同拼音／五笔编码时排在第一位，记录保存在本机。
- **独立实验台**：在浏览器模拟输入、检查五笔分词和候选，不注册系统输入法。

这些交互目前针对 Windows 实现。仓库保留上游跨平台源码，但其他平台通过构建或测试不代表已具备这些定制功能。

## 构建与部署

从 [定制构建说明](CUSTOM_BUILD.md) 开始，再按 [Windows 工程文档](apps/windows/README.md) 和 [安装包说明](apps/windows/installer/README.md) 准备环境。

**拉取源码不等于完成安装。** 还需要完整运行词库、语言模型、五笔码表，以及单独安装的 Ollama 和翻译模型。模型权重、个人配置、输入日志和本机安装产物没有上传到本仓库。换电脑部署时，需补齐数据并重新配置。

上游官网下载的是上游版本，不保证包含本仓库的定制功能。仓库内部分工程文档仍沿用上游程序名和路径；对外发布安装包前，还需核实程序名称、图标和安装器中的品牌使用。

## 使用说明

- [拼音候选自动翻译](docs/user/cloud/pinyin-auto-translation.md)
- [五笔连续整句与字词编码提示](docs/user/input/wubi-sentence.md)
- [长按右 Ctrl 切换方案](docs/user/input/sentence-scheme-switch.md)
- [独立五笔实验台](experiments/wubi-sentence-lab/README.md)

默认翻译接口为本机服务。改成远程服务后，待翻译文本会发送到所配置的服务端。输入法只提交文字，不代替用户发送聊天消息；不同应用的换行和光标行为仍需实际验证。

## 反馈与上游来源

本仓库定制功能的问题请提交到 [本仓库 Issues](https://github.com/weixiaozt/Qingjian-Input-Method/issues)，并附提交号、Windows 版本和复现步骤。

- 上游源码与原作者：[qingjian-team/qingjian](https://github.com/qingjian-team/qingjian)
- 上游使用文档：[qingjian.app/docs](https://qingjian.app/docs)
- 本仓库开发入口：[开发约定](docs/contributing.md)

感谢上游作者、贡献者及第三方词库和工具的维护者。上游文档仅作为基础功能参考，不能代替本仓库的定制说明。

## 许可证与品牌

本衍生项目整体按 **GPL-3.0-or-later** 发布，见 [LICENSE](LICENSE)。原有代码版权归各原权利人所有；本仓库的修改不改变原作者署名及许可声明。各子目录、第三方数据和组件的独立许可也应一并保留，见 [数据来源清单](docs/design/landscape.md)。

上游明确声明“项目名称与 logo 不包含在代码授权中”。本首页使用描述性标题，不使用上游 Logo 作为本项目标识；“青简 / Qingjian”用于说明来源及兼容路径，不表示获得品牌授权。软件和安装器中遗留的标识尚未完成替换，本次文档整理不代表获得了品牌使用许可。
