use destack_dir as dir;

use crate::CompilerResult;
use crate::lower::ModuleLowerer;

/// The identity one type alias declares for its value.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::lower) enum AliasForm {
    /// An object type: a named struct behind a managed reference.
    Object,
    /// A compound value family: a named type holding the value content.
    Value,
}

impl ModuleLowerer<'_> {
    /// Return the value one type alias names, none for every other declaration.
    pub(in crate::lower) fn alias_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match self.definition(symbol)? {
            Some(dir::Definition::TypeAlias(_)) => Ok(Some(self.symbol_type(symbol)?)),
            _ => Ok(None),
        }
    }

    /// Return the identity one alias declares, none for transparent renames.
    pub(in crate::lower) fn alias_form(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<AliasForm>> {
        let Some(value) = self.alias_value(symbol)? else {
            return Ok(None);
        };
        Ok(match self.ty(value)? {
            dir::Type::Object(shape) if shape.declares_signatures() => Some(AliasForm::Value),
            dir::Type::Object(_) => Some(AliasForm::Object),
            // give identity to the families whose lowering recurses into children
            other if other.is_structural() => Some(AliasForm::Value),
            _ => None,
        })
    }
}
