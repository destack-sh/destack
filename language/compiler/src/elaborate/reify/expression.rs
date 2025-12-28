use destack_dir::{Expression, LocalNodeId, NodeTree, SymbolTable, TypeTable};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::{Compiler, ElaborateResult};

// FUGU #Incomplete: elaborate reify (ranges, trees, types-as-values/comptime, ...)

impl Compiler {
    /// Reify a module to make abstractions concrete:
    /// - Range expressions → iterator construction
    /// - Tree literals → constructor/function calls (`<div>` → `createElement(div, ...)`)
    /// - Operators → resolved method calls (`a + b` → `a.add(b)` based on Resolution)
    /// - Type descriptors → runtime type objects (`Type<T>` → actual descriptor)
    /// - Maybe/Must → explicit error handling (if not overloaded)
    pub(crate) fn elaborate_module_reify(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ElaborateResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let mut tree = dir.tree.write();
        let symbols = dir.symbols.read();
        let types = dir.types.read();

        // reify expressions
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.reify_expression(expression_id, &mut tree, &symbols, &types)?;
        }

        Ok(())
    }

    /// Reify an expression.
    fn reify_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> ElaborateResult<()> {
        let expression = tree.get(expression_id).clone();
        match expression {
            // tree literals → constructor/function calls
            Expression::TreeExpression { .. } => {
                self.reify_tree_expression(expression_id, tree, symbols, types)?;
            }

            // operators → resolved method calls (deload)
            Expression::Binary { .. } => {
                self.reify_operator_expression(expression_id, tree, symbols, types)?;
            }

            // type expressions → runtime type descriptors
            // #Incomplete: reify type expressions?
            _ => {}
        }

        Ok(())
    }

    /// Reify a tree literal into constructor calls.
    fn reify_tree_expression(
        &self,
        _expression_id: LocalNodeId<Expression>,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: reify tree literals
        // <Div>{children}</Div> → createElement(Div, null, children)
        Ok(())
    }

    /// Reify an operator into resolved method calls ("deload").
    /// - Builtin: keep as primitive operator
    /// - Static: emit direct method call `a.add(b)`
    /// - Dynamic: emit type dispatch match expression
    fn reify_operator_expression(
        &self,
        _expression_id: LocalNodeId<Expression>,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: reify operators based on Resolutions
        //  - static: a + b → a.add(b)
        //  - dynamic: a + b → match typeof(..) { T1 => ..., T2 => ... } (conceptually, use if-else)
        Ok(())
    }
}
