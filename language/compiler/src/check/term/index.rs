use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CallTerm, CheckState, ConstraintOrigin, MemberCallTerm, MemberProtocol, Progress, Reduction,
    SubscriptMethod, TypeTerm, VariableId,
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

/// Runtime index set term.
///
/// ```ts
/// values[index] = value
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct IndexSetTerm {
    /// The source assignment expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The indexed receiver type.
    pub(in crate::check) receiver: VariableId,
    /// The index expression type.
    pub(in crate::check) index: VariableId,
    /// The assigned value type.
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

impl IndexSetTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();

        variables.push(self.receiver);
        variables.push(self.index);
        variables.push(self.value);

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one runtime index operation.
    pub(in crate::check) fn reduce_index_term(
        &mut self,
        module: ModuleId,
        index: &IndexTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        if let Some(term) = self.reduce_structural_index_type(module, index)? {
            return Ok(Reduction::value(term));
        }

        let call = self.index_call_term(index)?;

        self.reduce_call_term(module, &call)
    }

    /// Reduce one runtime index set operation.
    pub(in crate::check) fn reduce_index_set_term(
        &mut self,
        module: ModuleId,
        set: &IndexSetTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        if self.reduce_structural_index_set(module, set)? {
            return Ok(Reduction::value(TypeTerm::Variable(set.value)));
        }

        let call = self.index_set_call_term(set)?;
        let reduction = self.reduce_call_term(module, &call)?;
        if reduction.value.is_none() {
            return Ok(Reduction::progress(reduction.progress));
        };

        Ok(Reduction {
            value: Some(TypeTerm::Variable(set.value)),
            progress: reduction.progress,
        })
    }

    /// Expect structural or protocol index selection to produce the expected result.
    pub(in crate::check) fn expect_index_term(
        &mut self,
        index: &IndexTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        if let Some(term) = self.reduce_structural_index_type(result.module, index)? {
            let origin = ConstraintOrigin::Node(index.source);
            let term = self.solve_anonymous_type(result.module, origin, term)?;

            return self.solve_type_assignability(term, result);
        }
        let call = self.index_call_term(index)?;

        self.expect_call_term(&call, result)
    }

    /// Expect an index set value to satisfy the assigned value type.
    pub(in crate::check) fn expect_index_set_term(
        &mut self,
        set: &IndexSetTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        self.solve_type_assignability(set.value, result)
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

        self.resolve_member_type(module, &receiver, &key, &[])
    }

    /// Return whether one structural index set is accepted.
    fn reduce_structural_index_set(
        &mut self,
        module: ModuleId,
        set: &IndexSetTerm,
    ) -> CompilerResult<bool> {
        let Some(key) = set.key else {
            return Ok(false);
        };
        let Some(receiver) = self.solved_type_term(set.receiver)? else {
            return Ok(false);
        };
        let Some(term) = self.resolve_member_type(module, &receiver, &key, &[])? else {
            return Ok(false);
        };
        let origin = ConstraintOrigin::Node(set.source);
        let term = self.solve_anonymous_type(module, origin, term)?;
        self.solve_type_assignability(set.value, term)?;

        Ok(true)
    }

    /// Return the protocol call represented by one index operation.
    fn index_call_term(&mut self, index: &IndexTerm) -> CompilerResult<CallTerm> {
        let key = {
            let strings = &self.input(index.source.module_id).strings;

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
        let member = self.terms.push(member);

        Ok(CallTerm {
            source: index.source,
            callee: index.receiver,
            member: Some(member),
            candidates: Vec::new(),
            generic_arguments: Vec::new(),
            arguments: vec![index.index],
        })
    }

    /// Return the protocol call represented by one index set.
    fn index_set_call_term(&mut self, set: &IndexSetTerm) -> CompilerResult<CallTerm> {
        let key = {
            let strings = &self.input(set.source.module_id).strings;

            SubscriptMethod::IndexSet.key(strings)
        };
        let member = MemberCallTerm {
            receiver: set.receiver,
            key,
            arguments: Vec::new(),
            protocol: Some(MemberProtocol {
                item: dir::LanguageItem::IndexSet,
                arguments: Vec::new(),
            }),
        };
        let member = self.terms.push(member);

        Ok(CallTerm {
            source: set.source,
            callee: set.receiver,
            member: Some(member),
            candidates: Vec::new(),
            generic_arguments: Vec::new(),
            arguments: vec![set.index, set.value],
        })
    }
}
