use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CallCallee, CallTerm, CheckState, MemberCallCallee, MemberCallTerm, MemberProtocol,
    MemberResolution, MemberResolutionTarget, MemberSelection, Origin, Progress, Reduction,
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
        origin: Origin,
        module: ModuleId,
        index: &IndexTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        if let Some(term) = self.reduce_structural_index_type(origin, module, index)? {
            return Ok(Reduction::value(term));
        }

        let call = self.index_call_term(index)?;

        self.reduce_call_term(origin, module, &call)
    }

    /// Reduce one runtime index set operation.
    pub(in crate::check) fn reduce_index_set_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        set: &IndexSetTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        if self.reduce_structural_index_set(origin, module, set)? {
            return Ok(Reduction::value(TypeTerm::Variable(set.value)));
        }

        let call = self.index_set_call_term(set)?;
        let reduction = self.reduce_call_term(origin, module, &call)?;
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
        origin: Origin,
        index: &IndexTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        if let Some(term) = self.reduce_structural_index_type(origin, result.module, index)? {
            let term = self.terms.push(term);

            return self.solve_contextual_type_assignability(origin, term, result);
        }
        let call = self.index_call_term(index)?;

        self.expect_call_term(origin, &call, result)
    }

    /// Expect an index set value to satisfy the assigned value type.
    pub(in crate::check) fn expect_index_set_term(
        &mut self,
        origin: Origin,
        set: &IndexSetTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        self.solve_contextual_type_assignability(origin, set.value, result)
    }

    /// Reduce one structural tuple or shape index.
    fn reduce_structural_index_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        index: &IndexTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(receiver) = self.solved_type_term(index.receiver)? else {
            return Ok(None);
        };
        let receiver = match self.reduce_type_term(origin, &receiver)? {
            Reduction {
                value: Some(value),
                progress: _,
            } => value,
            Reduction {
                value: None,
                progress: _,
            } => receiver,
        };
        let selects_slice = self.index_syntax_selects_slice(index.source);

        if let Some(term) = self.index_array_type(index, &receiver, selects_slice) {
            return Ok(Some(term));
        }

        let Some(key) = index.key else {
            return Ok(None);
        };
        let term = self.resolve_member_type(origin, module, &receiver, &key, &[])?;
        if term.is_some() {
            self.select_builtin_index_member(index, dir::BuiltinMember::Index);
        }

        Ok(term)
    }

    /// Return a structural array or slice subscript result.
    fn index_array_type(
        &mut self,
        index: &IndexTerm,
        receiver: &TypeTerm,
        selects_slice: bool,
    ) -> Option<TypeTerm> {
        let (element, is_readonly) = match receiver {
            TypeTerm::Array { element } => (*element, false),
            TypeTerm::FixedArray {
                element,
                length: _,
                is_readonly,
            } => (*element, *is_readonly),
            TypeTerm::Slice {
                element,
                is_readonly,
            } => (*element, *is_readonly),
            _ => return None,
        };
        if selects_slice {
            self.select_builtin_index_member(index, dir::BuiltinMember::Slice);

            return Some(TypeTerm::Slice {
                element,
                is_readonly,
            });
        }

        self.select_builtin_index_member(index, dir::BuiltinMember::Index);

        Some(element.to_type_term(self))
    }

    /// Return whether this subscript uses range syntax.
    fn index_syntax_selects_slice(&self, source: dir::GlobalNodeIdAny) -> bool {
        if source.local_id.ty != dir::NodeType::Expression {
            return false;
        }
        let view = self.module(source.module_id).view();
        let source = dir::LocalNodeId::<dir::Expression>::new(source.local_id.id);
        let dir::Expression::Index {
            left: _,
            index: Some(index),
            position: _,
        } = view.get(source)
        else {
            return false;
        };

        matches!(view.get(*index), dir::Expression::RangeExpression { .. })
    }

    /// Select one builtin subscript member resolution.
    fn select_builtin_index_member(&mut self, index: &IndexTerm, builtin: dir::BuiltinMember) {
        let resolution = MemberResolution {
            source: index.source,
            receiver: index.receiver,
            target: MemberResolutionTarget::Builtin(builtin),
        };

        self.select_member(index.source, MemberSelection::Resolved(resolution));
    }

    /// Return whether one structural index set is accepted.
    fn reduce_structural_index_set(
        &mut self,
        origin: Origin,
        module: ModuleId,
        set: &IndexSetTerm,
    ) -> CompilerResult<bool> {
        let Some(key) = set.key else {
            return Ok(false);
        };
        let Some(receiver) = self.solved_type_term(set.receiver)? else {
            return Ok(false);
        };
        let Some(term) = self.resolve_member_type(origin, module, &receiver, &key, &[])? else {
            return Ok(false);
        };
        let term = self.terms.push(term);
        self.solve_contextual_type_assignability(origin, set.value, term)?;

        Ok(true)
    }

    /// Return the protocol call represented by one index operation.
    fn index_call_term(&mut self, index: &IndexTerm) -> CompilerResult<CallTerm> {
        let key = {
            let strings = &self.module(index.source.module_id).strings;

            SubscriptMethod::Index.key(strings)
        };
        let member = MemberCallTerm {
            callee: MemberCallCallee::Protocol {
                protocol: MemberProtocol {
                    item: dir::LanguageItem::Index,
                    arguments: Vec::new().into(),
                },
            },
            receiver: index.receiver,
            key,
            arguments: Vec::new().into(),
        };
        let member = self.terms.push(member);

        Ok(CallTerm {
            source: index.source,
            callee: CallCallee::Member(member),
            generic_arguments: Default::default(),
            arguments: vec![index.index.into()].into(),
        })
    }

    /// Return the protocol call represented by one index set.
    fn index_set_call_term(&mut self, set: &IndexSetTerm) -> CompilerResult<CallTerm> {
        let key = {
            let strings = &self.module(set.source.module_id).strings;

            SubscriptMethod::IndexSet.key(strings)
        };
        let member = MemberCallTerm {
            callee: MemberCallCallee::Protocol {
                protocol: MemberProtocol {
                    item: dir::LanguageItem::IndexSet,
                    arguments: Vec::new().into(),
                },
            },
            receiver: set.receiver,
            key,
            arguments: Vec::new().into(),
        };
        let member = self.terms.push(member);

        Ok(CallTerm {
            source: set.source,
            callee: CallCallee::Member(member),
            generic_arguments: Default::default(),
            arguments: vec![set.index.into(), set.value.into()].into(),
        })
    }
}
