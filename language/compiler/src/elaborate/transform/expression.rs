use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

use crate::{Compiler, ElaborateError, ElaborateResult};

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
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> ElaborateResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ElaborateError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;

        // ensure analysis is complete
        self.require_analyze_module(module_id, profile)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let mut tree = dir.tree.write();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();

        // 0. split multi-declarators into individual lets
        if self.options.elaborate_split_declarators {
            self.transform_split_declarators(&mut tree, &symbols)?;
        }

        // 1. unwrap single-expression blocks in SOURCE if/else
        // this must happen BEFORE match transform so match-generated blocks stay
        self.unwrap_single_expression_blocks(&mut tree, &symbols)?;

        // 2. lower if let expressions into match
        self.transform_if_let(&mut tree, &symbols, &types)?;

        // 3. match → decision trees (creates proper blocks)
        self.transform_match(&mut tree, &symbols, &types)?;

        // 4. ternary optimization (only for source if/else that were unwrapped)
        if self.options.elaborate_with_ternary {
            self.transform_if_to_ternary(&mut tree, &symbols)?;
        }

        // 5. implicit returns → explicit return statements
        if self.options.elaborate_explicit_return {
            self.transform_explicit_return(&mut tree, &symbols, &mut types)?;
        }

        // 6. drop parenthesized expressions
        self.transform_drop_parenthesized(&mut tree, &symbols)?;

        // 7. normalize value expressions into statement form
        self.transform_normalize_value_expressions(&mut tree, &symbols, module_id)?;

        Ok(())
    }
}
