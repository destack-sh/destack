use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CallTerm, CheckComponentState, MemberCallTerm, MemberProtocol, Progress, SubscriptMethod,
    TypeTerm, VariableId,
};

/// Runtime index access term.
///
/// ```ts
/// values[index]
/// tuple[0]
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct IndexTerm {
    /// The source index expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The indexed receiver type.
    pub(in crate::check) receiver: VariableId,
    /// The index expression type.
    pub(in crate::check) index: VariableId,
    /// The direct structural key when syntax makes it obvious.
    pub(in crate::check) key: Option<dir::StaticKey>,
}

/// Runtime index write term.
///
/// ```ts
/// values[index] = value
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct IndexWriteTerm {
    /// The source assignment expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The indexed receiver type.
    pub(in crate::check) receiver: VariableId,
    /// The index expression type.
    pub(in crate::check) index: VariableId,
    /// The written value type.
    pub(in crate::check) value: VariableId,
    /// The direct structural key when syntax makes it obvious.
    pub(in crate::check) key: Option<dir::StaticKey>,
}

impl IndexTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();

        variables.push(self.receiver);
        variables.push(self.index);

        variables
    }
}

impl IndexWriteTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();

        variables.push(self.receiver);
        variables.push(self.index);
        variables.push(self.value);

        variables
    }
}

impl CheckComponentState<'_> {
    /// Reduce one runtime index operation.
    pub(in crate::check) fn reduce_index_type(
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

    /// Reduce one runtime index write operation.
    pub(in crate::check) fn reduce_index_write_type(
        &mut self,
        module: ModuleId,
        write: &IndexWriteTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        if self.reduce_structural_index_write(module, write)? {
            return Ok(Some(TypeTerm::Variable(write.value)));
        }

        let call = self.index_write_call_term(write)?;
        let Some(_) = self.reduce_call_type(module, &call)? else {
            return Ok(None);
        };

        Ok(Some(TypeTerm::Variable(write.value)))
    }

    /// Apply an expected index result to structural or protocol selection.
    pub(in crate::check) fn expect_index_result(
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

    /// Apply an expected index write result to the written value.
    pub(in crate::check) fn expect_index_write_result(
        &mut self,
        write: &IndexWriteTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        self.relate_type_assignable(write.value, result)
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

    /// Return whether one structural index write is accepted.
    fn reduce_structural_index_write(
        &mut self,
        module: ModuleId,
        write: &IndexWriteTerm,
    ) -> CompilerResult<bool> {
        let Some(key) = write.key else {
            return Ok(false);
        };
        let Some(receiver) = self.solved_type_term(write.receiver)? else {
            return Ok(false);
        };
        let Some(term) = self.member_type_term(module, &receiver, &key)? else {
            return Ok(false);
        };
        let term = self.push_solved_type_variable(module, term)?;
        self.relate_type_assignable(write.value, term)?;

        Ok(true)
    }

    /// Return the protocol call represented by one index operation.
    fn index_call_term(&self, index: &IndexTerm) -> CompilerResult<CallTerm> {
        let key = {
            let strings = &self.module(index.source.module_id)?.input.strings;

            SubscriptMethod::Index.key(strings)
        };
        let member = MemberCallTerm {
            receiver: index.receiver,
            key,
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

    /// Return the protocol call represented by one index write.
    fn index_write_call_term(&self, write: &IndexWriteTerm) -> CompilerResult<CallTerm> {
        let key = {
            let strings = &self.module(write.source.module_id)?.input.strings;

            SubscriptMethod::IndexSet.key(strings)
        };
        let member = MemberCallTerm {
            receiver: write.receiver,
            key,
            arguments: Vec::new(),
            protocol: Some(MemberProtocol {
                item: dir::LanguageItem::IndexSet,
                arguments: Vec::new(),
            }),
        };

        Ok(CallTerm {
            source: write.source,
            callee: write.receiver,
            member: Some(member),
            candidates: Vec::new(),
            generic_arguments: Vec::new(),
            arguments: vec![write.index, write.value],
        })
    }
}
