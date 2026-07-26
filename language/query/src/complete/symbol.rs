use destack_dir as dir;

use super::builder::CompletionBuilder;
use crate::format::{format_global_callable_type, format_global_type};
use crate::{CompletionCandidate, CompletionItemKind, QueryResult};

impl CompletionBuilder<'_, '_, '_> {
    /// Format a type detail string for a symbol's declared or inferred type.
    pub(super) fn format_symbol_type_detail(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<String>> {
        let module = self.program.module(symbol_id.module_id)?;
        let Some(type_id) = module.types().get_symbol_type_id(symbol_id) else {
            return Ok(None);
        };

        // retain authored parameter names on callable types
        if let Some(parameter_names) = self.program.symbol_parameter_names(symbol_id)? {
            format_global_callable_type(type_id, &parameter_names, module, self.program)
        } else {
            format_global_type(type_id, module, self.program)
        }
    }

    /// Attach symbol documentation and deprecation to one completion.
    pub(super) fn attach_symbol_completion(
        &self,
        mut completion: CompletionCandidate,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        // attach the exact checked type before following dependency aliases
        if completion.detail.is_none()
            && completion.kind.has_type_detail()
            && let Some(detail) = self.format_symbol_type_detail(symbol_id)?
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
                | Self::Constructor
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
