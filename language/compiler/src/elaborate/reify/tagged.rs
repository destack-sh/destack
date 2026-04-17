use std::collections::HashSet;

use destack_dir as dir;
use destack_source::ModuleId;
use dir::{
    Argument, Declaration, Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, NodeTree,
    NodeType, SymbolTable, Type, TypeExpression, TypeTable,
};

use crate::analyze::TreeSymbolView;
use crate::elaborate::common::ElaborateState;
use crate::{Compiler, ElaborateResult};

/// The constructor kind inferred for a nominal type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConstructorKind {
    /// The constructor wraps a single scalar value.
    Scalar,
    /// The constructor wraps a tuple of elements.
    Tuple,
    /// The constructor wraps an object literal.
    Object,
}

/// The table view used for nominal constructor lookup.
#[derive(Clone, Copy)]
struct NominalLookupView<'a> {
    /// The module id for global node conversion.
    module_id: ModuleId,
    /// The node tree for declaration lookup.
    tree: &'a NodeTree,
    /// The symbol table for declaration mapping.
    symbols: &'a SymbolTable,
    /// The type table for constructor classification.
    types: &'a TypeTable,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify nominal constructor calls into tagged expressions.
    pub(super) fn reify_tagged_constructor_call(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        callee: LocalNodeId<Expression>,
        generic_arguments: &[LocalNodeId<dir::GenericArgument>],
        arguments: &[LocalNodeId<Argument>],
    ) -> ElaborateResult<bool> {
        // resolve the callee symbol for nominal constructor calls
        let Some(callee_id) =
            self.insert_constructor_callee_type_expression(state, callee, generic_arguments)
        else {
            return Ok(false);
        };
        let Some(callee_symbol) = self.reference_symbol_for_type_expression(
            TreeSymbolView::new(
                state.ctx.compiler_context,
                state.ctx.module,
                state.ctx.profile,
                state.tree,
                state.symbols,
            ),
            callee_id,
        ) else {
            return Ok(false);
        };

        // determine the constructor kind from the nominal declaration
        let Some(constructor_kind) =
            self.nominal_constructor_kind_for_symbol(state, callee_symbol)?
        else {
            return Ok(false);
        };

        // object constructors are not represented as call expressions
        if constructor_kind == ConstructorKind::Object {
            return Ok(false);
        }

        // scalar constructors require exactly one argument
        if constructor_kind == ConstructorKind::Scalar && arguments.len() != 1 {
            return Ok(false);
        }

        // replace the call with the tagged constructor expression
        match constructor_kind {
            ConstructorKind::Scalar => {
                let argument_id = arguments[0];
                let value_id = state.tree.get(argument_id).value();
                state.tree.replace(
                    expression_id,
                    Expression::TaggedScalarExpression {
                        ty: callee_id,
                        value: value_id,
                    },
                );
            }
            ConstructorKind::Tuple => {
                state.tree.replace(
                    expression_id,
                    Expression::TaggedTupleExpression {
                        ty: callee_id,
                        elements: arguments.to_vec(),
                    },
                );
            }
            ConstructorKind::Object => {}
        }

        Ok(true)
    }

    /// Resolve the constructor kind for a nominal type symbol.
    fn nominal_constructor_kind_for_symbol(
        &self,
        state: &ElaborateState<'_>,
        symbol: GlobalSymbolId,
    ) -> ElaborateResult<Option<ConstructorKind>> {
        // use current module data when the symbol is local
        if symbol.module_id == state.ctx.module.id {
            let view = NominalLookupView {
                module_id: state.ctx.module.id,
                tree: state.tree,
                symbols: state.symbols,
                types: state.types,
            };
            return Ok(self.nominal_constructor_kind_for_symbol_in_dir(symbol, view));
        }

        // load the remote module data for imported symbols
        let dir = self
            .require_artifact_dir_analyzed(
                state.ctx.compiler_context.revision(),
                symbol.module_id,
                state.ctx.profile,
            )
            .map_err(|error| self.elaborate_error_from_requirement(error))?;

        let view = NominalLookupView {
            module_id: symbol.module_id,
            tree: &dir.tree,
            symbols: &dir.symbols,
            types: &dir.types,
        };
        Ok(self.nominal_constructor_kind_for_symbol_in_dir(symbol, view))
    }

    /// Resolve the constructor kind for a symbol using a specific module dir.
    fn nominal_constructor_kind_for_symbol_in_dir(
        &self,
        symbol: GlobalSymbolId,
        view: NominalLookupView<'_>,
    ) -> Option<ConstructorKind> {
        // use the primary declaration for nominal type aliases
        let symbol_entry = view.symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        let Ok(primary_declaration) = primary_declaration.try_into_typed::<Declaration>() else {
            return None;
        };
        let declaration_id: LocalNodeId<Declaration> = primary_declaration.into();
        let declaration = view.tree.get(declaration_id);

        // only newtype aliases use constructor call tagging
        let Declaration::Type(declaration) = declaration else {
            return None;
        };
        if !declaration.is_nominal {
            return None;
        }

        // derive the constructor kind from the evaluated alias type
        let declared_type_id = view
            .types
            .get_declared_type_id(declaration.value.into_global_any(view.module_id))?;
        let constructor_kind = match view.types.get_type(declared_type_id) {
            Type::Unevaluated(expression_id) => match view.tree.get(*expression_id) {
                TypeExpression::Tuple { .. } | TypeExpression::Array { .. } => {
                    ConstructorKind::Tuple
                }
                TypeExpression::Object { .. } => ConstructorKind::Object,
                _ => ConstructorKind::Scalar,
            },
            _ => {
                let mut visited = HashSet::new();
                self.constructor_kind_for_type_id(view, declared_type_id, &mut visited)
            }
        };

        Some(constructor_kind)
    }

    /// Classify a type id into a constructor kind.
    fn constructor_kind_for_type_id(
        &self,
        view: NominalLookupView<'_>,
        type_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> ConstructorKind {
        // break cycles by defaulting to scalar
        if !visited.insert(type_id) {
            return ConstructorKind::Scalar;
        }

        match view.types.get_type(type_id) {
            Type::Tuple { .. } => ConstructorKind::Tuple,
            Type::Object { .. } => ConstructorKind::Object,
            Type::Value { value } => self.constructor_kind_for_type_id(view, *value, visited),
            Type::Reference { symbol, .. } => {
                if let Some(instance_id) = view.types.get_instance_type_id(*symbol) {
                    self.constructor_kind_for_type_id(view, instance_id, visited)
                } else {
                    ConstructorKind::Scalar
                }
            }
            _ => ConstructorKind::Scalar,
        }
    }

    /// Insert a type expression for one constructor callee.
    fn insert_constructor_callee_type_expression(
        &self,
        state: &mut ElaborateState<'_>,
        callee_id: LocalNodeId<Expression>,
        generic_arguments: &[LocalNodeId<dir::GenericArgument>],
    ) -> Option<LocalNodeId<TypeExpression>> {
        let callee_id = self.unwrap_parenthesized_expression(callee_id, state.tree);
        let callee = state.tree.get(callee_id).clone();
        let scope = state.tree.get_scope(callee_id);
        let parent_id = state.tree.get_parent(callee_id.id);
        let generic_arguments = generic_arguments.to_vec();

        match callee {
            Expression::LocalReference {
                path,
                target_symbol,
                generic_arguments: callee_generic_arguments,
            } => {
                let type_expression_id = state.tree.reserve_from(
                    NodeType::TypeExpression,
                    callee_id.into_any(),
                    scope,
                    parent_id,
                    Some(dir::ProvenanceReason::Elaborated),
                );

                Some(state.tree.insert(
                    type_expression_id,
                    TypeExpression::LocalReference {
                        path,
                        generic_arguments: if generic_arguments.is_empty() {
                            callee_generic_arguments
                        } else {
                            generic_arguments
                        },
                        target_symbol,
                    },
                ))
            }
            Expression::ModuleReference {
                path,
                target_symbol,
                generic_arguments: callee_generic_arguments,
            } => {
                let type_expression_id = state.tree.reserve_from(
                    NodeType::TypeExpression,
                    callee_id.into_any(),
                    scope,
                    parent_id,
                    Some(dir::ProvenanceReason::Elaborated),
                );

                Some(state.tree.insert(
                    type_expression_id,
                    TypeExpression::ModuleReference {
                        path,
                        generic_arguments: if generic_arguments.is_empty() {
                            callee_generic_arguments
                        } else {
                            generic_arguments
                        },
                        target_symbol,
                    },
                ))
            }
            Expression::GlobalReference {
                path,
                target_symbol,
                generic_arguments: callee_generic_arguments,
            } => {
                let type_expression_id = state.tree.reserve_from(
                    NodeType::TypeExpression,
                    callee_id.into_any(),
                    scope,
                    parent_id,
                    Some(dir::ProvenanceReason::Elaborated),
                );

                Some(state.tree.insert(
                    type_expression_id,
                    TypeExpression::GlobalReference {
                        path,
                        generic_arguments: if generic_arguments.is_empty() {
                            callee_generic_arguments
                        } else {
                            generic_arguments
                        },
                        target_symbol,
                    },
                ))
            }
            Expression::Member {
                left,
                name,
                generic_arguments: callee_generic_arguments,
            } => {
                let name = name?;
                let left = self.insert_constructor_callee_type_expression(state, left, &[])?;
                let type_expression_id = state.tree.reserve_from(
                    NodeType::TypeExpression,
                    callee_id.into_any(),
                    scope,
                    parent_id,
                    Some(dir::ProvenanceReason::Elaborated),
                );

                Some(state.tree.insert(
                    type_expression_id,
                    TypeExpression::Member {
                        left,
                        name,
                        generic_arguments: if generic_arguments.is_empty() {
                            callee_generic_arguments
                        } else {
                            generic_arguments
                        },
                    },
                ))
            }
            _ => None,
        }
    }
}
