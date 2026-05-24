use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CallTerm, CheckComponentState, IndexTerm, MemberCallTerm, MemberProtocol, TypeTerm, VariableId,
};

use super::queue::Progress;

impl CheckComponentState<'_> {
    /// Reduce one runtime index operation.
    pub(super) fn reduce_index_type(
        &mut self,
        module: ModuleId,
        index: &IndexTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        if let Some(term) = self.reduce_structural_index_type(module, index)? {
            return Ok(Some(term));
        }

        let call = self.index_call_term(index)?;

        self.reduce_call_type(module, &call)
    }

    /// Apply an expected index result to structural or protocol selection.
    pub(super) fn expect_index_result(
        &mut self,
        index: &IndexTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        if let Some(term) = self.reduce_structural_index_type(result.module, index)? {
            let term = self.push_solved_type_variable(result.module, term)?;

            return self.relate_type_assignable(term, result);
        }
        let call = self.index_call_term(index)?;

        self.expect_call_result(&call, result)
    }

    /// Reduce one structural tuple or shape index.
    fn reduce_structural_index_type(
        &mut self,
        module: ModuleId,
        index: &IndexTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(key) = index.key else {
            return Ok(None);
        };
        let Some(receiver) = self.solved_type_term(index.receiver)? else {
            return Ok(None);
        };

        self.member_type_term(module, &receiver, &key)
    }

    /// Return the protocol call represented by one index operation.
    fn index_call_term(&self, index: &IndexTerm) -> CompilerResult<CallTerm> {
        let name = self
            .module(index.source.module_id)?
            .input
            .strings
            .intern("index");
        let member = MemberCallTerm {
            receiver: index.receiver,
            key: dir::StaticKey::Name(name),
            arguments: Vec::new(),
            protocol: Some(MemberProtocol {
                item: dir::LanguageItem::Index,
                arguments: Vec::new(),
            }),
        };

        Ok(CallTerm {
            source: index.source,
            callee: index.receiver,
            member: Some(member),
            candidates: Vec::new(),
            generic_arguments: Vec::new(),
            arguments: vec![index.index],
        })
    }
}
