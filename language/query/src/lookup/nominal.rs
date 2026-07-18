use destack_dir as dir;

use crate::ModuleQueryContext;

impl ModuleQueryContext<'_> {
    /// Return the recorded symbol target for one member access.
    pub(crate) fn member_access_symbol_target(
        &self,
        expr_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        self.recorded_member_resolution(expr_id)
    }

    /// Return the nominal type symbol named by one type expression.
    pub(crate) fn resolve_nominal_symbol_from_type_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let view = self.view();
        let expression = view.get::<dir::Expression>(expression_id);

        // unwrap type operators to the underlying nominal expression
        match expression {
            dir::Expression::BorrowOf { right, .. }
            | dir::Expression::Maybe { left: right, .. }
            | dir::Expression::Must { left: right, .. } => {
                return self.resolve_nominal_symbol_from_type_expression(*right);
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

        if let Some(target_symbol) = self.expression_symbol_target(expression_id) {
            if self.symbol_can_be_used_as_type(target_symbol) {
                return Some(target_symbol);
            }
        }

        None
    }

    /// Return true when a symbol can be used as a type.
    fn symbol_can_be_used_as_type(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        let symbol_module = self.module_context(symbol_id.module_id);

        let symbols = symbol_module.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);

        symbol.kind.can_be_used_as_type()
    }

    /// Return one unambiguous symbol target from a recorded expression resolution.
    fn recorded_member_resolution(
        &self,
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
                dir::MemberTarget::Existential(candidates)
                | dir::MemberTarget::Universal(candidates) => {
                    if candidates.len() == 1 {
                        let symbol_id = candidates[0].symbol;
                        if !self.symbol_is_visible(symbol_id) {
                            return None;
                        }

                        return Some(symbol_id);
                    }

                    None
                }
                dir::MemberTarget::Field(_)
                | dir::MemberTarget::Element(_)
                | dir::MemberTarget::Index(_) => None,
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
