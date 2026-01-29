use std::collections::HashSet;

use destack_dir::{GlobalSymbolId, LocalTypeId, PrimitiveType, TypeLiteral, TypeTable};
use destack_workspace::Module;

use super::r#type::TypeContainmentVisitor;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check whether a type contains the `any` literal.
    pub(crate) fn type_contains_any(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        self.type_contains_forbidden_literal(module, ty_id, types, literal_is_any, true)
    }

    /// Check whether a type contains the `unknown` literal.
    pub(crate) fn type_contains_unknown(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        self.type_contains_forbidden_literal(module, ty_id, types, literal_is_unknown, true)
    }

    /// Check whether a type contains imprecise primitive literals.
    pub(crate) fn type_contains_imprecise_primitive(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        self.type_contains_forbidden_literal(
            module,
            ty_id,
            types,
            literal_is_imprecise_primitive,
            false,
        )
    }

    /// Check whether a type contains a forbidden literal.
    fn type_contains_forbidden_literal(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        skip_imported_types: bool,
    ) -> bool {
        // track visited ids to prevent cycles
        let mut visited_types = HashSet::new();
        let mut visited_symbols = HashSet::new();

        // scan the type graph
        self.type_contains_forbidden_literal_inner(
            module,
            ty_id,
            types,
            predicate,
            skip_imported_types,
            &mut visited_types,
            &mut visited_symbols,
        )
    }

    /// Walk a type tree for forbidden literal usage.
    fn type_contains_forbidden_literal_inner(
        &self,
        module: &Module,
        ty_id: LocalTypeId,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        skip_imported_types: bool,
        visited_types: &mut HashSet<LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        let visitor = TypeContainmentVisitor::new_forbidden_literal(
            self,
            module,
            types,
            predicate,
            skip_imported_types,
            visited_types,
            visited_symbols,
        );
        visitor.contains(types, ty_id)
    }

    /// Check whether a reference symbol uses a forbidden literal.
    pub(super) fn type_reference_contains_forbidden_literal(
        &self,
        module: &Module,
        symbol_id: GlobalSymbolId,
        types: &TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        skip_imported_types: bool,
        visited_types: &mut HashSet<LocalTypeId>,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // skip remote symbols to avoid scanning other modules
        if symbol_id.module_id != module.id {
            return false;
        }

        // guard against cycles
        if !visited_symbols.insert(symbol_id) {
            return false;
        }

        // follow alias targets when present
        if let Some(alias_target) = types.get_alias_target_type_id(symbol_id) {
            return self.type_contains_forbidden_literal_inner(
                module,
                alias_target,
                types,
                predicate,
                skip_imported_types,
                visited_types,
                visited_symbols,
            );
        }

        // follow instance types when present
        let Some(instance_id) = types.get_instance_type_id(symbol_id) else {
            return false;
        };

        // scan the instance type
        self.type_contains_forbidden_literal_inner(
            module,
            instance_id,
            types,
            predicate,
            skip_imported_types,
            visited_types,
            visited_symbols,
        )
    }
}

/// Return true when the literal is `any`.
fn literal_is_any(value: &TypeLiteral) -> bool {
    matches!(value, TypeLiteral::Any)
}

/// Return true when the literal is `unknown`.
fn literal_is_unknown(value: &TypeLiteral) -> bool {
    matches!(value, TypeLiteral::Unknown)
}

/// Return true when the literal is an imprecise primitive.
fn literal_is_imprecise_primitive(value: &TypeLiteral) -> bool {
    matches!(value, TypeLiteral::Primitive(PrimitiveType::Number))
}
