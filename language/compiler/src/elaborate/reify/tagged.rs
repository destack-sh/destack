use std::collections::HashSet;

use destack_dir::{
    Argument, Declaration, Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, NodeTree,
    SymbolTable, Type, TypeKind, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use crate::analyze::TreeSymbolView;
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

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify nominal constructor calls into tagged expressions.
    pub(super) fn reify_tagged_constructor_call(
        &self,
        _module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        callee: LocalNodeId<Expression>,
        static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<bool> {
        // resolve the callee symbol for nominal constructor calls
        let callee_id = self.unwrap_parenthesized_expression(callee, tree);
        let Some(callee_symbol) = self.reference_symbol_for_expression(
            TreeSymbolView::new(module, profile, tree, symbols),
            callee_id,
        ) else {
            return Ok(false);
        };

        // determine the constructor kind from the nominal declaration
        let Some(constructor_kind) = self.nominal_constructor_kind_for_symbol(
            callee_symbol,
            module,
            profile,
            tree,
            symbols,
            types,
        ) else {
            return Ok(false);
        };

        // object constructors are not represented as call expressions
        if constructor_kind == ConstructorKind::Object {
            return Ok(false);
        }

        // scalar constructors require exactly one argument
        if constructor_kind == ConstructorKind::Scalar && dynamic_arguments.len() != 1 {
            return Ok(false);
        }

        // move static arguments onto the callee reference when present
        if let Some(static_arguments) = static_arguments.as_ref()
            && !static_arguments.is_empty()
        {
            self.apply_static_arguments_to_callee(callee_id, static_arguments, tree);
        }

        // replace the call with the tagged constructor expression
        match constructor_kind {
            ConstructorKind::Scalar => {
                let argument_id = dynamic_arguments[0];
                let value_id = tree.get(argument_id).value();
                tree.replace(
                    expression_id,
                    Expression::TaggedScalarExpression {
                        ty: callee_id,
                        value: value_id,
                    },
                );
            }
            ConstructorKind::Tuple => {
                tree.replace(
                    expression_id,
                    Expression::TaggedTupleExpression {
                        ty: callee_id,
                        elements: dynamic_arguments.to_vec(),
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
        symbol: GlobalSymbolId,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<ConstructorKind> {
        // use current module data when the symbol is local
        if symbol.module_id == module.id {
            return self.nominal_constructor_kind_for_symbol_in_dir(
                symbol, module.id, tree, symbols, types,
            );
        }

        // load the remote module data for imported symbols
        let module = self.program.modules.get(symbol.module_id);
        let module = module.read();
        let dir = module.dir(profile);

        // borrow the remote dir tables
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let types = dir.types.read();

        self.nominal_constructor_kind_for_symbol_in_dir(
            symbol,
            symbol.module_id,
            &tree,
            &symbols,
            &types,
        )
    }

    /// Resolve the constructor kind for a symbol using a specific module dir.
    fn nominal_constructor_kind_for_symbol_in_dir(
        &self,
        symbol: GlobalSymbolId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<ConstructorKind> {
        // use the primary declaration for nominal type aliases
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        let Ok(primary_declaration) = primary_declaration.try_into_typed::<Declaration>() else {
            return None;
        };
        let declaration_id: LocalNodeId<Declaration> = primary_declaration.into();
        let declaration = tree.get(declaration_id);

        // only newtype aliases use constructor call tagging
        let Declaration::Type {
            kind: TypeKind::Nominal,
            value,
            ..
        } = declaration
        else {
            return None;
        };

        // derive the constructor kind from the evaluated alias type
        let declared_type_id = types.get_declared_type_id(value.into_global_any(module_id))?;
        let constructor_kind = match types.get_type(declared_type_id) {
            Type::Unevaluated(expression_id) => match tree.get(*expression_id) {
                Expression::TupleExpression { .. } | Expression::ArrayExpression { .. } => {
                    ConstructorKind::Tuple
                }
                Expression::ObjectExpression { .. } => ConstructorKind::Object,
                _ => ConstructorKind::Scalar,
            },
            _ => {
                let mut visited = HashSet::new();
                self.constructor_kind_for_type_id(declared_type_id, types, &mut visited)
            }
        };

        Some(constructor_kind)
    }

    /// Classify a type id into a constructor kind.
    fn constructor_kind_for_type_id(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> ConstructorKind {
        // break cycles by defaulting to scalar
        if !visited.insert(type_id) {
            return ConstructorKind::Scalar;
        }

        match types.get_type(type_id) {
            Type::Tuple { .. } => ConstructorKind::Tuple,
            Type::Object { .. } => ConstructorKind::Object,
            Type::Value { value } => self.constructor_kind_for_type_id(*value, types, visited),
            Type::Reference { symbol, .. } => {
                if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                    self.constructor_kind_for_type_id(instance_id, types, visited)
                } else {
                    ConstructorKind::Scalar
                }
            }
            _ => ConstructorKind::Scalar,
        }
    }

    /// Apply call static arguments to a callee expression.
    fn apply_static_arguments_to_callee(
        &self,
        callee_id: LocalNodeId<Expression>,
        static_arguments: &[LocalNodeId<Argument>],
        tree: &mut NodeTree,
    ) {
        // update reference expressions to carry static arguments
        let expression = tree.get(callee_id).clone();
        match expression {
            Expression::LocalReference {
                path,
                target_symbol,
                static_arguments: None,
            } => {
                tree.replace(
                    callee_id,
                    Expression::LocalReference {
                        path,
                        target_symbol,
                        static_arguments: Some(static_arguments.to_vec()),
                    },
                );
            }
            Expression::ModuleReference {
                path,
                target_symbol,
                static_arguments: None,
            } => {
                tree.replace(
                    callee_id,
                    Expression::ModuleReference {
                        path,
                        target_symbol,
                        static_arguments: Some(static_arguments.to_vec()),
                    },
                );
            }
            Expression::GlobalReference {
                path,
                target_symbol,
                static_arguments: None,
            } => {
                tree.replace(
                    callee_id,
                    Expression::GlobalReference {
                        path,
                        target_symbol,
                        static_arguments: Some(static_arguments.to_vec()),
                    },
                );
            }
            Expression::Member {
                left,
                name,
                static_arguments: None,
            } => {
                tree.replace(
                    callee_id,
                    Expression::Member {
                        left,
                        name,
                        static_arguments: Some(static_arguments.to_vec()),
                    },
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::{TestProgram, root_expression_id};
    use destack_dir::Expression;

    /// Reify scalar constructor calls into tagged scalar expressions.
    #[test]
    fn test_reify_nominal_scalar_constructor_call() {
        // build a scalar newtype constructor call
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
newtype UserId = int64;

let value = UserId(42);
"#,
        );

        // run the elaborate pipeline
        test.elaborate_module(module_id);
        test.compile_check_clean();
        // surface output remains a constructor call
        test.assert_elaborated(
            module_id,
            r#"
newtype UserId = int64;

let value = UserId(42);
"#,
        );

        // inspect the initializer expression
        test.with_dir_read(
            module_id,
            |_module, _profile, dir, tree, _symbols, _types| {
                // locate the let initializer
                let roots = dir.roots.clone();
                let expression_id = root_expression_id(&roots, tree, 1);
                let Expression::Let { declarators, .. } = tree.get(expression_id) else {
                    panic!("expected let expression");
                };

                // read the initializer expression
                let declarator_id = declarators.first().copied().expect("expected declarator");
                let declarator = tree.get(declarator_id);
                let value_id = declarator.value.expect("expected initializer value");

                // ensure a tagged scalar expression was created
                let Expression::TaggedScalarExpression { ty: _, value: _ } = tree.get(value_id)
                else {
                    panic!("expected tagged scalar expression");
                };
            },
        );
    }

    /// Reify tuple constructor calls into tagged tuple expressions.
    #[test]
    fn test_reify_nominal_tuple_constructor_call() {
        // build a tuple newtype constructor call
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
newtype Point = (float32, float32);

let value = Point(1.0, 2.0);
"#,
        );

        // run the elaborate pipeline
        test.elaborate_module(module_id);
        test.compile_check_clean();
        // surface output remains a constructor call
        test.assert_elaborated(
            module_id,
            r#"
newtype Point = (float32, float32,);

let value = Point(1, 2);
"#,
        );

        // inspect the initializer expression
        test.with_dir_read(
            module_id,
            |_module, _profile, dir, tree, _symbols, _types| {
                // locate the let initializer
                let roots = dir.roots.clone();
                let expression_id = root_expression_id(&roots, tree, 1);
                let Expression::Let { declarators, .. } = tree.get(expression_id) else {
                    panic!("expected let expression");
                };

                // read the initializer expression
                let declarator_id = declarators.first().copied().expect("expected declarator");
                let declarator = tree.get(declarator_id);
                let value_id = declarator.value.expect("expected initializer value");

                // ensure a tagged tuple expression was created
                let Expression::TaggedTupleExpression { ty: _, elements } = tree.get(value_id)
                else {
                    panic!("expected tagged tuple expression");
                };
                assert_eq!(elements.len(), 2);
            },
        );
    }

    /// Move static arguments onto tagged tuple constructor expressions.
    #[test]
    fn test_reify_nominal_constructor_static_arguments() {
        // build a generic newtype constructor call
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
newtype Box<T> = (T,);

let value = Box<int32>(1);
"#,
        );

        // run the elaborate pipeline
        test.elaborate_module(module_id);
        test.compile_check_clean();
        // surface output remains a constructor call
        test.assert_elaborated(
            module_id,
            r#"
newtype Box<T> = (T,);

let value = Box<int32>(1);
"#,
        );

        // inspect the initializer expression
        test.with_dir_read(
            module_id,
            |_module, _profile, dir, tree, _symbols, _types| {
                // locate the let initializer
                let roots = dir.roots.clone();
                let expression_id = root_expression_id(&roots, tree, 1);
                let Expression::Let { declarators, .. } = tree.get(expression_id) else {
                    panic!("expected let expression");
                };

                // read the initializer expression
                let declarator_id = declarators.first().copied().expect("expected declarator");
                let declarator = tree.get(declarator_id);
                let value_id = declarator.value.expect("expected initializer value");

                // verify tagged tuple constructor uses static arguments on the tag
                let Expression::TaggedTupleExpression { ty, elements: _ } = tree.get(value_id)
                else {
                    panic!("expected tagged tuple expression");
                };

                let ty_expression = tree.get(*ty);
                let arguments = match ty_expression {
                    Expression::LocalReference {
                        static_arguments: Some(arguments),
                        ..
                    }
                    | Expression::ModuleReference {
                        static_arguments: Some(arguments),
                        ..
                    }
                    | Expression::GlobalReference {
                        static_arguments: Some(arguments),
                        ..
                    }
                    | Expression::Member {
                        static_arguments: Some(arguments),
                        ..
                    } => arguments,
                    _ => {
                        panic!(
                            "expected tag reference with static arguments, got {ty_expression:?}"
                        );
                    }
                };

                assert_eq!(arguments.len(), 1);
            },
        );
    }
}
