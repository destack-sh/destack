use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, ObligationCheck, ObligationFailure, Origin, WritablePlaceObligation, answer,
};

/// Writable storage selected by a source expression.
///
/// Examples:
/// ```ds
/// value = next
/// object.field = next
/// values[index] = next
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) struct WriteTarget {
    /// The selected storage.
    pub(in crate::check) storage: dir::Storage,
    /// The type written through the storage.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The write validation mode.
    pub(in crate::check) mode: WriteMode,
    /// The source node for diagnostics.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

impl WriteTarget {
    /// Create a directly writable target.
    pub(in crate::check) fn new(
        storage: dir::Storage,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
    ) -> Self {
        Self {
            storage,
            ty,
            mode: WriteMode::Direct,
            source,
        }
    }

    /// Create a target that writes through non-exclusive indirection.
    pub(in crate::check) fn stable_overwrite(
        storage: dir::Storage,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
    ) -> Self {
        Self {
            storage,
            ty,
            mode: WriteMode::StableOverwrite { receiver },
            source,
        }
    }

    /// Return the durable place resolution selected by this target.
    pub(in crate::check) fn resolution(self) -> dir::PlaceResolution {
        dir::PlaceResolution {
            source: self.source,
            storage: self.storage,
            ty: self.ty,
        }
    }
}

/// Write validation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum WriteMode {
    /// The storage owner decides whether the write is legal.
    Direct,
    /// The write goes through non-exclusive indirection.
    StableOverwrite {
        /// The value whose access determines whether the write is exclusive.
        receiver: dir::GlobalTypeId,
    },
}

impl CheckState<'_> {
    /// Check one writable place requirement.
    pub(in crate::check) fn check_writable_place(
        &mut self,
        origin: Origin,
        obligation: &WritablePlaceObligation,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let target = &obligation.place;
        let check = match &target.storage {
            dir::Storage::Binding { symbol } => {
                self.check_writable_binding(target.source, *symbol)?
            }
            dir::Storage::Field { receiver, field } => {
                self.check_writable_field(origin, target.source, *receiver, *field)?
            }
            dir::Storage::Property { .. }
            | dir::Storage::Subscript { .. }
            | dir::Storage::Dereference { .. } => Answer::Ready(ObligationCheck::holds()),
        };
        match check {
            Answer::Ready(ObligationCheck::Fails(_)) | Answer::Pending(_) => Ok(check),
            Answer::Ready(ObligationCheck::Holds) => match target.mode {
                WriteMode::Direct => Ok(Answer::Ready(ObligationCheck::holds())),
                WriteMode::StableOverwrite { receiver } => {
                    self.check_stable_overwrite(origin, target.source, receiver, obligation.ty)
                }
            },
        }
    }

    /// Check one non-exclusive overwrite.
    fn check_stable_overwrite(
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

    /// Return whether one receiver carries exclusive access.
    fn is_exclusive_receiver(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);
        let access = match self.ty(receiver)? {
            dir::Type::Variable(variable) => {
                return Ok(Answer::pending([self.variable_dependency(variable)?]));
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
        // cross module symbols are imported into this module
        if symbol.module_id != source.module_id {
            let failure = ObligationFailure::CannotAssignImportedBinding { source, symbol };

            return Ok(Answer::Ready(ObligationCheck::fail(failure)));
        }

        let input = self.module(symbol.module_id);
        let bindings = input.binding_table();
        let local_symbol = bindings.get_symbol(symbol.local_id);

        // imported aliases never accept writes
        if input
            .resolved
            .imports
            .symbol_target(symbol.local_id)
            .is_some()
        {
            let failure = ObligationFailure::CannotAssignImportedBinding { source, symbol };

            return Ok(Answer::Ready(ObligationCheck::fail(failure)));
        }

        // immutable bindings reject writes
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
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalTypeId,
        field: dir::ProjectionField,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let owner = answer!(self.reduce_type_head(origin, owner)?);

        // readonly receiver views reject stored field writes
        if answer!(
            self.body(origin.module())
                .receiver_projects_readonly(origin, owner)?
        ) {
            let failure = ObligationFailure::CannotAssignReadonlyMember {
                source,
                member: field,
            };

            return Ok(Answer::Ready(ObligationCheck::fail(failure)));
        }

        // structural fields carry their write access directly
        if let dir::ProjectionField::Key(key) = field
            && let dir::Type::Shape(shape) = self.ty(owner)?
        {
            let field = self
                .shape_fields(owner.module_id, shape.fields)?
                .iter()
                .find(|field| field.key == key)
                .copied();

            if let Some(field) = field {
                if field.is_readonly {
                    let failure = ObligationFailure::CannotAssignReadonlyMember {
                        source,
                        member: dir::ProjectionField::Key(key),
                    };

                    return Ok(Answer::Ready(ObligationCheck::fail(failure)));
                }

                return Ok(Answer::Ready(ObligationCheck::holds()));
            }

            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        Ok(Answer::Ready(ObligationCheck::holds()))
    }
}
