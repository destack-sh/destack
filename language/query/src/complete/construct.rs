use destack_dir as dir;

use super::CompletionBuilder;
use super::call::CallSnippet;
use crate::{
    CompletionCandidate, CompletionItemKind, CompletionOrigin, Formatter, QueryError, QueryResult,
    SORT_LOCAL_SYMBOL,
};

impl CompletionBuilder<'_, '_, '_> {
    /// Build completion candidates for one checked class constructor family.
    pub(super) fn complete_class(
        &self,
        name: &str,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let definition =
            self.module
                .definitions()
                .definition(symbol_id)
                .ok_or(QueryError::missing(format!(
                    "completion definition: {symbol_id:?}"
                )))?;
        let dir::Definition::Class(definition) = definition else {
            return Err(QueryError::invalid(format!(
                "completion definition: {symbol_id:?}"
            )));
        };
        if definition.constructors.is_empty() {
            return Err(QueryError::missing(format!(
                "completion constructor: {symbol_id:?}"
            )));
        }
        let mut completions = Vec::with_capacity(definition.constructors.len());

        // preserve each checked constructor overload
        for constructor in &definition.constructors {
            let parameter_names = match constructor.constructor.call_symbol() {
                Some(constructor_symbol) => self
                    .program
                    .symbol_parameter_names(constructor_symbol)?
                    .ok_or(QueryError::missing(format!(
                        "completion parameters: {constructor_symbol:?}"
                    )))?,
                None => Vec::new(),
            };
            let signature = Formatter::new(self.module, self.program)
                .callable_type(constructor.ty, &parameter_names)?
                .ok_or(QueryError::invalid(format!(
                    "completion type formatting: {:?}",
                    constructor.ty
                )))?;
            let snippet = CallSnippet::new(name, &parameter_names);
            let completion = CompletionCandidate::new(
                name,
                CompletionItemKind::Class,
                CompletionOrigin::Local,
                SORT_LOCAL_SYMBOL,
            )
            .with_detail(format!("new {signature}"))
            .with_insert_text(snippet.text);
            let completion = if snippet.is_snippet {
                completion.with_snippet()
            } else {
                completion
            };
            completions.push(self.attach_symbol_completion(completion, symbol_id)?);
        }

        Ok(completions)
    }

    /// Build one struct literal completion from its required checked fields.
    pub(super) fn complete_struct(
        &self,
        name: &str,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        let definition =
            self.module
                .definitions()
                .definition(symbol_id)
                .ok_or(QueryError::missing(format!(
                    "completion definition: {symbol_id:?}"
                )))?;
        let dir::Definition::Struct(definition) = definition else {
            return Err(QueryError::invalid(format!(
                "completion definition: {symbol_id:?}"
            )));
        };
        let mut fields = Vec::new();

        // retain required instance fields in declaration order
        for member in &definition.members {
            let dir::DefinitionMember::Field(field) = member else {
                continue;
            };
            if field.space != dir::MemberSpace::Instance
                || field.is_optional
                || field.initializer.is_some()
            {
                continue;
            }

            let dir::StaticKey::Name(field_name) = field.key else {
                return Err(QueryError::invalid(format!(
                    "completion field: {:?}",
                    field.symbol
                )));
            };
            fields.push(self.module.strings().get(field_name).to_string());
        }

        let insert_text = if fields.is_empty() {
            format!("{name} {{}}")
        } else {
            let fields = fields
                .iter()
                .enumerate()
                .map(|(index, field)| format!("{field}: ${{{}}}", index + 1))
                .collect::<Vec<_>>()
                .join(", ");

            format!("{name} {{ {fields} }}$0")
        };
        let completion = CompletionCandidate::new(
            name,
            CompletionItemKind::Struct,
            CompletionOrigin::Local,
            SORT_LOCAL_SYMBOL,
        )
        .with_detail(name)
        .with_insert_text(insert_text);
        let completion = if fields.is_empty() {
            completion
        } else {
            completion.with_snippet()
        };

        self.attach_symbol_completion(completion, symbol_id)
    }

    /// Build one nominal backing constructor completion.
    pub(super) fn complete_newtype(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        // FUGU #Incomplete: retain newtype constructor signatures in DIR
        Err(QueryError::missing(format!(
            "newtype constructor signature: {symbol_id:?}"
        )))
    }
}
