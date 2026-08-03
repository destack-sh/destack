use destack_dir as dir;

use super::builder::CompletionBuilder;
use crate::{CompletionCandidate, CompletionItemKind, Formatter, QueryError, QueryResult};

impl CompletionBuilder<'_, '_, '_> {
    /// Attach symbol documentation and deprecation to one completion.
    pub(super) fn attach_symbol_completion(
        &self,
        mut completion: CompletionCandidate,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        // attach the exact checked type before following dependency aliases
        if completion.kind.has_type_detail() {
            let module = self.program.module(symbol_id.module_id)?;
            let type_id =
                module
                    .types()?
                    .get_symbol_type_id(symbol_id)
                    .ok_or(QueryError::missing(format!(
                        "completion symbol type: {symbol_id:?}"
                    )))?;
            let is_error_type = self
                .program
                .read_type(type_id, |ty, _| Ok(matches!(ty, dir::Type::Error)))?;

            // leave optional detail absent for an explicit checked error type
            if completion.detail.is_none() && !is_error_type {
                let detail = Formatter::new(self.module, self.program).symbol_type(symbol_id)?;
                completion = completion.with_detail(detail);
            }
            completion = completion.with_type_id(type_id);
        }

        let Some(symbol_id) = self.program.canonical_symbol(symbol_id)? else {
            return Err(QueryError::missing(format!(
                "completion declaration: {symbol_id:?}"
            )));
        };

        // transcribe exact decorator state
        if self.program.symbol_is_deprecated(symbol_id)? {
            completion = completion.with_deprecated();
        }

        // transcribe exact declaration documentation
        if let Some(documentation) = self.program.symbol_documentation(symbol_id)? {
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
