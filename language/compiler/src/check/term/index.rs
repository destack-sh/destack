use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CallCallee, CallTerm, CheckState, FormTerm, MemberCallTerm, MemberDecision,
    MemberProjectionOrigin, MemberProtocol, MemberResolution, MemberTargetResolution, Origin,
    SubscriptMethod, TypeOperand, TypeTerm, VariableId,
};

/// Runtime index access term.
///
/// ```ds
/// values[index]
/// tuple[0]
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct IndexTerm {
    /// The source index expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The selected index form.
    pub(in crate::check) kind: IndexKind,
    /// The indexed receiver type.
    pub(in crate::check) receiver: TypeOperand,
    /// The index expression type.
    pub(in crate::check) index: TypeOperand,
    /// The direct structural key when syntax makes it obvious.
    pub(in crate::check) key: Option<dir::StaticKey>,
}

/// Source-selected index form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum IndexKind {
    /// Element index access.
    ///
    /// ```ds
    /// values[index]
    /// ```
    Element,
    /// Slice index access.
    ///
    /// ```ds
    /// values[start..end]
    /// ```
    Slice,
}

/// Runtime index set term.
///
/// ```ds
/// values[index] = value
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct IndexSetTerm {
    /// The source assignment expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The indexed receiver type.
    pub(in crate::check) receiver: TypeOperand,
    /// The index expression type.
    pub(in crate::check) index: TypeOperand,
    /// The assigned value type.
    pub(in crate::check) value: TypeOperand,
    /// The direct structural key when syntax makes it obvious.
    pub(in crate::check) key: Option<dir::StaticKey>,
}

/// Receiver type prepared for structural index lookup.
struct IndexReceiver {
    /// The reduced receiver type.
    ty: TypeTerm,
    /// Whether the indexed receiver is readonly.
    is_readonly: bool,
}

impl IndexTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> smallvec::SmallVec<[VariableId; 2]> {
        let mut variables = smallvec::SmallVec::new();

        variables.extend(self.receiver.referenced_variables(state));
        variables.extend(self.index.referenced_variables(state));

        variables
    }
}

impl IndexSetTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> smallvec::SmallVec<[VariableId; 2]> {
        let mut variables = smallvec::SmallVec::new();

        variables.extend(self.receiver.referenced_variables(state));
        variables.extend(self.index.referenced_variables(state));
        variables.extend(self.value.referenced_variables(state));

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
    ) -> CompilerResult<Option<TypeTerm>> {
        if let Some(term) = self.reduce_structural_index_type(origin, module, index)? {
            return Ok(Some(term));
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
    ) -> CompilerResult<Option<TypeTerm>> {
        if self.reduce_structural_index_set(origin, module, set)? {
            let Some(value) = self.type_operand_term(set.value)? else {
                return Ok(None);
            };

            return Ok(Some(value));
        }

        let call = self.index_set_call_term(set)?;
        let reduction = self.reduce_call_term(origin, module, &call)?;
        if reduction.is_none() {
            return Ok(None);
        };

        let Some(value) = self.type_operand_term(set.value)? else {
            return Ok(None);
        };

        Ok(Some(value))
    }

    /// Expect structural or protocol index selection to produce the expected result.
    pub(in crate::check) fn expect_index_term(
        &mut self,
        origin: Origin,
        index: &IndexTerm,
        result: VariableId,
    ) -> CompilerResult<()> {
        if let Some(term) = self.reduce_structural_index_type(origin, result.module, index)? {
            let term = self.inference.push_term(term);

            return self.reduce_contextual_type_assignability(origin, term, result);
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
    ) -> CompilerResult<()> {
        self.reduce_contextual_type_assignability(origin, set.value, result)
    }

    /// Reduce one structural tuple or shape index.
    fn reduce_structural_index_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        index: &IndexTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(receiver) = self.reduce_index_receiver_type(origin, index.receiver)? else {
            return Ok(None);
        };

        if let Some(term) = self.index_array_type(index, &receiver.ty, receiver.is_readonly)? {
            return Ok(Some(term));
        }

        let Some(key) = index.key else {
            return Ok(None);
        };
        let term = self.resolve_member_type(origin, module, &receiver.ty, &key, &[])?;
        if term.is_some() {
            self.select_builtin_index_member(index, dir::BuiltinMember::Index)?;
        }

        Ok(term)
    }

    /// Return one reduced receiver for structural index lookup.
    fn reduce_index_receiver_type(
        &mut self,
        origin: Origin,
        receiver: TypeOperand,
    ) -> CompilerResult<Option<IndexReceiver>> {
        let Some(receiver) = self.reduce_type_operand(origin, receiver)? else {
            return Ok(None);
        };
        let Some(receiver) = self.type_operand_term(receiver)? else {
            return Ok(None);
        };

        let TypeTerm::Form { form, payload } = receiver else {
            return Ok(Some(IndexReceiver {
                ty: receiver,
                is_readonly: false,
            }));
        };
        let Some(payload) = self.type_operand_term(payload)? else {
            return Ok(None);
        };
        let is_readonly = matches!(self.inference.term(form), FormTerm::Readonly);

        Ok(Some(IndexReceiver {
            ty: payload,
            is_readonly,
        }))
    }

    /// Return a structural array or slice subscript result.
    fn index_array_type(
        &mut self,
        index: &IndexTerm,
        receiver: &TypeTerm,
        is_readonly: bool,
    ) -> CompilerResult<Option<TypeTerm>> {
        let element = match receiver {
            TypeTerm::Array { element } => *element,
            TypeTerm::FixedArray { element, length: _ } => *element,
            TypeTerm::Slice { element } => *element,
            _ => return Ok(None),
        };
        if index.kind == IndexKind::Slice {
            self.select_builtin_index_member(index, dir::BuiltinMember::Slice)?;

            let term = TypeTerm::Slice { element };
            if is_readonly {
                let payload = self.inference.push_term(term);
                let form = self.inference.push_term(FormTerm::Readonly);

                return Ok(Some(TypeTerm::Form {
                    form,
                    payload: payload.into(),
                }));
            }

            return Ok(Some(term));
        }

        self.select_builtin_index_member(index, dir::BuiltinMember::Index)?;

        self.type_operand_term(element)
    }

    /// Select one builtin subscript member resolution.
    fn select_builtin_index_member(
        &mut self,
        index: &IndexTerm,
        builtin: dir::BuiltinMember,
    ) -> CompilerResult<()> {
        let resolution = MemberResolution {
            source: index.source,
            receiver: index.receiver,
            target: MemberTargetResolution::Builtin(builtin),
        };

        self.inference
            .select_member(index.source, MemberDecision::Resolved(resolution))?;

        Ok(())
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
        let Some(receiver) = self.reduce_index_receiver_type(origin, set.receiver)? else {
            return Ok(false);
        };
        if receiver.is_readonly {
            return Ok(false);
        }
        let Some(term) = self.resolve_member_type(origin, module, &receiver.ty, &key, &[])? else {
            return Ok(false);
        };
        let term = self.inference.push_term(term);
        self.reduce_contextual_type_assignability(origin, set.value, term)?;

        Ok(true)
    }

    /// Return the protocol call represented by one index operation.
    fn index_call_term(&mut self, index: &IndexTerm) -> CompilerResult<CallTerm> {
        let key = {
            let strings = &self.module(index.source.module_id).strings;

            SubscriptMethod::Index.key(strings)
        };
        let member = MemberCallTerm {
            origin: MemberProjectionOrigin::Protocol {
                protocol: MemberProtocol {
                    item: dir::LanguageItem::Index,
                    arguments: Vec::new().into(),
                },
            },
            receiver: index.receiver,
            key,
            arguments: Vec::new().into(),
        };
        let member = self.inference.push_term(member);

        Ok(CallTerm {
            source: index.source,
            callee: CallCallee::Member(member),
            generic_arguments: Default::default(),
            argument_types: vec![index.index.into()].into(),
            arguments: Default::default(),
        })
    }

    /// Return the protocol call represented by one index set.
    fn index_set_call_term(&mut self, set: &IndexSetTerm) -> CompilerResult<CallTerm> {
        let key = {
            let strings = &self.module(set.source.module_id).strings;

            SubscriptMethod::IndexSet.key(strings)
        };
        let member = MemberCallTerm {
            origin: MemberProjectionOrigin::Protocol {
                protocol: MemberProtocol {
                    item: dir::LanguageItem::IndexSet,
                    arguments: Vec::new().into(),
                },
            },
            receiver: set.receiver,
            key,
            arguments: Vec::new().into(),
        };
        let member = self.inference.push_term(member);

        Ok(CallTerm {
            source: set.source,
            callee: CallCallee::Member(member),
            generic_arguments: Default::default(),
            argument_types: vec![set.index.into(), set.value.into()].into(),
            arguments: Default::default(),
        })
    }
}
