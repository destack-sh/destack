use destack_mir::{
    Access, LocalNodeIdAny, Place, Reference, Substitution, Type, TypeId, Value, is_copy,
};

use crate::verify::VerifyError;

use super::checker::FunctionChecker;

impl FunctionChecker<'_, '_> {
    /// Check the access one borrowed place grants.
    pub(super) fn check_address_grant(
        &mut self,
        reference: Value,
        place: &Place,
        anchor: LocalNodeIdAny,
    ) {
        let reference_type = self.value_type(reference);
        let kind = reference_type.reference_kind();
        let access = reference_type
            .reference_access()
            .filter(|_| kind != Some(Reference::Raw));

        // require constant or retained storage behind a managed reference, which outlives every origin
        let error = if matches!(kind, Some(Reference::Managed(_)))
            && !place.is_constant(self.tree)
            && !place.is_retained(self.function_id, self.tree)
        {
            Some(VerifyError::BorrowOutlivesOrigin {
                anchor: self.anchor(anchor),
            })
        }
        // preserve the requested access through every safe indirection
        else if access.is_some_and(|access| !self.place_grants(place, access, anchor)) {
            Some(VerifyError::BorrowAccessStrengthening {
                anchor: self.anchor(anchor),
            })
        }
        // require an immutable grant on every enclosing variant, which fixes its case
        else if access.is_some()
            && place
                .variants()
                .any(|variant| !self.place_grants(&variant, Access::Immutable, anchor))
        {
            Some(VerifyError::BorrowOfAliasableVariant {
                anchor: self.anchor(anchor),
            })
        }
        // accept the borrowed place
        else {
            None
        };
        if let Some(error) = error {
            self.verification.emit_error(error);
            self.reject_borrow(reference);
        }
    }

    /// Return whether every traversed safe reference grants the requested access.
    ///
    /// A handle of a fresh allocation at the root has no alias yet, so aliasing narrows none of its grants.
    fn place_grants(&self, place: &Place, requested: Access, anchor: LocalNodeIdAny) -> bool {
        let is_fresh = self.places.is_fresh_at(place, anchor);

        place.dereferences(self.function_id, self.tree).fold(
            true,
            |is_granted, (length, reference)| match reference.dereference_kind() {
                // reset the grant at an unchecked indirection, which its author vouches for
                Reference::Raw => true,
                // admit an immutable borrow through the sole path to unique storage
                Reference::Unique => {
                    let access = reference
                        .reference_access()
                        .unwrap_or_else(|| unreachable!("a unique reference has no access"));
                    is_granted
                        && (access.grants(requested)
                            || requested == Access::Immutable
                            || (requested == Access::Exclusive && access.grants(Access::Mutable)))
                }
                // narrow the grant by what the handle admits to its object
                Reference::Managed(_) => {
                    is_granted && self.handle_grants(reference, requested, is_fresh && length == 0)
                }
                // narrow the grant by what the borrow itself permits
                Reference::Borrowed => {
                    let access = reference
                        .reference_access()
                        .unwrap_or_else(|| unreachable!("a borrowed reference has no access"));
                    is_granted && access.grants(requested)
                }
            },
        )
    }

    /// Return whether one handle grants the requested access to its object.
    ///
    /// A handle grants `&readonly` always and `&` at its own access.
    /// It grants `&immutable` while its object holds no inline variant an alias could retag.
    /// It grants `&exclusive` at mutable access while its object also has only Copy components.
    /// A fresh handle has no alias, so it grants `&immutable` always and `&exclusive` at mutable access.
    fn handle_grants(&self, handle: &Type, requested: Access, is_fresh: bool) -> bool {
        let access = handle
            .reference_access()
            .unwrap_or_else(|| unreachable!("a handle has no access"));
        let referent = handle.pointee_type();

        match requested {
            Access::Readonly => true,
            Access::Mutable => access.grants(Access::Mutable),
            Access::Immutable => {
                is_fresh
                    || referent.is_some_and(|referent| {
                        !self.holds_inline_variant(referent, &mut Vec::new())
                    })
            }
            Access::Exclusive => {
                access.grants(Access::Mutable)
                    && (is_fresh
                        || referent.is_some_and(|referent| {
                            !self.holds_inline_variant(referent, &mut Vec::new())
                                && self.components_copy(referent, &mut Vec::new())
                        }))
            }
            Access::Parameter(_) => access == requested,
        }
    }

    /// Return whether one type, inline or through unique pointees, holds a variant of several cases.
    fn holds_inline_variant(&self, ty: TypeId, visited: &mut Vec<TypeId>) -> bool {
        let ty = Substitution::resolve(ty, self.tree);
        if visited.contains(&ty) {
            return false;
        }
        visited.push(ty);

        // descend through inline components, variant payloads, and unique pointees
        match self.tree.type_definition(ty) {
            Type::Variant { cases, .. } => {
                cases.len() > 1
                    || cases
                        .iter()
                        .any(|case| self.holds_inline_variant(case.ty, visited))
            }
            Type::Struct { fields } => fields
                .iter()
                .any(|field| self.holds_inline_variant(self.tree.get(*field).ty, visited)),
            Type::Tuple { elements } => elements
                .iter()
                .any(|element| self.holds_inline_variant(*element, visited)),
            Type::FixedArray { element, .. } | Type::Vector { element, .. } => {
                self.holds_inline_variant(*element, visited)
            }
            Type::Newtype { value } | Type::Uninit { value } | Type::ManuallyDrop { value } => {
                self.holds_inline_variant(*value, visited)
            }
            Type::Reference {
                kind: Reference::Unique,
                pointee,
                ..
            } => self.holds_inline_variant(*pointee, visited),
            Type::Slice {
                kind: Reference::Unique,
                element,
                ..
            } => self.holds_inline_variant(*element, visited),
            _ => false,
        }
    }

    /// Return whether every inline component of one type copies.
    fn components_copy(&self, ty: TypeId, visited: &mut Vec<TypeId>) -> bool {
        let ty = Substitution::resolve(ty, self.tree);
        if visited.contains(&ty) {
            return true;
        }
        visited.push(ty);

        // descend through inline components to the leaves that copy
        match self.tree.type_definition(ty) {
            Type::Struct { fields } => fields
                .iter()
                .all(|field| self.components_copy(self.tree.get(*field).ty, visited)),
            Type::Tuple { elements } => elements
                .iter()
                .all(|element| self.components_copy(*element, visited)),
            Type::Variant { cases, .. } => cases
                .iter()
                .all(|case| self.components_copy(case.ty, visited)),
            Type::FixedArray { element, .. } | Type::Vector { element, .. } => {
                self.components_copy(*element, visited)
            }
            Type::Newtype { value } | Type::ManuallyDrop { value } => {
                self.components_copy(*value, visited)
            }
            _ => is_copy(self.tree, ty, &self.function.generics),
        }
    }

    /// Check write access through every reference one written place traverses.
    pub(super) fn check_write_grant(&mut self, place: &Place, anchor: LocalNodeIdAny) {
        if !self.place_grants(place, Access::Mutable, anchor) {
            self.verification
                .emit_error(VerifyError::WriteThroughReadonlyReference {
                    anchor: self.anchor(anchor),
                });
        }
    }
}
