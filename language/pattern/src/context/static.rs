use tspp_dir as dir;

use crate::{ContextError, ModuleContext, ProgramContext};

impl ModuleContext {
    /// Return one candidate expression's checked static scalar.
    pub fn static_scalar(
        &self,
        node: dir::LocalNodeIdAny,
        program: &ProgramContext,
    ) -> Result<Option<dir::Literal>, ContextError> {
        if node.ty == dir::NodeType::Expression {
            let expression = dir::LocalNodeId::<dir::Expression>::new(node.id);
            if let dir::Expression::Literal(value) = self.view().get(expression) {
                return Ok(Some(*value));
            }
        }

        let symbols = self.symbol_targets(node);
        let symbols = program.symbol_targets(&symbols)?;
        let [symbol] = symbols.as_slice() else {
            return Ok(None);
        };

        program.static_scalar(*symbol)
    }
}

impl ProgramContext {
    /// Return one target symbol's static scalar.
    pub fn static_scalar(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::Literal>, ContextError> {
        let symbols = self.symbol_targets(&[symbol])?;
        let [symbol] = symbols.as_slice() else {
            return Ok(None);
        };
        let module = self.module(symbol.module_id)?;
        if let Some(static_id) = module.statics().get_symbol_static_id(*symbol) {
            let static_module = self.module(static_id.module_id)?;
            let value = static_module.statics().get_static_maybe(static_id.local_id);

            return Ok(value.and_then(dir::StaticTerm::as_scalar));
        }

        // checked singleton symbol types retain literal values without a static table entry
        let Some(type_id) = module.types().get_symbol_type_id(*symbol) else {
            return Ok(None);
        };
        let scalar = match self.type_by_id(type_id)? {
            dir::Type::Literal(value) => Some(value),
            dir::Type::Null => Some(dir::Literal::Null),
            dir::Type::Undefined => Some(dir::Literal::Undefined),
            _ => None,
        };

        Ok(scalar)
    }
}
