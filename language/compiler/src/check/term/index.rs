use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    Answer, CallArgument, CallCallee, CallTerm, CheckState, Condition, FormTerm, GenericArgument,
    MemberCallTerm, MemberDecision, MemberLookup, MemberProjectionOrigin, MemberProtocol,
    MemberReceiver, MemberResolution, MemberTargetResolution, Origin, SubscriptMethod, TermId,
    TypeOperand, TypeRelation, TypeTerm, VariableId,
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// The reduced receiver operand.
    operand: TypeOperand,
    /// The reduced receiver type.
    ty: TermId<TypeTerm>,
    /// Whether the indexed receiver is readonly.
    is_readonly: bool,
}

impl CheckState<'_> {
    /// Reduce one runtime index operation.
    pub(in crate::check) fn reduce_index_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        index: IndexTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        match self.reduce_structural_index_type(origin, module, &index)? {
            Answer::Ready(Some(term)) => return Ok(Answer::Ready(term)),
            Answer::Ready(None) => {}
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }

        let call = self.index_call_term(&index)?;
        let call = self.inference.push_term(call);

        self.reduce_call_term(origin, module, call)
    }

    /// Reduce one runtime index set operation.
    pub(in crate::check) fn reduce_index_set_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        set: IndexSetTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        match self.reduce_structural_index_set(origin, module, &set)? {
            Answer::Ready(true) => {
                let Some(value) = self.type_operand_term_id(set.value)? else {
                    return Ok(Answer::pending(set.value.dependencies(self)));
                };

                return Ok(Answer::Ready(value.into()));
            }
            Answer::Ready(false) => {}
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }

        let call = self.index_set_call_term(&set)?;
        let call = self.inference.push_term(call);
        match self.reduce_call_term(origin, module, call)? {
            Answer::Ready(_) => {}
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }

        let Some(value) = self.type_operand_term_id(set.value)? else {
            return Ok(Answer::pending(set.value.dependencies(self)));
        };

        Ok(Answer::Ready(value.into()))
    }

    /// Expect structural or protocol index selection to produce the expected result.
    pub(in crate::check) fn expect_index_term(
        &mut self,
        origin: Origin,
        index: &IndexTerm,
        result: VariableId,
    ) -> CompilerResult<Answer<()>> {
        match self.reduce_structural_index_type(origin, result.module, index)? {
            Answer::Ready(Some(term)) => {
                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    term,
                    result,
                    Condition::Always,
                );

                return Ok(Answer::Ready(()));
            }
            Answer::Ready(None) => {}
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }
        let call = self.index_call_term(index)?;
        let call = self.inference.push_term(call);

        self.expect_call_term(origin, call, result)
    }

    /// Expect an index set value to satisfy the assigned value type.
    pub(in crate::check) fn expect_index_set_term(
        &mut self,
        origin: Origin,
        set: &IndexSetTerm,
        result: VariableId,
    ) -> CompilerResult<Answer<()>> {
        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            set.value,
            result,
            Condition::Always,
        );

        Ok(Answer::Ready(()))
    }

    /// Reduce one structural tuple or shape index.
    fn reduce_structural_index_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        index: &IndexTerm,
    ) -> CompilerResult<Answer<Option<TypeOperand>>> {
        let receiver = match self.reduce_index_receiver_type(origin, index.receiver)? {
            Answer::Ready(receiver) => receiver,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        if let Some(term) = self.index_array_type(index, receiver.ty, receiver.is_readonly)? {
            return Ok(Answer::Ready(Some(term)));
        }

        let Some(key) = index.key else {
            return Ok(Answer::Ready(None));
        };

        let receiver = MemberReceiver::Value(receiver.operand);
        let lookup = self.resolve_member(origin, module, &receiver, &key)?;
        let lookup = match lookup {
            MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            MemberLookup::Missing => return Ok(Answer::Ready(None)),
            lookup => lookup,
        };
        let Some(member) = self.member_type_from_lookup(module, origin, key, &[], lookup, None)?
        else {
            return Ok(Answer::Ready(None));
        };
        let Some(term) = self.type_operand_term_id(member)? else {
            return Ok(Answer::pending(member.dependencies(self)));
        };

        self.select_builtin_index_member(index, dir::BuiltinMember::Index)?;

        Ok(Answer::Ready(Some(term.into())))
    }

    /// Return one reduced receiver for structural index lookup.
    fn reduce_index_receiver_type(
        &mut self,
        origin: Origin,
        receiver: TypeOperand,
    ) -> CompilerResult<Answer<IndexReceiver>> {
        let receiver_operand = match self.reduce_type_operand(origin, receiver)? {
            Answer::Ready(receiver_operand) => receiver_operand,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let Some(receiver) = self.type_operand_term_id(receiver_operand)? else {
            return Ok(Answer::pending(receiver_operand.dependencies(self)));
        };

        let TypeTerm::Form {
            form,
            payload: payload_operand,
        } = self.inference.term(receiver)
        else {
            return Ok(Answer::Ready(IndexReceiver {
                operand: receiver_operand,
                ty: receiver,
                is_readonly: false,
            }));
        };
        let form = *form;
        let payload_operand = *payload_operand;
        let Some(payload) = self.type_operand_term_id(payload_operand)? else {
            return Ok(Answer::pending(payload_operand.dependencies(self)));
        };
        let is_readonly = matches!(self.inference.term(form), FormTerm::Readonly);

        Ok(Answer::Ready(IndexReceiver {
            operand: payload_operand,
            ty: payload,
            is_readonly,
        }))
    }

    /// Return a structural array or slice subscript result.
    fn index_array_type(
        &mut self,
        index: &IndexTerm,
        receiver: TermId<TypeTerm>,
        is_readonly: bool,
    ) -> CompilerResult<Option<TypeOperand>> {
        let element = match self.inference.term(receiver) {
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

                return Ok(Some(self.type_term_operand(TypeTerm::Form {
                    form,
                    payload: payload.into(),
                })));
            }

            return Ok(Some(self.type_term_operand(term)));
        }

        self.select_builtin_index_member(index, dir::BuiltinMember::Index)?;

        Ok(Some(element))
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
    ) -> CompilerResult<Answer<bool>> {
        let Some(key) = set.key else {
            return Ok(Answer::Ready(false));
        };
        let receiver = match self.reduce_index_receiver_type(origin, set.receiver)? {
            Answer::Ready(receiver) => receiver,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        if receiver.is_readonly {
            return Ok(Answer::Ready(false));
        }

        let receiver = MemberReceiver::Value(receiver.operand);
        let lookup = self.resolve_member(origin, module, &receiver, &key)?;
        let lookup = match lookup {
            MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            MemberLookup::Missing => return Ok(Answer::Ready(false)),
            lookup => lookup,
        };
        let Some(member) = self.member_type_from_lookup(module, origin, key, &[], lookup, None)?
        else {
            return Ok(Answer::Ready(false));
        };

        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            set.value,
            member,
            Condition::Always,
        );

        Ok(Answer::Ready(true))
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
                    arguments: vec![GenericArgument::Type(index.index)].into(),
                },
            },
            receiver: MemberReceiver::Value(index.receiver),
            key,
            arguments: Vec::new().into(),
        };
        let member = self.inference.push_term(member);

        Ok(CallTerm {
            source: index.source,
            callee: CallCallee::Member(member),
            generic_arguments: Default::default(),
            arguments: smallvec::smallvec![CallArgument {
                source: index.source,
                ty: index.index,
                is_spread: false,
            }],
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
                    arguments: vec![
                        GenericArgument::Type(set.index),
                        GenericArgument::Type(set.value),
                    ]
                    .into(),
                },
            },
            receiver: MemberReceiver::Value(set.receiver),
            key,
            arguments: Vec::new().into(),
        };
        let member = self.inference.push_term(member);

        Ok(CallTerm {
            source: set.source,
            callee: CallCallee::Member(member),
            generic_arguments: Default::default(),
            arguments: smallvec::smallvec![
                CallArgument {
                    source: set.source,
                    ty: set.index,
                    is_spread: false,
                },
                CallArgument {
                    source: set.source,
                    ty: set.value,
                    is_spread: false,
                }
            ],
        })
    }
}
