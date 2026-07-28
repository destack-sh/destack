use destack_dir as dir;

use crate::check::{
    Answer, CheckState, Dependency, ObligationCheck, ObligationFailure, Origin,
    WritableTargetObligation, answer,
};
use crate::{CompilerError, CompilerResult};

/// Assignment target selected by a source expression.
///
/// Examples:
/// ```ds
/// value = next
/// object.field = next
/// values[index] = next
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) struct AssignmentSelection {
    /// The selected read, when the source reads before writing.
    pub(in crate::check) read: Option<dir::ReadResolution>,
    /// The selected write.
    pub(in crate::check) write: dir::WriteResolution,
    /// The write validation mode.
    pub(in crate::check) mode: WriteMode,
    /// The source node for diagnostics.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

impl AssignmentSelection {
    /// Return the durable assignment resolution selected by this target.
    pub(in crate::check) fn resolution(self) -> dir::AssignmentResolution {
        dir::AssignmentResolution {
            target: self.source,
            read: self.read,
            write: self.write,
        }
    }
}

/// Write validation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum WriteMode {
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
    pub(in crate::check) fn check_writable_assignment(
        &mut self,
        origin: Origin,
        obligation: &WritableTargetObligation,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let target = &obligation.target;
        let check =
            self.check_writable_target(origin, target.source, &target.write, target.mode)?;
        match check {
            Answer::Ready(ObligationCheck::Fails(_)) | Answer::Pending(_) => Ok(check),
            Answer::Ready(ObligationCheck::Holds) => match target.mode {
                WriteMode::Direct | WriteMode::Initialize { .. } => {
                    Ok(Answer::Ready(ObligationCheck::holds()))
                }
                WriteMode::Indirect { receiver } => {
                    self.check_indirect_write(origin, target.source, receiver, obligation.ty)
                }
            },
        }
    }

    /// Check one write target tree for writable leaves.
    fn check_writable_target(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        target: &dir::WriteResolution,
        mode: WriteMode,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let check = match target {
            dir::WriteResolution::Binding { symbol, .. } => {
                self.check_writable_binding(source, *symbol)?
            }
            dir::WriteResolution::Member(member) => {
                self.check_writable_member(origin, source, member, mode)?
            }
            dir::WriteResolution::Subscript(subscript) => {
                self.check_writable_subscript(origin, source, subscript, mode)?
            }
            dir::WriteResolution::Dereference(_) => Answer::Ready(ObligationCheck::holds()),
        };

        Ok(check)
    }

    /// Check one writable member resolution.
    fn check_writable_member(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        member: &dir::MemberResolution,
        mode: WriteMode,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        for access in member.iter() {
            let check = self.check_writable_member_access(origin, source, access, mode)?;
            match check {
                Answer::Ready(ObligationCheck::Holds) => {}
                Answer::Ready(ObligationCheck::Fails(_)) | Answer::Pending(_) => {
                    return Ok(check);
                }
            }
        }

        Ok(Answer::Ready(ObligationCheck::holds()))
    }

    /// Check one singular writable member access.
    fn check_writable_member_access(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        access: &dir::MemberAccess,
        mode: WriteMode,
    ) -> CompilerResult<Answer<ObligationCheck>> {
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
    ) -> CompilerResult<Answer<ObligationCheck>> {
        match target {
            dir::MemberTarget::Field(field) => {
                if answer!(self.body().receiver_projects_readonly(origin, receiver)?) {
                    let failure = ObligationFailure::CannotAssignReadonlyMember {
                        source,
                        member: target.clone(),
                    };

                    return Ok(Answer::Ready(ObligationCheck::fail(failure)));
                }

                self.check_writable_field(source, field, mode)
            }
            dir::MemberTarget::Intersection(targets) => {
                for target in targets {
                    let check =
                        self.check_writable_member_target(origin, source, receiver, target, mode)?;
                    match check {
                        Answer::Ready(ObligationCheck::Holds) => {}
                        Answer::Ready(ObligationCheck::Fails(_)) | Answer::Pending(_) => {
                            return Ok(check);
                        }
                    }
                }

                Ok(Answer::Ready(ObligationCheck::holds()))
            }
            dir::MemberTarget::Call(_) => Ok(Answer::Ready(ObligationCheck::holds())),
            dir::MemberTarget::Index(index) => {
                if answer!(self.body().receiver_projects_readonly(origin, receiver)?) {
                    let failure = ObligationFailure::CannotAssignReadonlyMember {
                        source,
                        member: target.clone(),
                    };

                    return Ok(Answer::Ready(ObligationCheck::fail(failure)));
                }

                self.check_writable_index(origin, source, index)
            }
            dir::MemberTarget::Projection { .. }
            | dir::MemberTarget::Symbol(_)
            | dir::MemberTarget::Existential(_) => Err(CompilerError::Internal {
                message: format!("write selected non-writable member target {target:?}"),
            }),
        }
    }

    /// Check one writable subscript resolution.
    fn check_writable_subscript(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        subscript: &dir::SubscriptResolution,
        mode: WriteMode,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        for subscript in subscript.iter() {
            let check = match &subscript.target {
                dir::SubscriptTarget::Member(member) => {
                    self.check_writable_member_access(origin, source, member, mode)?
                }
                dir::SubscriptTarget::Call(_) => Answer::Ready(ObligationCheck::holds()),
                dir::SubscriptTarget::Index(_) => Answer::Ready(ObligationCheck::holds()),
            };
            match check {
                Answer::Ready(ObligationCheck::Holds) => {}
                Answer::Ready(ObligationCheck::Fails(_)) | Answer::Pending(_) => {
                    return Ok(check);
                }
            }
        }

        Ok(Answer::Ready(ObligationCheck::holds()))
    }

    /// Check one write across potentially shared indirection.
    fn check_indirect_write(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        if answer!(self.is_exclusive_receiver(origin, receiver)?) {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        let ty = answer!(self.reduce_type_head(origin, ty)?);
        if answer!(self.satisfies_auto_interface(
            origin,
            ty,
            dir::AutoInterface::OverwriteStable,
        )?) {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        let failure = ObligationFailure::OverwriteStabilityNotSatisfied { source, ty };

        Ok(Answer::Ready(ObligationCheck::fail(failure)))
    }

    /// Return whether one receiver grants exclusive access.
    fn is_exclusive_receiver(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);
        let access = match self.ty(receiver)? {
            dir::Type::Variable(variable) => {
                return Ok(Answer::pending([Dependency::Variable(variable)]));
            }
            dir::Type::Form(form) => match form.form {
                dir::Form::Borrowed(borrow) => {
                    Some(self.type_borrow(receiver.module_id, borrow)?.access)
                }
                _ => None,
            },
            _ => None,
        };
        let Some(access) = access else {
            return Ok(Answer::Ready(false));
        };
        let access = answer!(self.reduce_type_head(origin, access)?);
        let is_exclusive = matches!(
            self.ty(access)?,
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Exclusive))
        );

        Ok(Answer::Ready(is_exclusive))
    }

    /// Check one binding write.
    fn check_writable_binding(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        // treat a cross module symbol as imported into this module
        if symbol.module_id != source.module_id {
            let failure = ObligationFailure::CannotAssignImportedBinding { source, symbol };

            return Ok(Answer::Ready(ObligationCheck::fail(failure)));
        }

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

            return Ok(Answer::Ready(ObligationCheck::fail(failure)));
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

            return Ok(Answer::Ready(ObligationCheck::fail(failure)));
        }

        Ok(Answer::Ready(ObligationCheck::holds()))
    }

    /// Check one field write.
    fn check_writable_field(
        &mut self,
        source: dir::GlobalNodeIdAny,
        field: &dir::FieldResolution,
        mode: WriteMode,
    ) -> CompilerResult<Answer<ObligationCheck>> {
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
        resolution: &dir::FieldResolution,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let is_readonly = match self.ty(owner)? {
            dir::Type::Shape(shape) => self
                .shape_properties(owner.module_id, shape.properties)?
                .iter()
                .find(|field| field.key == key)
                .map(|field| !field.access.is_writable()),
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

        if is_readonly {
            let failure = ObligationFailure::CannotAssignReadonlyMember {
                source,
                member: dir::MemberTarget::Field(resolution.clone()),
            };

            Ok(Answer::Ready(ObligationCheck::fail(failure)))
        } else {
            Ok(Answer::Ready(ObligationCheck::holds()))
        }
    }

    /// Check one declaration-backed field write.
    fn check_writable_nominal_field(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::FieldResolution,
        symbol: dir::GlobalSymbolId,
        mode: WriteMode,
    ) -> CompilerResult<Answer<ObligationCheck>> {
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
        let field = definition
            .field(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("definition {owner:?} does not declare field {symbol:?}"),
            })?;

        let is_initialization =
            matches!(mode, WriteMode::Initialize { owner: initialized } if initialized == owner);
        if field.is_readonly && !is_initialization {
            let failure = ObligationFailure::CannotAssignReadonlyMember {
                source,
                member: dir::MemberTarget::Field(resolution.clone()),
            };

            Ok(Answer::Ready(ObligationCheck::fail(failure)))
        } else {
            Ok(Answer::Ready(ObligationCheck::holds()))
        }
    }

    /// Check one structural index write.
    fn check_writable_index(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        index: &dir::IndexResolution,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let receiver = answer!(self.reduce_type_head(origin, index.receiver.ty())?);
        let dir::Type::Shape(shape) = self.ty(receiver)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "structural index {:?} has non-shape receiver {receiver:?}",
                    index.key_type
                ),
            });
        };

        let is_readonly = match &index.target {
            dir::IndexTarget::Signature(position) => self
                .shape_index_signatures(receiver.module_id, shape.index_signatures)?
                .get(*position)
                .map(|signature| signature.is_readonly)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "selected index signature {position} is absent from {receiver:?}"
                    ),
                })?,
            dir::IndexTarget::Fields(keys) => {
                let fields = self.shape_properties(receiver.module_id, shape.properties)?;
                let mut is_readonly = false;
                for key in keys {
                    let field = fields
                        .iter()
                        .find(|field| field.key == *key)
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "selected structural field {key:?} is absent from {receiver:?}"
                            ),
                        })?;
                    is_readonly |= !field.access.is_writable();
                }
                if keys.is_empty() {
                    return Err(CompilerError::Internal {
                        message: format!("structural index reaches no field in {receiver:?}"),
                    });
                }

                is_readonly
            }
        };

        if is_readonly {
            let failure = ObligationFailure::CannotAssignReadonlyMember {
                source,
                member: dir::MemberTarget::Index(index.clone()),
            };

            return Ok(Answer::Ready(ObligationCheck::fail(failure)));
        }

        Ok(Answer::Ready(ObligationCheck::holds()))
    }
}
