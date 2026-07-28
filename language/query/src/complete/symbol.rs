use destack_dir as dir;

use super::builder::CompletionBuilder;
use crate::{CompletionCandidate, CompletionItemKind, Formatter, QueryResult};

impl CompletionBuilder<'_, '_, '_> {
    /// Attach symbol documentation and deprecation to one completion.
    pub(super) fn attach_symbol_completion(
        &self,
        mut completion: CompletionCandidate,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        // attach the exact checked type before following dependency aliases
        if completion.detail.is_none()
            && completion.kind.has_type_detail()
            && let Some(detail) =
                Formatter::new(self.module, self.program).symbol_type(symbol_id)?
        {
            completion = completion.with_detail(detail);
        }
        if let Some(type_id) = self
            .program
            .module(symbol_id.module_id)?
            .types()
            .get_symbol_type_id(symbol_id)
        {
            completion = completion.with_type_id(type_id);
        }

        let Some(symbol_id) = self.program.canonical_symbol(symbol_id)? else {
            return Ok(completion);
        };

        // transcribe exact decorator state
        if self.program.symbol_is_deprecated(symbol_id)? {
            completion = completion.with_deprecated();
        }

        // transcribe exact declaration documentation
        if let Some(documentation) = self.program.symbol_doc_text(symbol_id)? {
            completion = completion.with_documentation(documentation);
        }

        Ok(completion)
    }
}

impl CompletionItemKind {
    /// Return whether this editor item benefits from its checked type as detail.
    fn has_type_detail(self) -> bool {
        matches!(
            self,
            Self::AssociatedConst
                | Self::Constant
                | Self::EnumMember
                | Self::Field
                | Self::Function
                | Self::Method
                | Self::Property
                | Self::Value
                | Self::ValueParameter
                | Self::Variable
        )
    }
}
