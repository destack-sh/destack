use destack_dir as dir;

use crate::CompilerResult;
use crate::lower::LowerState;

/// The identity one type alias declares for its value.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::lower) enum AliasForm {
    /// An object type: a named struct behind a managed reference.
    Object,
    /// A compound value family: a named type holding the value content.
    Value,
}

impl LowerState<'_> {
    /// Return the identity one alias declares, none for transparent renames.
    pub(in crate::lower) fn alias_form(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<AliasForm>> {
        let Some(dir::Definition::TypeAlias(alias)) = self.definition(symbol)? else {
            return Ok(None);
        };
        Ok(match self.ty(alias.value)? {
            dir::Type::Object(shape) if shape.declares_signatures() => Some(AliasForm::Value),
            dir::Type::Object(_) => Some(AliasForm::Object),
            // give identity to the families whose lowering recurses into children
            dir::Type::Union(_)
            | dir::Type::Tuple(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Function(_) => Some(AliasForm::Value),
            _ => None,
        })
    }
}
