use tspp_dir as dir;
use tspp_source::Span;

use super::CompletionCollector;
use crate::source::ImportDeclarations;
use crate::{
    CompletionCandidate, CompletionEntry, CompletionInsertion, CompletionItemKind,
    CompletionMember, ConstructorFamily, DeclarationUse, Formatter, QueryError, QueryPosition,
    QueryResult,
};

impl CompletionCandidate {
    /// Select the insertion for one declaration and its use.
    pub(super) fn with_declaration(
        mut self,
        symbol: dir::GlobalSymbolId,
        usage: DeclarationUse,
    ) -> Self {
        match (usage, self.kind) {
            (DeclarationUse::Expression, CompletionItemKind::Struct) => self.with_struct(symbol),
            (DeclarationUse::Expression, CompletionItemKind::Newtype) => {
                self.kind = CompletionItemKind::Constructor;

                self.with_newtype_constructors(symbol)
            }
            (DeclarationUse::Expression, kind) if kind.is_callable() => {
                self.with_symbol(symbol).with_call()
            }
            (DeclarationUse::Constructor, CompletionItemKind::Class) => {
                self.with_class_constructors(symbol)
            }
            _ => self.with_symbol(symbol),
        }
    }
}

impl CompletionCollector<'_, '_, '_> {
    /// Render the call insertion for one callable symbol.
    fn render_call(
        &self,
        mut completion: CompletionCandidate,
        symbol: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        // render a callable from its declared parameter names
        let parameter_names =
            self.program
                .symbol_parameter_names(symbol)?
                .ok_or(QueryError::missing(format!(
                    "completion parameters: {symbol:?}"
                )))?;

        completion.insertion = CompletionInsertion::call(
            &completion.label,
            parameter_names.iter().map(|name| Some(name.as_str())),
        );

        Ok(completion)
    }

    /// Resolve one candidate's declaration and value type.
    fn resolve_symbol(
        &self,
        mut completion: CompletionCandidate,
        symbol: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        // read declaration modifiers
        if self.program.symbol_is_deprecated(symbol)? {
            completion = completion.with_deprecated();
        }

        // read the declaration's value type
        completion.type_id = self.resolve_type(&completion)?;

        Ok(completion)
    }

    /// Resolve the value type needed to rank one candidate.
    pub(super) fn resolve_type(
        &self,
        completion: &CompletionCandidate,
    ) -> QueryResult<Option<dir::GlobalTypeId>> {
        if completion.type_id.is_some() || !completion.kind.has_value_suffix() {
            return Ok(completion.type_id);
        }
        let symbol = completion.symbol().ok_or(QueryError::invalid(
            "completion value has no declaration or type",
        ))?;

        let module = self.program.module(symbol.module_id)?;
        let type_id = module
            .types()?
            .get_symbol_type_id(symbol)
            .ok_or(QueryError::missing(format!(
                "completion symbol type: {symbol:?}"
            )))?;

        Ok(Some(type_id))
    }

    /// Return the target declaration for one completion symbol.
    pub(super) fn symbol_target(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<dir::GlobalSymbolId> {
        self.program
            .symbol_target(symbol_id)?
            .ok_or(QueryError::missing(format!(
                "completion declaration: {symbol_id:?}"
            )))
    }

    /// Expand one ranked candidate into its constructor overloads when required.
    pub(crate) fn expand(
        &self,
        mut completion: CompletionCandidate,
        expanded: &mut Vec<CompletionCandidate>,
    ) -> QueryResult<()> {
        match completion.take_constructors() {
            Some(ConstructorFamily::Class) => {
                let symbol = completion
                    .symbol()
                    .ok_or(QueryError::invalid("class completion has no declaration"))?;

                self.expand_class_constructors(completion, symbol, expanded)?;
            }
            Some(ConstructorFamily::Newtype) => {
                let symbol = completion
                    .symbol()
                    .ok_or(QueryError::invalid("newtype completion has no declaration"))?;

                self.expand_newtype_constructors(completion, symbol, expanded)?;
            }
            None => expanded.push(completion),
        }

        Ok(())
    }

    /// Build one list entry from a ranked completion candidate.
    pub(crate) fn entry(
        &self,
        mut completion: CompletionCandidate,
        position: QueryPosition,
        replacement: Span,
        has_arguments: bool,
        imports: Option<&ImportDeclarations>,
    ) -> QueryResult<Option<CompletionEntry>> {
        // read the selected declaration once
        if let Some(symbol) = completion.symbol() {
            completion = self.resolve_symbol(completion, symbol)?;
        }

        // resolve the selected member completion
        match completion.take_member() {
            Some(CompletionMember::Access { site, key }) => {
                completion = self.resolve_member(completion, site, key)?;
            }
            Some(CompletionMember::ObjectField { site, key }) => {
                completion = self.resolve_object_field(completion, site, key)?;
            }
            None => {}
        }

        // render the insertion selected before ranking
        let insertion = completion.take_insertion();
        match insertion {
            CompletionInsertion::Label => {}
            CompletionInsertion::Call if has_arguments => {}
            CompletionInsertion::Call => {
                let symbol = completion
                    .symbol()
                    .ok_or(QueryError::invalid("call completion has no declaration"))?;
                completion = self.render_call(completion, symbol)?;
            }
            CompletionInsertion::StructExpression => {
                let symbol = completion
                    .symbol()
                    .ok_or(QueryError::invalid("struct completion has no declaration"))?;
                completion = self.resolve_struct(completion, symbol)?;
            }
            CompletionInsertion::ClassConstructor {
                type_id,
                call_symbol,
            } => {
                let symbol = completion.symbol().ok_or(QueryError::invalid(
                    "class constructor completion has no declaration",
                ))?;
                completion =
                    self.resolve_class_constructor(completion, symbol, type_id, call_symbol)?;
            }
            CompletionInsertion::NewtypeConstructor { type_id } => {
                let symbol = completion.symbol().ok_or(QueryError::invalid(
                    "newtype constructor completion has no declaration",
                ))?;
                completion = self.resolve_newtype_constructor(completion, symbol, type_id)?;
            }
            CompletionInsertion::Text(text) => {
                completion = completion.with_insert_text(text);
            }
            CompletionInsertion::Snippet(text) => {
                completion = completion.with_snippet(text);
            }
        }

        // preserve authored call arguments
        if has_arguments {
            completion.insertion = CompletionInsertion::Label;
        }

        // render the suffix shown in the completion list
        if let Some(symbol) = completion.symbol() {
            let module = self.program.module(symbol.module_id)?;
            let formatter = Formatter::new(&module, self.program);
            if completion.label_suffix.is_none() && completion.kind.has_value_suffix() {
                let type_id = completion.type_id.ok_or(QueryError::missing(format!(
                    "completion value type: {symbol:?}"
                )))?;

                // omit suffixes for error types
                let is_error = matches!(
                    self.program
                        .module(type_id.module_id)?
                        .types()?
                        .get_type(type_id.local_id),
                    dir::Type::Error
                );
                if !is_error {
                    let suffix = if completion.kind.is_callable() {
                        let parameter_names = self.program.symbol_parameter_names(symbol)?.ok_or(
                            QueryError::missing(format!("completion parameters: {symbol:?}")),
                        )?;

                        formatter.callable_suffix(type_id, Some(&parameter_names))?
                    } else {
                        format!(": {}", formatter.global_type(type_id)?)
                    };
                    completion = completion.with_label_suffix(suffix);
                }
            } else if completion.label_suffix.is_none()
                && completion.kind.has_generic_suffix()
                && let Some(generics) = formatter.symbol_generics(symbol)?
            {
                completion = completion.with_label_suffix(generics);
            }
        }

        // render contextual value types without declarations
        if completion.kind.has_value_suffix()
            && completion.label_suffix.is_none()
            && let Some(type_id) = completion.type_id
            && !matches!(
                self.program
                    .module(type_id.module_id)?
                    .types()?
                    .get_type(type_id.local_id),
                dir::Type::Error
            )
        {
            let type_text = Formatter::new(self.module, self.program).global_type(type_id)?;
            completion = completion.with_label_suffix(format!(": {type_text}"));
        }

        // build the exact edit for one returned auto import
        let additional_edits = match completion.import() {
            Some(import) => {
                let imports = imports.ok_or(QueryError::invalid(
                    "auto import completion has no import declarations",
                ))?;
                let Some(edit) = imports.edit(&import.binding, &import.specifier) else {
                    return Ok(None);
                };

                vec![edit]
            }
            None => Vec::new(),
        };

        completion
            .into_entry(position, replacement, additional_edits)
            .map(Some)
    }
}
