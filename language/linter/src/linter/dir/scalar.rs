use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return the exact scalar constant selected by one checked expression.
    pub fn scalar_constant(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::ScalarLiteral>, ProviderError> {
        // select one symbol-backed constant expression
        if let Some(symbol) = self.symbol(node)? {
            let module = self.dir.module_storage(symbol.module_id)?;
            if let Some(static_id) = module.statics.get_symbol_static_id(symbol) {
                let value = match self.dir.get_static(static_id)? {
                    dir::StaticTerm::ScalarLiteral { value } => Some(*value),
                    _ => None,
                };
                if value.is_some() {
                    return Ok(value);
                }
            }
        }

        // read exact literal expression types directly
        let value = match self.node_type(node.into_any())? {
            dir::Type::Literal(literal) => Some(literal),
            _ => None,
        };

        Ok(value)
    }

    /// Return whether one expression is an exact negative-zero constant.
    pub fn is_negative_zero(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        let is_negative_zero = self
            .scalar_constant(node)?
            .is_some_and(|value| value.is_negative_zero());

        Ok(is_negative_zero)
    }

    /// Return whether one checked expression denotes NaN.
    pub fn is_nan(&self, node: dir::LocalNodeId<dir::Expression>) -> Result<bool, ProviderError> {
        // recognize exact checked scalar constants
        if self
            .scalar_constant(node)?
            .is_some_and(|value| value.is_nan())
        {
            return Ok(true);
        }

        // recognize the canonical standard-library constants by selected symbol
        let item = self
            .symbol(node)?
            .and_then(|symbol| self.dir.environment.language.item(symbol));
        let is_nan = matches!(
            item,
            Some(dir::LanguageItem::NaN | dir::LanguageItem::NumberNaN)
        );

        Ok(is_nan)
    }
}
