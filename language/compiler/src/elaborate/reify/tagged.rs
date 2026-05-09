use std::collections::HashSet;

use destack_dir as dir;
use destack_source::ModuleId;
use dir::{
    Argument, BindingTable, Declaration, Expression, GlobalSymbolId, LocalNodeId, LocalTypeId,
    NodeType, Tree, Type, TypeExpression, TypeTable, UnevaluatedType,
};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateError, ElaborateResult};

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
    /// The tree for declaration lookup.
    tree: &'a Tree,
    /// The symbol table for declaration mapping.
    symbols: &'a BindingTable,
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
        let callee_node = callee_id.into_global_any(state.module_id);
        let Some(callee_symbol) = state.types.symbol_resolution(callee_node) else {
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
        if symbol.module_id == state.module.id {
            let view = NominalLookupView {
                module_id: state.module.id,
                tree: state.tree,
                symbols: state.symbols,
                types: state.types,
            };
            return Ok(self.nominal_constructor_kind_for_symbol_in_dir(symbol, view));
        }

        // load the remote module data for imported symbols
        let declared = self
            .dir_declared(state.provider, symbol.module_id, state.profile)
            .map_err(|_| ElaborateError::UnsupportedConstruct {
                anchor: symbol.module_id.into(),
            })?;
        let checked = self
            .dir_checked(state.provider, symbol.module_id, state.profile)
            .map_err(|_| ElaborateError::UnsupportedConstruct {
                anchor: symbol.module_id.into(),
            })?;

        let view = NominalLookupView {
            module_id: symbol.module_id,
            tree: &declared.tree,
            symbols: &declared.bindings,
            types: &checked.types,
        };
        Ok(self.nominal_constructor_kind_for_symbol_in_dir(symbol, view))
    }

    /// Resolve the constructor kind for a symbol using a specific module dir.
    fn nominal_constructor_kind_for_symbol_in_dir(
        &self,
        symbol: GlobalSymbolId,
        view: NominalLookupView<'_>,
    ) -> Option<ConstructorKind> {
        // use the declaration for nominal type aliases
        let symbol_entry = view.symbols.get_symbol(symbol.local_id);
        let declaration = symbol_entry.declaration?;
        let Ok(declaration) = declaration.try_into_typed::<Declaration>() else {
            return None;
        };
        let declaration_id: LocalNodeId<Declaration> = declaration.into();
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
            Type::Unevaluated(UnevaluatedType {
                expression: expression_id,
            }) => match view.tree.get(*expression_id) {
                TypeExpression::Tuple { .. }
                | TypeExpression::ArrayTuple { .. }
                | TypeExpression::Array { .. }
                | TypeExpression::Slice { .. } => ConstructorKind::Tuple,
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
            Type::Tuple(_) => ConstructorKind::Tuple,
            Type::Object(_) => ConstructorKind::Object,
            Type::Value(value) => self.constructor_kind_for_type_id(view, value.value, visited),
            Type::Reference(reference) => {
                if let Some(instance_id) = view.types.get_instance_type_id(reference.symbol) {
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
        let callee_id = unwrap_parenthesized_expression(callee_id, state.tree);
        let callee = state.tree.get(callee_id).clone();
        let scope = state.tree.get_scope(callee_id);
        let parent_id = state.tree.get_parent(callee_id.id);
        let generic_arguments = generic_arguments.to_vec();

        match callee {
            Expression::Path {
                path,
                generic_arguments: callee_generic_arguments,
            } => {
                let type_expression_id = state.tree.reserve_from(
                    NodeType::TypeExpression,
                    callee_id.into_any(),
                    scope,
                    parent_id,
                    Some(dir::ProvenanceReason::Elaborated),
                );

                let type_expression_id = state.tree.insert(
                    type_expression_id,
                    TypeExpression::Reference {
                        path,
                        generic_arguments: if generic_arguments.is_empty() {
                            callee_generic_arguments
                        } else {
                            generic_arguments
                        },
                    },
                );

                let source_node = callee_id.into_global_any(state.module_id);
                let target_node = type_expression_id.into_global_any(state.module_id);
                if let Some(symbol_id) = state.types.symbol_resolution(source_node) {
                    state.types.set_symbol_resolution(target_node, symbol_id);
                }

                Some(type_expression_id)
            }
            Expression::Member { left, name } => {
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
                        generic_arguments,
                    },
                ))
            }
            _ => None,
        }
    }
}

/// Unwrap parenthesized expressions.
fn unwrap_parenthesized_expression(
    mut expression_id: LocalNodeId<Expression>,
    tree: &Tree,
) -> LocalNodeId<Expression> {
    loop {
        let Expression::Parenthesized { expression } = tree.get(expression_id) else {
            return expression_id;
        };
        expression_id = *expression;
    }
}
