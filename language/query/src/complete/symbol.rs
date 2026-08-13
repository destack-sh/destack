use destack_dir as dir;

use super::CompletionCollector;
use super::call::CallSnippet;
use crate::{CompletionCandidate, CompletionResolution, Formatter, QueryError, QueryResult};

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

        let snippet = CallSnippet::named(&completion.label, &parameter_names);
        completion = if snippet.is_snippet {
            completion.with_snippet(snippet.text)
        } else {
            completion.with_insert_text(snippet.text)
        };

        Ok(completion)
    }

    /// Collect one symbol candidate and its ranking fields.
    pub(super) fn collect_symbol(
        &self,
        completion: CompletionCandidate,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        let mut completion = self.collect_declaration(completion, symbol_id)?;
        let symbol_id = completion.symbol().ok_or(QueryError::invalid(
            "completion candidate has no declaration",
        ))?;

        // read the declaration's value type
        if completion.kind.has_value_suffix() {
            let module = self.program.module(symbol_id.module_id)?;
            let type_id =
                module
                    .types()?
                    .get_symbol_type_id(symbol_id)
                    .ok_or(QueryError::missing(format!(
                        "completion symbol type: {symbol_id:?}"
                    )))?;

            completion = completion.with_type_id(type_id);
        }

        Ok(completion)
    }

    /// Collect the target declaration and modifiers behind one candidate.
    pub(super) fn collect_declaration(
        &self,
        mut completion: CompletionCandidate,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        let symbol_id = self.symbol_target(symbol_id)?;

        // mark deprecated declarations
        if self.program.symbol_is_deprecated(symbol_id)? {
            completion = completion.with_deprecated();
        }

        Ok(completion.with_symbol(symbol_id))
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

    /// Resolve one ranked completion candidate.
    pub(crate) fn resolve(
        &self,
        mut completion: CompletionCandidate,
    ) -> QueryResult<CompletionCandidate> {
        // perform specialized work only after ranking
        match completion.take_resolution() {
            CompletionResolution::Member { site, key } => {
                completion = self.resolve_member(completion, site, key)?;
            }
            CompletionResolution::ObjectField { site, key } => {
                completion = self.resolve_object_field(completion, site, key)?;
            }
            CompletionResolution::Struct => {
                let symbol = completion
                    .symbol()
                    .ok_or(QueryError::invalid("struct completion has no declaration"))?;
                completion = self.resolve_struct(completion, symbol)?;
            }
            CompletionResolution::ClassConstructor {
                type_id,
                call_symbol,
            } => {
                let symbol = completion.symbol().ok_or(QueryError::invalid(
                    "class constructor completion has no declaration",
                ))?;
                completion =
                    self.resolve_class_constructor(completion, symbol, type_id, call_symbol)?;
            }
            CompletionResolution::NewtypeConstructor { type_id } => {
                let symbol = completion.symbol().ok_or(QueryError::invalid(
                    "newtype constructor completion has no declaration",
                ))?;
                completion = self.resolve_newtype_constructor(completion, symbol, type_id)?;
            }
            CompletionResolution::AutoImport { binding, specifier } => {
                let edits = self
                    .module
                    .build_import_edits(self.file_id, &binding, &specifier)?;
                if edits.is_empty() {
                    return Err(QueryError::invalid(format!(
                        "auto import produces no edit: {specifier}, {binding:?}"
                    )));
                }
                completion = completion.with_additional_edits(edits);
            }
            CompletionResolution::None => {}
        }

        // render callable insertion text
        if completion.is_call() {
            let symbol = completion
                .symbol()
                .ok_or(QueryError::invalid("call completion has no declaration"))?;
            completion = self.render_call(completion, symbol)?;
        }

        // render declaration text, label suffix, and documentation
        if let Some(symbol) = completion.symbol() {
            let module = self.program.module(symbol.module_id)?;
            let formatter = Formatter::new(&module, self.program);
            let declaration = formatter.symbol_signature(symbol)?;
            completion = completion.with_declaration(declaration);

            if completion.label_suffix.is_none() && completion.kind.has_value_suffix() {
                let type_id = completion.type_id.ok_or(QueryError::missing(format!(
                    "completion value type: {symbol:?}"
                )))?;

                // skip poisoned value types
                let is_poisoned = matches!(
                    self.program
                        .module(type_id.module_id)?
                        .types()?
                        .get_type(type_id.local_id),
                    destack_dir::Type::Error
                );
                if !is_poisoned {
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

            if let Some(documentation) = self.program.symbol_documentation(symbol)? {
                completion = completion.with_documentation(documentation);
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
                destack_dir::Type::Error
            )
        {
            let type_text = Formatter::new(self.module, self.program).global_type(type_id)?;
            completion = completion.with_label_suffix(format!(": {type_text}"));
        }

        Ok(completion)
    }
}
