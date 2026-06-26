use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, AutoInterface, CheckError, CheckState, Dependency, MemberLookup, Origin,
    WritablePlaceObligation, answer,
};

/// Writable storage selected by a source expression.
///
/// Examples:
/// ```ds
/// value = next
/// object.field = next
/// values[index] = next
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct Place {
    /// How the expression selected the place.
    pub(in crate::check) target: PlaceTarget,
    /// How writes through this place are justified.
    pub(in crate::check) write: PlaceWrite,
    /// The source node for diagnostics.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

impl Place {
    /// Create a place.
    pub(in crate::check) fn new(target: PlaceTarget, source: dir::GlobalNodeIdAny) -> Self {
        Self {
            target,
            write: PlaceWrite::Direct,
            source,
        }
    }

    /// Create a place that writes through non-exclusive indirection.
    pub(in crate::check) fn stable_overwrite(
        target: PlaceTarget,
        source: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
    ) -> Self {
        Self {
            target,
            write: PlaceWrite::StableOverwrite { receiver },
            source,
        }
    }
}

/// Proof needed to write through one place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum PlaceWrite {
    /// The storage owner decides whether the write is legal.
    Direct,
    /// The write goes through non-exclusive indirection.
    StableOverwrite {
        /// The value whose access determines whether the write is exclusive.
        receiver: dir::GlobalTypeId,
    },
}

/// How an expression selects a place.
///
/// Examples:
/// ```ds
/// value
/// object.field
/// values[index]
/// *pointer
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum PlaceTarget {
    /// Local or imported value binding.
    Binding {
        /// The selected local binding symbol.
        symbol: dir::GlobalSymbolId,
    },
    /// Structural or nominal member target.
    Member {
        /// The receiver type.
        owner: dir::GlobalTypeId,
        /// The selected member key.
        key: dir::StaticKey,
    },
    /// Protocol-backed index target.
    Index {
        /// The indexed receiver type.
        receiver: dir::GlobalTypeId,
        /// The index expression type.
        index: dir::GlobalTypeId,
    },
    /// Protocol-backed dereference target.
    Dereference,
}

impl CheckState<'_> {
    /// Check one writable place requirement.
    pub(in crate::check) fn check_writable_place(
        &mut self,
        obligation: &WritablePlaceObligation,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let place = obligation.place;
        let diagnostic = match place.target {
            PlaceTarget::Binding { symbol } => self.writable_binding_error(place.source, symbol)?,
            PlaceTarget::Member { owner, key } => {
                self.writable_member_error(place.source, owner, key)?
            }
            PlaceTarget::Index { .. } | PlaceTarget::Dereference => Answer::Ready(None),
        };
        match diagnostic {
            Answer::Ready(Some(_)) | Answer::Pending(_) => Ok(diagnostic),
            Answer::Ready(None) => match place.write {
                PlaceWrite::Direct => Ok(Answer::Ready(None)),
                PlaceWrite::StableOverwrite { receiver } => {
                    self.stable_overwrite_error(place.source, receiver, obligation.ty)
                }
            },
        }
    }

    /// Return the diagnostic for one non-exclusive overwrite.
    fn stable_overwrite_error(
        &mut self,
        source: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        if answer!(self.is_exclusive_receiver(Origin::Node(source), receiver)?) {
            return Ok(Answer::Ready(None));
        }

        let origin = Origin::Node(source);
        let ty = answer!(self.reduce_type_root(origin, ty)?);
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
        let receiver = answer!(self.reduce_type_root(origin, receiver)?);
        let access = match self.ty(receiver)? {
            dir::Type::Variable(variable) => {
                let representative = self.solver.representative(*variable)?;

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
        let access = answer!(self.reduce_type_root(origin, access)?);
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
        let name = self.format_symbol(symbol);

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

    /// Return the diagnostic for one member that rejects writes.
    fn writable_member_error(
        &mut self,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);
        let owner = answer!(self.reduce_type_root(origin, owner)?);

        // structural fields carry their write access directly
        if let dir::Type::Shape(shape) = self.ty(owner)? {
            let field = shape.fields.iter().find(|field| field.key == key);

            if let Some(field) = field {
                if field.is_readonly {
                    let (module, anchor) = self.source_anchor(source);
                    let member = self.format_static_key(&key);
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

        // declaration members default to writable until access modeling
        let lookup = self.lookup_member(
            origin,
            source.module_id,
            owner,
            dir::MemberSpace::Instance,
            key,
        )?;
        match lookup {
            MemberLookup::Pending(blockers) => Ok(Answer::Pending(blockers)),
            _ => Ok(Answer::Ready(None)),
        }
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
