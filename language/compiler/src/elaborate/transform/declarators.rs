use destack_dir::{
    Asynchrony, Block, DeclarationDescriptor, Expression, LocalNodeId, Mutability, NodeTree,
    NodeType,
};

use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Split multi-declarator let statements into individual let statements.
    ///
    /// ```ds
    /// let a = 1, b = 2;
    /// ```
    /// ->
    /// ```ds
    /// let a = 1;
    /// let b = 2;
    /// ```
    pub(super) fn transform_split_declarators(&self, tree: &mut NodeTree) -> ElaborateResult<()> {
        // collect all blocks that need transformation
        let block_ids: Vec<_> = tree.iter_node_ids_of_type::<Block>();

        for block_id in block_ids {
            self.split_declarators_in_block(block_id, tree)?;
        }

        Ok(())
    }

    /// Split multi-declarators in a single block.
    fn split_declarators_in_block(
        &self,
        block_id: LocalNodeId<Block>,
        tree: &mut NodeTree,
    ) -> ElaborateResult<()> {
        #[derive(Clone, Copy)]
        enum BindingKind {
            Let { mutability: Mutability },
            Using { asynchrony: Asynchrony },
        }

        let block = tree.get(block_id).clone();
        let mut new_expressions: Vec<LocalNodeId<Expression>> = Vec::new();
        let mut modified = false;

        for expr_id in &block.expressions {
            // check if this is a statement wrapping a binding
            let (binding_expr_id, is_statement) = match tree.get(*expr_id) {
                Expression::Statement { statement } => (*statement, true),
                Expression::Let { .. } | Expression::Using { .. } => (*expr_id, false),
                _ => {
                    new_expressions.push(*expr_id);
                    continue;
                }
            };

            let (descriptor, binding_kind, declarators) = match tree.get(binding_expr_id).clone() {
                Expression::Let {
                    descriptor,
                    mutability,
                    declarators,
                } => (descriptor, BindingKind::Let { mutability }, declarators),
                Expression::Using {
                    asynchrony,
                    descriptor,
                    declarators,
                } => (descriptor, BindingKind::Using { asynchrony }, declarators),
                _ => {
                    new_expressions.push(*expr_id);
                    continue;
                }
            };

            // only split if there are multiple declarators
            if declarators.len() <= 1 {
                new_expressions.push(*expr_id);
                continue;
            }

            modified = true;
            let scope = tree.get_scope(binding_expr_id);

            // create individual binding for each declarator
            for declarator_id in declarators {
                let new_let_id = tree.reserve_from(
                    NodeType::Expression,
                    binding_expr_id.into_any(),
                    scope,
                    None,
                );

                // create a new descriptor for this binding using the declarator symbol
                let declarator = tree.get(declarator_id);
                let pattern = tree.get(declarator.pattern);
                let new_symbol = pattern.symbol().unwrap_or(descriptor.symbol);
                let new_descriptor = DeclarationDescriptor {
                    symbol: new_symbol,
                    ..descriptor.clone()
                };

                let new_binding = match binding_kind {
                    BindingKind::Let { mutability } => Expression::Let {
                        descriptor: new_descriptor,
                        mutability,
                        declarators: vec![declarator_id],
                    },
                    BindingKind::Using { asynchrony } => Expression::Using {
                        asynchrony,
                        descriptor: new_descriptor,
                        declarators: vec![declarator_id],
                    },
                };
                let new_let: LocalNodeId<Expression> = tree.insert(new_let_id, new_binding);

                // wrap in statement if original was wrapped
                let final_expr = if is_statement {
                    let stmt_id = tree.reserve_from(
                        NodeType::Expression,
                        binding_expr_id.into_any(),
                        scope,
                        None,
                    );
                    tree.insert(stmt_id, Expression::Statement { statement: new_let })
                } else {
                    new_let
                };

                new_expressions.push(final_expr);
            }
        }

        // update block if modified
        if modified {
            let new_block = Block {
                expressions: new_expressions,
                ..block
            };
            tree.replace(block_id, new_block);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_transform_split_declarators() {
        // multi-declarator let statements are split into individual lets
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): number {
    let a = 1, b = 2, c = 3;
    a + b + c
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): number {
    let a = 1;
    let b = 2;
    let c = 3;
    return a + b + c;
}
"#,
        );
    }

    #[test]
    fn test_transform_split_declarators_with_types() {
        // split declarators with type annotations (types are inferred after analysis)
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): number {
    let a: number = 1, b: number = 2;
    a + b
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): number {
    let a = 1;
    let b = 2;
    return a + b;
}
"#,
        );
    }

    #[test]
    fn test_transform_split_declarators_single_unchanged() {
        // single declarator let statements are unchanged
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): number {
    let a = 1;
    a
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): number {
    let a = 1;
    return a;
}
"#,
        );
    }
}
