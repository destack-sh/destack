use std::collections::HashSet;

use destack_dir::{
    GlobalSymbolId, LocalTypeId, PrimitiveType, StaticExpression, Type, TypeLiteral, TypeTable,
    TypeUnaryOperator, TypeVisitor, TypeVisitorOptions, walk_static_expression, walk_type,
};
use destack_workspace::Module;

use crate::Compiler;

/// Walk types to detect forbidden type literals.
struct ForbiddenLiteralVisitor<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The current module.
    module: &'a Module,
    /// The type table for the current module.
    types: &'a TypeTable,
    /// The literal predicate used for filtering.
    predicate: fn(&TypeLiteral) -> bool,
    /// Whether imported types should be skipped.
    skip_imported_types: bool,
    /// The visited type ids.
    visited_types: &'a mut HashSet<LocalTypeId>,
    /// The visited symbols.
    visited_symbols: &'a mut HashSet<GlobalSymbolId>,
    /// Whether a forbidden literal was found.
    found: bool,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl<'a> ForbiddenLiteralVisitor<'a> {
    /// Create a visitor for forbidden literal detection.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        types: &'a TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        skip_imported_types: bool,
        visited_types: &'a mut HashSet<LocalTypeId>,
        visited_symbols: &'a mut HashSet<GlobalSymbolId>,
    ) -> Self {
        Self {
            compiler,
            module,
            types,
            predicate,
            skip_imported_types,
            visited_types,
            visited_symbols,
            found: false,
            options: TypeVisitorOptions::default(),
        }
    }
}

impl TypeVisitor for ForbiddenLiteralVisitor<'_> {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        if self.found {
            return;
        }
        if self.skip_imported_types && types.is_imported_type(id) {
            return;
        }
        if !self.visited_types.insert(id) {
            return;
        }
        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
    }

    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        if self.found {
            return;
        }
        match ty {
            Type::TypeLiteral { value } => {
                if (self.predicate)(value) {
                    self.found = true;
                }
            }
            Type::Unary {
                operator: TypeUnaryOperator::Keyof,
                ..
            } => {}
            Type::Reference { symbol, .. } => {
                if self.compiler.type_reference_contains_forbidden_literal(
                    self.module,
                    *symbol,
                    self.types,
                    self.predicate,
                    self.skip_imported_types,
                    self.visited_types,
                    self.visited_symbols,
                ) {
                    self.found = true;
                }
            }
            _ => walk_type(self, types, id, ty),
        }
    }

    fn visit_static_expression(&mut self, types: &TypeTable, expression: &StaticExpression) {
        if self.found {
            return;
        }
        if let StaticExpression::TypeLiteral { value } = expression {
            if (self.predicate)(value) {
                self.found = true;
            }
            return;
        }
        walk_static_expression(self, types, expression);
    }
}

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
        let mut visitor = ForbiddenLiteralVisitor::new(
            self,
            module,
            types,
            predicate,
            skip_imported_types,
            visited_types,
            visited_symbols,
        );
        visitor.visit_type_id(types, ty_id);
        visitor.found
    }

    /// Check whether a reference symbol uses a forbidden literal.
    fn type_reference_contains_forbidden_literal(
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
