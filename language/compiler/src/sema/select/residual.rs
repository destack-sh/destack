use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, FlowSite, Origin};

impl CheckState<'_> {
    /// Select the branch call one try implementor splits through.
    pub(in crate::sema) fn select_try_branch(
        &mut self,
        origin: Origin,
        operand: FlowSite,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Call>> {
        // leave a nullish operand to branch on its own absent case
        if self.split_nullish_type(origin, value)?.is_some() {
            return Ok(None);
        }

        // select the protocol's branch over the operand
        let receiver = self.expression_value(operand, value)?;
        let key = dir::StaticKey::Name(self.strings().intern("branch"));
        let selected = self.select_language_protocol_call(
            origin,
            receiver,
            value,
            dir::MemberSpace::Instance,
            key,
            dir::LanguageItem::Try,
            &[],
            &[],
            &[],
        )?;

        Ok(selected.and_then(|(_, call)| match call.resolution {
            dir::OperationResolution::One(call) => Some(call),
            dir::OperationResolution::Union { .. } => None,
        }))
    }

    /// Select the call rebuilding one propagation target from its residual.
    pub(in crate::sema) fn select_from_residual(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        target: dir::GlobalTypeId,
        residual: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Call>> {
        // transfer a nullish residual as written
        if self.split_nullish_type(origin, residual)?.is_some() {
            return Ok(None);
        }

        // select the protocol's static constructor over the target
        let site = self.visit_site(node)?;
        let receiver = self.expression_value(site, target)?;
        let key = dir::StaticKey::Name(self.strings().intern("fromResidual"));
        let selected = self.select_language_protocol_call(
            origin,
            receiver,
            target,
            dir::MemberSpace::Static,
            key,
            dir::LanguageItem::FromResidual,
            &[residual],
            &[residual],
            &[dir::ArgumentSource::Supplied(0)],
        )?;

        Ok(selected.and_then(|(_, call)| match call.resolution {
            dir::OperationResolution::One(call) => Some(call),
            dir::OperationResolution::Union { .. } => None,
        }))
    }
}
