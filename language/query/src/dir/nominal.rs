use destack_dir as dir;

use crate::core::{DirQueryContext, ModuleQueryContext, NominalEntry};

impl ModuleQueryContext<'_> {
    /// Build nominal index entries for this module.
    pub(crate) fn build_nominal_relations(&self) -> Vec<NominalEntry> {
        Vec::new()
    }
}

impl DirQueryContext<'_> {
    /// Return the recorded symbol target for one member access.
    pub(crate) fn member_access_symbol_target(
        self,
        expr_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        self.recorded_member_resolution(expr_id)
    }

    /// Return the nominal type symbol named by one type expression.
    pub(crate) fn resolve_nominal_symbol_from_type_expression(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let view = self.view();
        let expression = view.get::<dir::Expression>(expression_id);

        // unwrap type operators and wrappers to the underlying nominal expression
        match expression {
            dir::Expression::BorrowOf { right, .. }
            | dir::Expression::MoveOf { right, .. }
            | dir::Expression::Maybe { left: right, .. }
            | dir::Expression::Must { left: right, .. } => {
                return self.resolve_nominal_symbol_from_type_expression(*right);
            }
            dir::Expression::Parenthesized { expression } => {
                return self.resolve_nominal_symbol_from_type_expression(*expression);
            }
            dir::Expression::Instantiation { left, .. } => {
                return self.resolve_nominal_symbol_from_type_expression(*left);
            }
            dir::Expression::Member { .. } => {
                if let Some(symbol_id) = self.member_access_symbol_target(expression_id) {
                    return Some(symbol_id);
                }
            }
            _ => {}
        }

        if let Some(target_symbol) = self.expression_symbol_target(expression_id)
            && self.symbol_is_visible_in_type_space(target_symbol)
        {
            return Some(target_symbol);
        }

        None
    }

    /// Check whether a symbol is visible in type space.
    fn symbol_is_visible_in_type_space(self, symbol_id: dir::GlobalSymbolId) -> bool {
        let _ctx = self;
        let Some(ctx) = self.module_context(symbol_id.module_id) else {
            return false;
        };

        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);

        symbol.kind.is_visible_in(dir::SymbolSpace::Type)
    }

    /// Return one unambiguous symbol target from a recorded expression resolution.
    fn recorded_member_resolution(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let node_id = dir::GlobalNodeIdAny {
            module_id: self.module_id(),
            local_id: expression_id.into(),
        };
        if let Some(resolution) = self.resolutions().member_resolution(node_id) {
            return match &resolution.target {
                dir::MemberTarget::Symbol(candidate) => {
                    if !self.symbol_is_visible(candidate.symbol) {
                        return None;
                    }

                    Some(candidate.symbol)
                }
                dir::MemberTarget::Union(candidates) => {
                    if candidates.len() == 1 {
                        let symbol_id = candidates[0].symbol;
                        if !self.symbol_is_visible(symbol_id) {
                            return None;
                        }

                        return Some(symbol_id);
                    }

                    None
                }
                dir::MemberTarget::Builtin(_) | dir::MemberTarget::Field(_) => None,
            };
        }

        let resolution = self.resolutions().name_resolution(node_id)?;
        let symbol = resolution.symbol();
        if !self.symbol_is_visible(symbol) {
            return None;
        }

        Some(symbol)
    }
}
