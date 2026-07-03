use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, AutoInterface, CheckError, CheckState, Dependency, Origin, WritablePlaceObligation,
    answer,
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
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let target = &obligation.place;
        let diagnostic = match &target.storage {
            dir::Storage::Binding { symbol } => {
                self.writable_binding_error(target.source, *symbol)?
            }
            dir::Storage::Field { receiver, field } => match field {
                dir::ProjectionField::Key(_) => {
                    self.writable_field_error(origin, target.source, *receiver, *field)?
                }
                dir::ProjectionField::Member(_) => {
                    self.writable_field_error(origin, target.source, *receiver, *field)?
                }
            },
            dir::Storage::Property { .. }
            | dir::Storage::Subscript { .. }
            | dir::Storage::Dereference { .. } => Answer::Ready(None),
        };
        match diagnostic {
            Answer::Ready(Some(_)) | Answer::Pending(_) => Ok(diagnostic),
            Answer::Ready(None) => match target.mode {
                WriteMode::Direct => Ok(Answer::Ready(None)),
                WriteMode::StableOverwrite { receiver } => {
                    self.stable_overwrite_error(origin, target.source, receiver, obligation.ty)
                }
            },
        }
    }

    /// Return the diagnostic for one non-exclusive overwrite.
    fn stable_overwrite_error(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        if answer!(self.is_exclusive_receiver(origin, receiver)?) {
            return Ok(Answer::Ready(None));
        }

        let ty = answer!(self.reduce_type_head(origin, ty)?);
        if answer!(self.satisfies_auto_interface(origin, ty, AutoInterface::OverwriteStable)?) {
            return Ok(Answer::Ready(None));
        }

        let (module, anchor) = self.source_anchor(source);
        let ty = self.format_type(ty);
        let error = CheckError::OverwriteStabilityNotSatisfied { anchor, module, ty };

        Ok(Answer::Ready(Some(error.into())))
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
                let representative = self.solver.representative(variable)?;

                return Ok(Answer::pending([Dependency::Variable(representative)]));
            }
            dir::Type::Form(form) => match form.form {
                dir::Form::Borrowed { access, .. } => Some(access),
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

    /// Return the diagnostic for one binding that rejects writes.
    fn writable_binding_error(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let (module, anchor) = self.source_anchor(source);
        let name = self.format_assignment_binding(source, symbol);

        // cross module symbols are imported into this module
        if symbol.module_id != source.module_id {
            let error = CheckError::CannotAssignImportedBinding {
                anchor,
                module,
                name,
            };
            let diagnostic = DiagnosticBuilder::new(error);

            return Ok(Answer::Ready(Some(diagnostic)));
        }

        let input = self.module(symbol.module_id);
        let bindings = input.binding_table();
        let local_symbol = bindings.get_symbol(symbol.local_id);

        // parameters are writable local bindings
        if local_symbol
            .declaration
            .is_some_and(|declaration| declaration.local_id.ty == dir::NodeType::Parameter)
        {
            return Ok(Answer::Ready(None));
        }

        // imported aliases never accept writes
        if input
            .resolved
            .imports
            .symbol_target(symbol.local_id)
            .is_some()
        {
            let error = CheckError::CannotAssignImportedBinding {
                anchor,
                module,
                name,
            };
            let diagnostic = self.label_binding_declaration(DiagnosticBuilder::new(error), symbol);

            return Ok(Answer::Ready(Some(diagnostic)));
        }

        // immutable bindings reject writes
        let is_mutable = local_symbol.binding_mutability.is_some_and(|mutability| {
            matches!(
                mutability,
                dir::Mutability::Mutable | dir::Mutability::Exclusive
            )
        });
        if !is_mutable {
            let error = CheckError::CannotAssignImmutableBinding {
                anchor,
                module,
                name,
            };
            let diagnostic = self.label_binding_declaration(DiagnosticBuilder::new(error), symbol);

            return Ok(Answer::Ready(Some(diagnostic)));
        }

        Ok(Answer::Ready(None))
    }

    /// Return the diagnostic for one field that rejects writes.
    fn writable_field_error(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalTypeId,
        field: dir::ProjectionField,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let owner = answer!(self.reduce_type_head(origin, owner)?);
        let member = match field {
            dir::ProjectionField::Key(key) => self.format_static_key(&key),
            dir::ProjectionField::Member(symbol) => self.format_symbol(symbol),
        };

        // readonly receiver views reject stored field writes
        if answer!(self.receiver_projects_readonly(origin, owner)?) {
            let (module, anchor) = self.source_anchor(source);
            let error = CheckError::CannotAssignReadonlyMember {
                anchor,
                module,
                member,
            };
            let diagnostic = DiagnosticBuilder::new(error);

            return Ok(Answer::Ready(Some(diagnostic)));
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
                    let (module, anchor) = self.source_anchor(source);
                    let error = CheckError::CannotAssignReadonlyMember {
                        anchor,
                        module,
                        member,
                    };
                    let diagnostic = DiagnosticBuilder::new(error);

                    return Ok(Answer::Ready(Some(diagnostic)));
                }

                return Ok(Answer::Ready(None));
            }

            return Ok(Answer::Ready(None));
        }

        Ok(Answer::Ready(None))
    }

    /// Add a declaration label to a binding diagnostic when the declaration is local.
    fn label_binding_declaration(
        &self,
        diagnostic: DiagnosticBuilder<CheckError>,
        symbol: dir::GlobalSymbolId,
    ) -> DiagnosticBuilder<CheckError> {
        let declaration = self
            .binding_table(symbol.module_id)
            .get_symbol_maybe(symbol.local_id)
            .and_then(|binding| binding.declaration)
            .filter(|declaration| declaration.module_id == symbol.module_id);

        if let Some(declaration) = declaration {
            let anchor = self.diagnostic_anchor(symbol.module_id, declaration.local_id);

            diagnostic.label(anchor, "declared here")
        } else {
            diagnostic
        }
    }
}
