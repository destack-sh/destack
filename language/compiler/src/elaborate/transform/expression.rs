use destack_dir::{SymbolTable, Tree, TypeTable};
use destack_workspace::{Module, ProfileId, ProviderContext};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateResult};

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
        context: &dyn ProviderContext,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> ElaborateResult<()> {
        // ensure analysis is complete
        if !self.is_code_module(context.revision(), module.id) {
            return Ok(());
        }

        let options = self.elaborate_options(context, module);
        let mut state = ElaborateState::new(
            context, module.id, module, profile, options, tree, symbols, types,
        );

        // 0. split multi-declarators into individual lets
        if state.options.split_declarators {
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
        if state.options.ternary {
            self.transform_if_to_ternary(&mut state)?;
        }

        // 5. implicit returns → explicit return statements
        if state.options.explicit_return {
            self.transform_explicit_return(&mut state)?;
        }

        // 6. drop parenthesized expressions
        self.transform_drop_parenthesized(&mut state)?;

        // 7. normalize value expressions into statement form
        self.transform_normalize_value_expressions(&mut state)?;

        Ok(())
    }
}
