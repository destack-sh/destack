use destack_dir as dir;

use super::builder::CompletionBuilder;
use super::call::CallSnippet;
use crate::{CompletionCandidate, CompletionItemKind, Formatter, QueryError, QueryResult};

impl CompletionBuilder<'_, '_, '_> {
    /// Attach a call snippet with the declaration's authored parameter names.
    pub(super) fn attach_call_snippet(
        &self,
        mut completion: CompletionCandidate,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        if !matches!(
            completion.kind,
            CompletionItemKind::Function | CompletionItemKind::Method
        ) {
            return Ok(completion);
        }
        let Some(symbol_id) = self.program.canonical_symbol(symbol_id)? else {
            return Err(QueryError::missing(format!(
                "completion declaration: {symbol_id:?}"
            )));
        };
        let parameter_names =
            self.program
                .symbol_parameter_names(symbol_id)?
                .ok_or(QueryError::missing(format!(
                    "completion parameters: {symbol_id:?}"
                )))?;

        let snippet = CallSnippet::named(&completion.label, &parameter_names);
        completion = completion.with_insert_text(snippet.text);
        if snippet.is_snippet {
            completion = completion.with_snippet();
        }

        Ok(completion)
    }

    /// Attach type, documentation, and deprecation from one declaration.
    pub(super) fn attach_symbol_description(
        &self,
        mut completion: CompletionCandidate,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        let Some(symbol_id) = self.program.canonical_symbol(symbol_id)? else {
            return Err(QueryError::missing(format!(
                "completion declaration: {symbol_id:?}"
            )));
        };

        // render the declaration's value type
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

            // omit detail for an explicit error type
            if completion.detail.is_none() && !is_error_type {
                let detail = Formatter::new(self.module, self.program).symbol_type(symbol_id)?;
                completion = completion.with_detail(detail);
            }
            completion = completion.with_type_id(type_id);
        }

        // mark deprecated declarations
        if self.program.symbol_is_deprecated(symbol_id)? {
            completion = completion.with_deprecated();
        }

        // include declaration documentation
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
