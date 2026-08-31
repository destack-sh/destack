use destack_dir as dir;

use crate::sema::{
    Cause, CauseKind, CheckState, ObligationCheck, ObligationFailure, Origin, Relation, Verdict,
    WritableTargetObligation,
};
use crate::{CompilerError, CompilerResult};

/// Assignment target selected by a source expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::sema) struct AssignmentSelection {
    /// The selected read, when the source reads before writing.
    pub(in crate::sema) read: Option<dir::ReadResolution>,
    /// The selected write.
    pub(in crate::sema) write: dir::WriteResolution,
    /// The write validation mode.
    pub(in crate::sema) mode: WriteMode,
    /// The source node for diagnostics.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
}

impl AssignmentSelection {
    /// Return the durable assignment resolution selected by this target.
    pub(in crate::sema) fn resolution(self) -> dir::AssignmentDecision {
        dir::AssignmentDecision {
            target: self.source,
            read: self.read,
            write: self.write,
        }
    }
}

/// Write validation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum WriteMode {
    /// The storage owner decides whether the write is legal.
    Direct,
    /// The declaring constructor initializes one of its own fields.
    Initialize {
        /// The declaration being initialized.
        owner: dir::GlobalSymbolId,
    },
    /// The write crosses potentially shared indirection.
    Indirect {
        /// The value whose access determines whether the write is exclusive.
        receiver: dir::GlobalTypeId,
    },
}

impl CheckState<'_> {
    /// Check one writable assignment target requirement.
    pub(in crate::sema) fn check_writable_assignment(
        &mut self,
        origin: Origin,
        obligation: &WritableTargetObligation,
    ) -> CompilerResult<ObligationCheck> {
        // require a writable target before weighing the indirection it crosses
        let target = &obligation.target;
        let check =
            self.check_writable_target(origin, target.source, &target.write, target.mode)?;
        if !matches!(check, ObligationCheck::Holds) {
            return Ok(check);
        }

        // validate writes that cross potentially shared indirection
        let check = match target.mode {
            WriteMode::Direct | WriteMode::Initialize { .. } => ObligationCheck::holds(),
            WriteMode::Indirect { receiver } => {
                self.check_indirect_write(origin, target.source, receiver, obligation.ty)?
            }
        };

        // commit direct mutation below a binding, rebinding counts as its own write
        let mutates_direct_value = target.mode == WriteMode::Direct
            && !matches!(target.write, dir::WriteResolution::Binding { .. });
        if matches!(check, ObligationCheck::Holds) && mutates_direct_value {
            self.commit_access_use(target.source, dir::BindingUse::MUTATE);
        }

        Ok(check)
    }

    /// Check one write target tree for writable leaves.
    fn check_writable_target(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        target: &dir::WriteResolution,
        mode: WriteMode,
    ) -> CompilerResult<ObligationCheck> {
        // check by the write target's own kind
        match target {
            dir::WriteResolution::Binding { symbol, .. } => {
                self.check_writable_binding(source, *symbol)
            }
            dir::WriteResolution::Member(member) => {
                self.check_writable_member(origin, source, member, mode)
            }
            dir::WriteResolution::Subscript(subscript) => {
                self.check_writable_subscript(origin, source, subscript, mode)
            }
            // write a dereferenced place through its own borrow
            dir::WriteResolution::Dereference(_) => Ok(ObligationCheck::holds()),
        }
    }

    /// Check one writable member resolution.
    fn check_writable_member(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        member: &dir::MemberDecision,
        mode: WriteMode,
    ) -> CompilerResult<ObligationCheck> {
        // require every access along the resolved chain to be writable
        for access in member.arms() {
            let check = self.check_writable_member_access(origin, source, access, mode)?;
            match check {
                ObligationCheck::Holds => {}
                ObligationCheck::Fails(_) | ObligationCheck::Ambiguous(_) => {
                    return Ok(check);
                }
            }
        }

        Ok(ObligationCheck::holds())
    }

    /// Check one singular writable member access.
    fn check_writable_member_access(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        access: &dir::MemberAccess,
        mode: WriteMode,
    ) -> CompilerResult<ObligationCheck> {
        self.check_writable_member_target(origin, source, access.receiver, &access.target, mode)
    }

    /// Check one writable member target.
    fn check_writable_member_target(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        target: &dir::MemberTarget,
        mode: WriteMode,
    ) -> CompilerResult<ObligationCheck> {
        // check by the member target's own kind
        match target {
            // require a writable projection and a writable slot for a field write
            dir::MemberTarget::Field(field) => {
                if self.is_readonly_receiver_projection(receiver)? {
                    let failure = ObligationFailure::CannotAssignReadonlyMember {
                        source,
                        member: target.clone(),
                    };

                    return Ok(ObligationCheck::fail(failure));
                }

                self.check_writable_field(source, field, mode)
            }
            // write an intersected target through every arm
            dir::MemberTarget::Intersection(targets) => {
                for target in targets {
                    let check =
                        self.check_writable_member_target(origin, source, receiver, target, mode)?;
                    match check {
                        ObligationCheck::Holds => {}
                        ObligationCheck::Fails(_) | ObligationCheck::Ambiguous(_) => {
                            return Ok(check);
                        }
                    }
                }

                Ok(ObligationCheck::holds())
            }
            // leave a setter call to its own write rules
            dir::MemberTarget::Call(_) => Ok(ObligationCheck::holds()),
            // require a writable projection and writable fields for an index write
            dir::MemberTarget::Index(index) => {
                if self.is_readonly_receiver_projection(receiver)? {
                    let failure = ObligationFailure::CannotAssignReadonlyMember {
                        source,
                        member: target.clone(),
                    };

                    return Ok(ObligationCheck::fail(failure));
                }

                self.check_writable_index(origin, source, index)
            }
            dir::MemberTarget::Projection { .. }
            | dir::MemberTarget::Symbol(_)
            | dir::MemberTarget::OverloadSet(_) => Err(CompilerError::Internal {
                message: format!("write selected non-writable member target {target:?}"),
            }),
        }
    }

    /// Check one writable subscript resolution.
    fn check_writable_subscript(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        subscript: &dir::SubscriptDecision,
        mode: WriteMode,
    ) -> CompilerResult<ObligationCheck> {
        // require every access along the resolved chain to be writable
        for subscript in subscript.arms() {
            let check = match &subscript.target {
                dir::SubscriptTarget::Member(member) => {
                    self.check_writable_member_access(origin, source, member, mode)?
                }
                dir::SubscriptTarget::Call(_) => ObligationCheck::holds(),
                dir::SubscriptTarget::Index(_) => ObligationCheck::holds(),
            };
            match check {
                ObligationCheck::Holds => {}
                ObligationCheck::Fails(_) | ObligationCheck::Ambiguous(_) => {
                    return Ok(check);
                }
            }
        }

        Ok(ObligationCheck::holds())
    }

    /// Check one write across potentially shared indirection.
    fn check_indirect_write(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<ObligationCheck> {
        // accept an exclusive receiver, only the writer sees the overwrite
        if self.is_exclusive_receiver(receiver)? {
            return Ok(ObligationCheck::holds());
        }

        // require a value that overwrites atomically for a shared receiver
        let ty = self.normalize(origin, ty)?;
        match self.decide_auto_interface(origin, ty, dir::AutoInterface::OverwriteStable)? {
            Verdict::Holds => return Ok(ObligationCheck::holds()),
            // stall the obligation while an open variable leaves the rule undecided
            Verdict::Ambiguous => {
                return Ok(ObligationCheck::Ambiguous(
                    self.collect_open_variables([ty])?,
                ));
            }
            Verdict::Fails => {}
        }

        // solve an open receiver place as local, which grants exclusivity
        if let Some(place) = self.form_chain(origin, receiver)?.place() {
            let place = self.shallow_resolve(place)?;
            if matches!(self.ty(place)?, dir::Type::Variable(_)) {
                let local = self.local_place()?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                self.constrain_type(origin, cause, Relation::Equal, place, local)?;

                return Ok(ObligationCheck::holds());
            }
        }
        let failure = ObligationFailure::OverwriteStabilityNotSatisfied { source, ty };

        Ok(ObligationCheck::fail(failure))
    }

    /// Return whether one receiver grants exclusive access.
    fn is_exclusive_receiver(&mut self, receiver: dir::GlobalTypeId) -> CompilerResult<bool> {
        // read the access a borrowed receiver carries
        let access = match self.ty(receiver)? {
            dir::Type::Variable(variable) => {
                return Err(CompilerError::Internal {
                    message: format!("open variable {variable:?} reached a write obligation"),
                });
            }
            dir::Type::Form(form) => match form.form {
                dir::Form::Borrowed(borrow) => {
                    Some(self.type_borrow(receiver.module_id, borrow)?.access)
                }
                dir::Form::Managed { .. }
                | dir::Form::Owned
                | dir::Form::Readonly
                | dir::Form::Raw => None,
            },
            _ => None,
        };
        let Some(access) = access else {
            return Ok(false);
        };

        // report whether the borrow grants exclusive access
        let is_exclusive = self.access_of(access)? == Some(dir::Access::Exclusive);

        Ok(is_exclusive)
    }

    /// Check one binding write.
    fn check_writable_binding(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<ObligationCheck> {
        // reject writes to a binding another module declares
        if symbol.module_id != source.module_id {
            let failure = ObligationFailure::CannotAssignImportedBinding { source, symbol };

            return Ok(ObligationCheck::fail(failure));
        }

        // read the binding the write names
        let input = self.module(symbol.module_id);
        let bindings = input.binding_table();
        let local_symbol = bindings.get_symbol(symbol.local_id);

        // reject writes through imported aliases
        if input
            .resolved
            .imports
            .symbol_resolution(symbol.local_id)
            .is_some()
        {
            let failure = ObligationFailure::CannotAssignImportedBinding { source, symbol };

            return Ok(ObligationCheck::fail(failure));
        }

        // reject writes to immutable bindings
        let is_mutable = local_symbol.binding_mutability.is_some_and(|mutability| {
            matches!(
                mutability,
                dir::Mutability::Mutable | dir::Mutability::Exclusive
            )
        });
        if !is_mutable {
            let failure = ObligationFailure::CannotAssignImmutableBinding { source, symbol };

            return Ok(ObligationCheck::fail(failure));
        }

        Ok(ObligationCheck::holds())
    }

    /// Check one field write.
    fn check_writable_field(
        &mut self,
        source: dir::GlobalNodeIdAny,
        field: &dir::FieldResolution,
        mode: WriteMode,
    ) -> CompilerResult<ObligationCheck> {
        // check by the field target's own kind
        match field.target {
            dir::FieldTarget::Structural { owner, key } => {
                self.check_writable_structural_field(source, field, owner, key)
            }
            dir::FieldTarget::Member { symbol, .. } => {
                self.check_writable_nominal_field(source, field, symbol, mode)
            }
        }
    }

    /// Check one structural field write.
    fn check_writable_structural_field(
        &mut self,
        source: dir::GlobalNodeIdAny,
        field: &dir::FieldResolution,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<ObligationCheck> {
        // read the mutability the owning aggregate declares for the key
        let is_readonly = match self.ty(owner)? {
            dir::Type::Object(shape) => self
                .object_properties(owner.module_id, shape.properties)?
                .iter()
                .find(|property| property.key == key)
                .map(|property| !property.access.is_writable()),
            dir::Type::Tuple(tuple) => match key {
                dir::StaticKey::Index(index) => self
                    .tuple_elements(owner.module_id, tuple.elements)?
                    .get(index)
                    .map(|element| element.is_readonly),
                _ => None,
            },
            _ => {
                let owner_type = self.format_type(owner);

                return Err(CompilerError::Internal {
                    message: format!(
                        "structural field {key:?} has non-aggregate owner {owner_type} ({owner:?})"
                    ),
                });
            }
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("selected structural field {key:?} is absent from {owner:?}"),
        })?;

        // reject a write to a readonly slot
        if is_readonly {
            Ok(ObligationCheck::fail(
                ObligationFailure::CannotAssignReadonlyMember {
                    source,
                    member: dir::MemberTarget::Field(field.clone()),
                },
            ))
        }
        // otherwise the aggregate owns a writable slot
        else {
            Ok(ObligationCheck::holds())
        }
    }

    /// Check one field write backed by a declaration.
    fn check_writable_nominal_field(
        &mut self,
        source: dir::GlobalNodeIdAny,
        field: &dir::FieldResolution,
        symbol: dir::GlobalSymbolId,
        mode: WriteMode,
    ) -> CompilerResult<ObligationCheck> {
        // read the declaration that owns the written field
        let bindings = self.binding_table(symbol.module_id);
        let owner = bindings
            .symbol_path(symbol.local_id)
            .owner()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("nominal field {symbol:?} has no declaration owner"),
            })?;
        let owner = dir::GlobalSymbolId {
            module_id: symbol.module_id,
            local_id: owner,
        };
        let definition = self
            .definition(owner)?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("nominal field {symbol:?} owner {owner:?} has no definition"),
            })?;
        let declared = definition
            .field(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("definition {owner:?} does not declare field {symbol:?}"),
            })?;

        // reject a write to a readonly field outside its own initializer
        let is_initialization =
            matches!(mode, WriteMode::Initialize { owner: initialized } if initialized == owner);
        if declared.is_readonly && !is_initialization {
            Ok(ObligationCheck::fail(
                ObligationFailure::CannotAssignReadonlyMember {
                    source,
                    member: dir::MemberTarget::Field(field.clone()),
                },
            ))
        }
        // otherwise the declaration owns a writable field
        else {
            Ok(ObligationCheck::holds())
        }
    }

    /// Check one structural index write.
    fn check_writable_index(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        index: &dir::IndexResolution,
    ) -> CompilerResult<ObligationCheck> {
        // write a structural index into the fields of its own receiver
        let receiver = self.normalize(origin, index.receiver.ty())?;
        let dir::Type::Object(shape) = self.ty(receiver)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "structural index {:?} has non-object receiver {receiver:?}",
                    index.key_type
                ),
            });
        };

        // reject keyed writes through structural signatures
        let dir::IndexTarget::Fields(keys) = &index.target else {
            let failure = ObligationFailure::CannotAssignStructuralIndex { source, receiver };

            return Ok(ObligationCheck::fail(failure));
        };
        if keys.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("structural index reaches no field in {receiver:?}"),
            });
        }

        // require every field the key domain reaches to be writable
        let properties = self.object_properties(receiver.module_id, shape.properties)?;
        let mut is_readonly = false;
        for key in keys {
            let property = properties
                .iter()
                .find(|property| property.key == *key)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "selected structural field {key:?} is absent from {receiver:?}"
                    ),
                })?;
            is_readonly |= !property.access.is_writable();
        }

        // reject the write when any reached field is readonly
        if is_readonly {
            let failure = ObligationFailure::CannotAssignReadonlyMember {
                source,
                member: dir::MemberTarget::Index(index.clone()),
            };

            return Ok(ObligationCheck::fail(failure));
        }

        Ok(ObligationCheck::holds())
    }
}
