use destack_dir as dir;

use super::builder::CompletionBuilder;
use super::call::CallSnippet;
use crate::{CompletionCandidate, CompletionItemKind, Formatter, QueryError, QueryResult};

impl CompletionBuilder<'_, '_, '_> {
    /// Render the call insertion for one callable symbol.
    fn render_call(
        &self,
        mut completion: CompletionCandidate,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        let Some(symbol_id) = self.program.canonical_symbol(symbol_id)? else {
            return Err(QueryError::missing(format!(
                "completion declaration: {symbol_id:?}"
            )));
        };

        // render a derived tagged constructor from its object argument
        let module = self.program.module(symbol_id.module_id)?;
        if let Some((_, _, dir::DefinitionMember::TaggedVariant(variant))) =
            module.definitions()?.member(symbol_id)
        {
            let fields = match variant.argument {
                Some(argument) => self.program.read_type(argument, |ty, owner| {
                    let (dir::Type::Shape(shape) | dir::Type::Object(shape)) = ty else {
                        return Err(QueryError::invalid(format!(
                            "tagged constructor argument: {argument:?}"
                        )));
                    };
                    let formatter = Formatter::new(owner, self.program);
                    owner
                        .types()?
                        .properties(shape.properties)
                        .iter()
                        .filter(|field| !field.is_optional)
                        .map(|field| formatter.property_key(field.key))
                        .collect::<QueryResult<Vec<_>>>()
                })?,
                None => Vec::new(),
            };
            let snippet = CallSnippet::object(&completion.label, &fields);
            completion = if snippet.is_snippet {
                completion.with_snippet(snippet.text)
            } else {
                completion.with_insert_text(snippet.text)
            };

            return Ok(completion);
        }

        // render a callable from its declared parameter names
        let parameter_names =
            self.program
                .symbol_parameter_names(symbol_id)?
                .ok_or(QueryError::missing(format!(
                    "completion parameters: {symbol_id:?}"
                )))?;

        let snippet = CallSnippet::named(&completion.label, &parameter_names);
        completion = if snippet.is_snippet {
            completion.with_snippet(snippet.text)
        } else {
            completion.with_insert_text(snippet.text)
        };

        Ok(completion)
    }

    /// Resolve one candidate declaration and its ranking metadata.
    pub(super) fn resolve_symbol(
        &self,
        completion: CompletionCandidate,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        let mut completion = self.resolve_declaration(completion, symbol_id)?;
        let symbol_id = completion.symbol.ok_or(QueryError::invalid(
            "resolved completion has no declaration",
        ))?;

        // read the declaration's value type
        if completion.kind.has_type_detail() {
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

    /// Resolve the declaration and modifiers behind one candidate.
    pub(super) fn resolve_declaration(
        &self,
        mut completion: CompletionCandidate,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<CompletionCandidate> {
        let Some(symbol_id) = self.program.canonical_symbol(symbol_id)? else {
            return Err(QueryError::missing(format!(
                "completion declaration: {symbol_id:?}"
            )));
        };

        // mark deprecated declarations
        if self.program.symbol_is_deprecated(symbol_id)? {
            completion = completion.with_deprecated();
        }

        Ok(completion.with_symbol(symbol_id))
    }

    /// Render one filtered completion candidate.
    pub(crate) fn describe(
        &self,
        mut completion: CompletionCandidate,
    ) -> QueryResult<CompletionCandidate> {
        // render callable insertion text
        if completion.is_call() {
            let symbol = completion
                .symbol
                .ok_or(QueryError::invalid("call completion has no declaration"))?;
            completion = self.render_call(completion, symbol)?;
        }

        // render candidate type detail
        if completion.kind.has_type_detail()
            && completion.detail.is_none()
            && let Some(type_id) = completion.type_id
        {
            let detail = Formatter::new(self.module, self.program).global_type(type_id)?;
            completion = completion.with_detail(detail);
        }

        // render declaration text and documentation
        if let Some(symbol) = completion.symbol {
            if completion.kind.has_type_detail() && completion.detail.is_none() {
                let detail = Formatter::new(self.module, self.program).symbol_type(symbol)?;
                completion = completion.with_detail(detail);
            }
            if let Some(documentation) = self.program.symbol_documentation(symbol)? {
                completion = completion.with_documentation(documentation);
            }
        }

        Ok(completion)
    }
}

impl CompletionItemKind {
    /// Return whether this editor item includes its type as detail.
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
