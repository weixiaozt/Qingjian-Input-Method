//! 预览期间定时验证选区和原文；检测到改变即作废，确认时仍会再验证一次。
use super::Snapshot;
use crate::com::composition::Shared;
use std::rc::Rc;
use windows::Win32::UI::TextServices::{ITfEditSession, ITfEditSession_Impl, TF_ES_READ};
use windows::core::{Result, implement};

#[implement(ITfEditSession)]
struct ValidateSentence {
    shared: Rc<Shared>,

    snapshot: Snapshot,
}

impl ITfEditSession_Impl for ValidateSentence_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        if self
            .shared
            .sentence()
            .is_some_and(|s| s.request == self.snapshot.request)
            && !self.snapshot.valid(ec)
        {
            self.shared.cancel_sentence();
        }
        Ok(())
    }
}

pub(crate) fn request_validation(shared: Rc<Shared>, client_id: u32) {
    if let Some(snapshot) = shared.sentence() {
        let context = snapshot.context.clone();
        let session: ITfEditSession = ValidateSentence { shared, snapshot }.into();
        let _ = super::super::update::request(&context, client_id, session, TF_ES_READ);
    }
}
