use destack_dir as dir;

use crate::ModuleQueryContext;

/// Signature target resolved from a call callee.
#[derive(Debug, Clone)]
pub(crate) struct SignatureTarget {
    /// The name of the target.
    pub name: Option<String>,
    /// The symbol id of the target.
    pub symbol_id: Option<dir::GlobalSymbolId>,
}

impl SignatureTarget {
    /// Create a signature target with the given name and symbol.
    pub(crate) fn new(name: Option<String>, symbol_id: Option<dir::GlobalSymbolId>) -> Self {
        Self { name, symbol_id }
    }
}

impl ModuleQueryContext<'_> {
    /// Return the signature target for one call callee expression.
    pub(crate) fn signature_target(
        &self,
        left_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> SignatureTarget {
        let view = self.view();
        let left_expression = view.get::<dir::Expression>(left_expression_id);

        // inspect the target expression shape
        match left_expression {
            dir::Expression::Identifier { .. } => {
                let symbol = self.expression_symbol_target(left_expression_id);
                let name = symbol.and_then(|symbol| self.symbol_name(symbol));

                // resolve the canonical function symbol
                let function_symbol = symbol.and_then(|symbol| {
                    let canonical_symbol = self.canonical_symbol(symbol);
                    let symbol_kind = self.symbol_kind(canonical_symbol);

                    (symbol_kind == dir::SymbolKind::Function).then_some(canonical_symbol)
                });

                SignatureTarget::new(name, function_symbol)
            }
            dir::Expression::Member { name, .. } => {
                let Some(name) = *name else {
                    return SignatureTarget::new(None, None);
                };

                let member_name = self.strings().get(name).to_string();
                let member_symbol = self.member_access_symbol_target(left_expression_id);

                SignatureTarget::new(Some(member_name), member_symbol)
            }
            _ => SignatureTarget::new(None, None),
        }
    }
}
