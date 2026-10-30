//! 「翻译选中文字」的进行态。

/// 一次「翻译选中文字」的进行态。
pub(crate) struct Translation {
    /// 云端译文；`None` 表示还在等（候选窗显示「翻译中…」）。
    pub(crate) result: Option<String>,

    /// 快捷翻译对应 DLL 保存的文档范围；旧的选区翻译为 None。
    pub(crate) request: Option<u64>,

    pub(crate) failed: bool,
}
