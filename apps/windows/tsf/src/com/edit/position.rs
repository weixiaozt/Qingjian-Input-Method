//! 非组句提示的定位：只读光标矩形，不更改选区或输入状态。
use super::anchor::{anchor_rect, selection_range};
use crate::com::service::SharedClient;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, TF_ES_READ,
};
use windows::core::{Result, implement};

#[implement(ITfEditSession)]
struct Position {
    context: ITfContext,

    engine: SharedClient,
}

impl ITfEditSession_Impl for Position_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        if let Some(range) = selection_range(&self.context, ec)
            && let Ok(mut guard) = self.engine.try_borrow_mut()
            && let Some(client) = guard.as_mut()
        {
            let _ = client.position_candidates(anchor_rect(&self.context, ec, &range));
        }
        Ok(())
    }
}

pub(crate) fn request(context: &ITfContext, client_id: u32, engine: SharedClient) -> Result<()> {
    let session: ITfEditSession = Position {
        context: context.clone(),
        engine,
    }
    .into();
    super::update::request(context, client_id, session, TF_ES_READ)
}
