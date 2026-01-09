use destack_dir::{
    Expression, IfKind, LocalNodeId, NodeTree, SymbolTable, TypeBinaryOperator, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use crate::{Compiler, ElaborateError, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify a module to make abstractions concrete:
    /// - Implicit conversions → explicit cast nodes.
    /// - Operators → resolved method calls (`a + b` → `a.add(b)`).
    /// - Tree literals → constructor/function calls (`<div>` → `createElement(div, ...)`).
    /// - Range expressions → core range struct literals.
    /// - Nominal constructor calls → tagged expressions.
    pub(crate) fn elaborate_module_reify(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ElaborateResult<()> {
        // read the module state
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);

        // lock the dir tables
        let mut tree = dir.tree.write();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();

        // reify expression nodes
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.reify_expression(
                module_id,
                profile,
                expression_id,
                &mut tree,
                &symbols,
                &mut types,
                &module,
            )?;
        }

        Ok(())
    }

    /// Reify an expression.
    fn reify_expression(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // read the expression before mutating the tree
        let expression = tree.get(expression_id).clone();

        // reify expression forms that need concrete nodes
        match expression {
            // tree literals to constructor calls
            Expression::TreeExpression { .. } => {
                self.reify_tree_expression(expression_id, tree, symbols, types)?;
            }

            Expression::TypeBinary {
                left,
                operator: TypeBinaryOperator::Cast,
                right,
            } => {
                self.reify_explicit_cast_expression(
                    module_id,
                    profile,
                    expression_id,
                    left,
                    right,
                    tree,
                    symbols,
                    types,
                    module,
                )?;
            }

            Expression::Let { declarators, .. } | Expression::Using { declarators, .. } => {
                self.reify_implicit_casts_in_binding(
                    module_id,
                    profile,
                    &declarators,
                    tree,
                    symbols,
                    types,
                    module,
                )?;
            }

            Expression::Assign { left, right } => {
                self.reify_implicit_casts_in_assignment(
                    module_id,
                    profile,
                    expression_id,
                    left,
                    right,
                    tree,
                    symbols,
                    types,
                    module,
                )?;
            }

            Expression::Return { value } => {
                self.reify_implicit_casts_in_return(
                    module_id,
                    profile,
                    expression_id,
                    value,
                    tree,
                    symbols,
                    types,
                    module,
                )?;
            }

            Expression::If {
                kind: IfKind::Ternary,
                condition,
                then_expression,
                else_expression,
            } => {
                self.reify_implicit_casts_in_ternary(
                    module_id,
                    profile,
                    expression_id,
                    condition,
                    then_expression,
                    else_expression,
                    tree,
                    symbols,
                    types,
                    module,
                )?;
            }

            Expression::Match { cases, .. } => {
                self.reify_implicit_casts_in_match(
                    module_id,
                    profile,
                    expression_id,
                    &cases,
                    tree,
                    symbols,
                    types,
                    module,
                )?;
            }

            // operators to resolved method calls
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                self.reify_implicit_casts_in_binary(
                    module_id,
                    profile,
                    expression_id,
                    left,
                    operator,
                    right,
                    tree,
                    symbols,
                    types,
                    module,
                )?;
                self.reify_operator_expression(expression_id, tree, symbols, types)?;
            }

            // assign binary should be desugared during bind
            Expression::AssignBinary { .. } => {
                return Err(ElaborateError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(module_id)
                        .into_anchored(Some(profile)),
                });
            }

            // range expressions to core range structs
            Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                self.reify_range_expression(
                    expression_id,
                    start,
                    end,
                    is_inclusive,
                    tree,
                    symbols,
                    types,
                )?;
            }

            // nominal constructor calls to tagged expressions
            Expression::Call {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                self.reify_implicit_casts_in_call(
                    module_id,
                    profile,
                    expression_id,
                    &dynamic_arguments,
                    tree,
                    symbols,
                    types,
                    module,
                )?;
                self.reify_tagged_constructor_call(
                    module_id,
                    profile,
                    expression_id,
                    left,
                    &static_arguments,
                    &dynamic_arguments,
                    tree,
                    symbols,
                    types,
                    module,
                )?;
            }

            // type expressions to runtime type descriptors
            // #Incomplete: reify type expressions
            _ => {}
        }

        Ok(())
    }
}
