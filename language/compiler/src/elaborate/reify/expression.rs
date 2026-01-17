use std::collections::HashSet;

use destack_dir::{
    Expression, IfCondition, IfKind, LocalNodeId, MatchKind, NodeTree, SymbolTable,
    TypeBinaryOperator, TypeTable,
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
        // ensure transform phase is complete
        self.require_elaborate_module_transform(module_id, profile)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // read the module state
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);

        // lock the dir tables
        let mut tree = dir.tree.write();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();

        // collect member expressions used as call or new callees
        let mut member_callees: HashSet<u32> = HashSet::new();
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            if !self.is_node_active(&tree, &symbols, expression_id.into_any()) {
                continue;
            }

            let (Expression::Call { left, .. } | Expression::New { left, .. }) =
                tree.get(expression_id)
            else {
                continue;
            };

            let mut callee_id = *left;
            loop {
                let Expression::Parenthesized { expression } = tree.get(callee_id) else {
                    break;
                };
                callee_id = *expression;
            }

            if matches!(tree.get(callee_id), Expression::Member { .. }) {
                member_callees.insert(callee_id.id);
            }
        }

        // reify expression nodes
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            if !self.is_node_active(&tree, &symbols, expression_id.into_any()) {
                continue;
            }

            self.reify_expression(
                module_id,
                profile,
                expression_id,
                &mut tree,
                &symbols,
                &mut types,
                &module,
                &member_callees,
            )?;
        }

        // normalize return if expressions introduced during reify
        self.transform_normalize_value_expressions(&mut tree, &symbols, module_id)?;

        // collapse redundant nested casts
        self.normalize_redundant_casts(
            module_id, profile, &mut tree, &symbols, &mut types, &module,
        )?;

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
        member_callees: &HashSet<u32>,
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
                if let IfCondition::Expression { condition } = condition {
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
            }

            Expression::Match { kind, cases, .. } => {
                if kind == MatchKind::Match {
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
                self.reify_resolution(module_id, profile, expression_id, tree, symbols, types)?;
            }

            // unary operators may have resolutions
            Expression::Unary { .. } => {
                self.reify_resolution(module_id, profile, expression_id, tree, symbols, types)?;
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
                self.reify_resolution(module_id, profile, expression_id, tree, symbols, types)?;
            }

            // member access may have resolutions (for union types with different fields)
            Expression::Member { .. } => {
                // skip member reify when used as a call callee
                if member_callees.contains(&expression_id.id) {
                    return Ok(());
                }

                // reify member resolution when applicable
                self.reify_resolution(module_id, profile, expression_id, tree, symbols, types)?;
            }

            // index access may have resolutions (for union types with different Index implementations)
            Expression::Index { .. } => {
                self.reify_resolution(module_id, profile, expression_id, tree, symbols, types)?;
            }

            // type expressions to runtime type descriptors
            // #Incomplete: reify type expressions
            _ => {}
        }

        Ok(())
    }
}
