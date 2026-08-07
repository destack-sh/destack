use destack_dir as dir;

use super::CompletionCollector;
use super::call::CallSnippet;
use crate::{
    CompletionCandidate, CompletionItemKind, CompletionOrigin, Formatter, QueryError, QueryResult,
};

impl CompletionCollector<'_, '_, '_> {
    /// Collect one candidate per class constructor overload.
    pub(super) fn collect_class(
        &self,
        name: &str,
        symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let symbol = self.canonical_symbol(symbol)?;
        let module = self.program.module(symbol.module_id)?;
        let definition = module
            .definitions()?
            .definition(symbol)
            .ok_or(QueryError::missing(format!(
                "completion definition: {symbol:?}"
            )))?;
        let dir::Definition::Class(definition) = definition else {
            return Err(QueryError::invalid(format!(
                "completion class definition: {symbol:?}"
            )));
        };
        if definition.constructors.is_empty() {
            return Err(QueryError::missing(format!(
                "completion class constructor: {symbol:?}"
            )));
        }
        let mut completions = Vec::with_capacity(definition.constructors.len());

        // retain constructor identity for each overload
        for constructor in &definition.constructors {
            let completion =
                CompletionCandidate::new(name, CompletionItemKind::Class, CompletionOrigin::Local)
                    .with_class_constructor(
                        symbol,
                        constructor.ty,
                        constructor.constructor.call_symbol(),
                    );
            completions.push(self.collect_symbol(completion, symbol)?);
        }

        Ok(completions)
    }

    /// Resolve one class constructor overload.
    pub(super) fn resolve_class_constructor(
        &self,
        completion: CompletionCandidate,
        symbol: dir::GlobalSymbolId,
        type_id: dir::GlobalTypeId,
        call_symbol: Option<dir::GlobalSymbolId>,
    ) -> QueryResult<CompletionCandidate> {
        let module = self.program.module(symbol.module_id)?;

        // render the selected constructor signature and insertion
        let parameter_names = match call_symbol {
            Some(constructor_symbol) => self
                .program
                .symbol_parameter_names(constructor_symbol)?
                .ok_or(QueryError::missing(format!(
                    "completion parameters: {constructor_symbol:?}"
                )))?,
            None => Vec::new(),
        };
        let suffix = Formatter::new(&module, self.program)
            .callable_suffix(type_id, Some(&parameter_names))?;
        let snippet = CallSnippet::named(&completion.label, &parameter_names);
        let completion = completion.with_label_suffix(suffix);

        if snippet.is_snippet {
            Ok(completion.with_snippet(snippet.text))
        } else {
            Ok(completion.with_insert_text(snippet.text))
        }
    }

    /// Collect one struct expression candidate.
    pub(super) fn collect_struct(
        &self,
        name: &str,
        symbol: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        let symbol = self.canonical_symbol(symbol)?;
        let completion =
            CompletionCandidate::new(name, CompletionItemKind::Struct, CompletionOrigin::Local)
                .with_struct(symbol);

        self.collect_symbol(completion, symbol)
    }

    /// Resolve one struct expression candidate.
    pub(super) fn resolve_struct(
        &self,
        completion: CompletionCandidate,
        symbol: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        let module = self.program.module(symbol.module_id)?;
        let definition = module
            .definitions()?
            .definition(symbol)
            .ok_or(QueryError::missing(format!(
                "completion definition: {symbol:?}"
            )))?;
        let dir::Definition::Struct(definition) = definition else {
            return Err(QueryError::invalid(format!(
                "completion struct definition: {symbol:?}"
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
                    "completion struct field: {:?}",
                    field.symbol
                )));
            };
            fields.push(module.strings().get(field_name).to_string());
        }

        // render the exact struct expression
        let insert_text = if fields.is_empty() {
            format!("{} {{}}", completion.label)
        } else {
            let fields = fields
                .iter()
                .enumerate()
                .map(|(index, field)| format!("{field}: ${{{}}}", index + 1))
                .collect::<Vec<_>>()
                .join(", ");

            format!("{} {{ {fields} }}$0", completion.label)
        };
        if fields.is_empty() {
            Ok(completion.with_insert_text(insert_text))
        } else {
            Ok(completion.with_snippet(insert_text))
        }
    }

    /// Collect one candidate per newtype constructor overload.
    pub(super) fn collect_newtype(
        &self,
        name: &str,
        symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let symbol = self.canonical_symbol(symbol)?;
        let module = self.program.module(symbol.module_id)?;
        let definition = module
            .definitions()?
            .definition(symbol)
            .ok_or(QueryError::missing(format!(
                "completion definition: {symbol:?}"
            )))?;
        let dir::Definition::Newtype(definition) = definition else {
            return Err(QueryError::invalid(format!(
                "completion newtype definition: {symbol:?}"
            )));
        };
        let mut completions = Vec::with_capacity(definition.constructors.len());

        // retain constructor identity for each overload
        for constructor in &definition.constructors {
            let completion = CompletionCandidate::new(
                name,
                CompletionItemKind::Constructor,
                CompletionOrigin::Local,
            )
            .with_newtype_constructor(symbol, constructor.ty);
            completions.push(self.collect_symbol(completion, symbol)?);
        }

        Ok(completions)
    }

    /// Resolve one newtype constructor overload.
    pub(super) fn resolve_newtype_constructor(
        &self,
        completion: CompletionCandidate,
        symbol: dir::GlobalSymbolId,
        type_id: dir::GlobalTypeId,
    ) -> QueryResult<CompletionCandidate> {
        let module = self.program.module(symbol.module_id)?;

        // render the selected constructor signature and insertion
        let suffix = Formatter::new(&module, self.program).callable_suffix(type_id, None)?;
        let snippet = CallSnippet::positional(&completion.label, type_id, self.program)?;
        let completion = completion.with_label_suffix(suffix);

        if snippet.is_snippet {
            Ok(completion.with_snippet(snippet.text))
        } else {
            Ok(completion.with_insert_text(snippet.text))
        }
    }
}
