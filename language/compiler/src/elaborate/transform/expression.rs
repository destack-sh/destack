use destack_dir::{NodeTree, SymbolTable, TypeTable};
use destack_workspace::{Module, ProfileId};

use crate::elaborate::common::{ElaborateContext, ElaborateState};
use crate::{Compiler, CompilerContext, ElaborateResult};

impl Compiler {
    /// Transform a module with target-independent simplifications:
    /// 0. Split multi-declarator lets into individual lets
    /// 1. Unwrap single-expression blocks in SOURCE if/else (enables ternary)
    /// 2. `if let` → match
    /// 3. `match` → decision trees (if-else chains with proper blocks)
    /// 4. Ternary optimization for simple if/else
    /// 5. Implicit returns → explicit `return` statements
    /// 6. Drop parenthesized expressions
    /// 7. Normalize value expressions into statement form
    pub(crate) fn elaborate_module_transform(
        &self,
        module: &Module,
        profile: ProfileId,
        context: &CompilerContext<'_>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> ElaborateResult<()> {
        // ensure analysis is complete
        if !context.is_code_module(module.id) {
            return Ok(());
        }

        let ctx = ElaborateContext::new(context, module.id, module, profile);
        let mut state = ElaborateState::new(ctx, tree, symbols, types);

        // 0. split multi-declarators into individual lets
        if self.options.elaborate_split_declarators {
            self.transform_split_declarators(&mut state)?;
        }

        // 1. unwrap single-expression blocks in SOURCE if/else
        // this must happen BEFORE match transform so match-generated blocks stay
        self.unwrap_single_expression_blocks(&mut state)?;

        // 2. lower if let expressions into match
        self.transform_if_let(&mut state)?;

        // 3. match → decision trees (creates proper blocks)
        self.transform_match(&mut state)?;

        // 4. ternary optimization (only for source if/else that were unwrapped)
        if self.options.elaborate_with_ternary {
            self.transform_if_to_ternary(&mut state)?;
        }

        // 5. implicit returns → explicit return statements
        if self.options.elaborate_explicit_return {
            self.transform_explicit_return(&mut state)?;
        }

        // 6. drop parenthesized expressions
        self.transform_drop_parenthesized(&mut state)?;

        // 7. normalize value expressions into statement form
        self.transform_normalize_value_expressions(&mut state)?;

        Ok(())
    }
}
