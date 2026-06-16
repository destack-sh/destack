use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckError, CheckState, MemberLookup, Origin};

/// Writable storage selected by source syntax.
///
/// Examples:
/// ```ds
/// value = next
/// object.field = next
/// values[index] = next
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct Place {
    /// The selected storage type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// How source syntax selected the place.
    pub(in crate::check) target: PlaceTarget,
    /// The source syntax node for diagnostics.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

impl Place {
    /// Create a place.
    pub(in crate::check) fn new(
        ty: dir::GlobalTypeId,
        target: PlaceTarget,
        source: dir::GlobalNodeIdAny,
    ) -> Self {
        Self { ty, target, source }
    }
}

/// How source syntax accesses a place expression.
///
/// Selection projects protocol direction from this fact: subscripts
/// select `index` for reads and `indexSet` for writes, members select
/// getters for reads and setters for writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum PlaceAccess {
    /// The place is only read.
    Read,
    /// The place is written as an assignment target.
    Write,
    /// The place is read and written as a compound assignment target.
    ReadWrite,
}

/// How source syntax selects a place.
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
        /// The local binding symbol selected by syntax.
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
        place: Place,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        self.build_writable_place_error(place)
    }

    /// Build the diagnostic for one place that rejects writes.
    fn build_writable_place_error(
        &mut self,
        place: Place,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let diagnostic = match place.target {
            PlaceTarget::Binding { symbol } => {
                self.build_writable_binding_error(place.source, symbol)?
            }
            PlaceTarget::Member { owner, key } => {
                self.build_writable_member_error(place.source, owner, key)?
            }
            PlaceTarget::Index { .. } | PlaceTarget::Dereference => Answer::Ready(None),
        };

        Ok(diagnostic)
    }

    /// Build the diagnostic for one binding that rejects writes.
    fn build_writable_binding_error(
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

    /// Build the diagnostic for one member that rejects writes.
    fn build_writable_member_error(
        &mut self,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);
        let owner = match self.evaluate_root(origin, owner)? {
            Answer::Ready(owner) => owner,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

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
