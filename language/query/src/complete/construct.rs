use tspp_dir as dir;

use super::CompletionCollector;
use crate::{CompletionCandidate, CompletionInsertion, Formatter, QueryError, QueryResult};

impl CompletionCollector<'_, '_, '_> {
    /// Expand one ranked class into its constructor overloads.
    pub(super) fn expand_class_constructors(
        &self,
        completion: CompletionCandidate,
        symbol: dir::GlobalSymbolId,
        expanded: &mut Vec<CompletionCandidate>,
    ) -> QueryResult<()> {
        let module = self.program.module(symbol.module_id)?;
        let members = module.members()?;
        let constructors = members.class_constructors(symbol).ok_or_else(|| {
            QueryError::missing(format!("completion class constructors: {symbol:?}"))
        })?;
        expanded.reserve(constructors.len());

        // retain constructor identity for each declared, forwarded, or default overload
        for constructor in constructors {
            let completion = completion.clone().with_class_constructor(
                symbol,
                constructor.ty,
                constructor.constructor.call_symbol(),
            );
            expanded.push(completion);
        }

        Ok(())
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
                .ok_or_else(|| {
                    QueryError::missing(format!("completion parameters: {constructor_symbol:?}"))
                })?,
            None => Vec::new(),
        };
        let suffix = Formatter::new(&module, self.program)
            .callable_suffix(type_id, Some(&parameter_names))?;
        let mut completion = completion.with_label_suffix(suffix);
        completion.insertion = CompletionInsertion::call(
            &completion.label,
            parameter_names.iter().map(|name| Some(name.as_str())),
        );

        Ok(completion)
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
            .ok_or_else(|| QueryError::missing(format!("completion definition: {symbol:?}")))?;
        let dir::Definition::Struct(definition) = definition else {
            return Err(QueryError::invalid(format!(
                "completion struct definition: {symbol:?}"
            )));
        };
        let formatter = Formatter::new(&module, self.program);
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

            fields.push(formatter.property_key(field.key));
        }

        // render the exact struct expression
        let mut completion = completion;
        completion.insertion = CompletionInsertion::struct_expression(
            &completion.label,
            fields.iter().map(String::as_str),
        );

        Ok(completion)
    }

    /// Expand one ranked newtype into its constructor overloads.
    pub(super) fn expand_newtype_constructors(
        &self,
        completion: CompletionCandidate,
        symbol: dir::GlobalSymbolId,
        expanded: &mut Vec<CompletionCandidate>,
    ) -> QueryResult<()> {
        let module = self.program.module(symbol.module_id)?;
        let definition = module
            .definitions()?
            .definition(symbol)
            .ok_or_else(|| QueryError::missing(format!("completion definition: {symbol:?}")))?;
        let dir::Definition::Newtype(_) = definition else {
            return Err(QueryError::invalid(format!(
                "completion newtype definition: {symbol:?}"
            )));
        };

        let members = module.members()?;
        let constructors = members.newtype_constructors(symbol).unwrap_or_default();
        expanded.reserve(constructors.len());

        // retain constructor identity for each overload
        for constructor in constructors {
            let completion = completion
                .clone()
                .with_newtype_constructor(symbol, constructor.ty);
            expanded.push(completion);
        }

        Ok(())
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
        let mut completion = completion.with_label_suffix(suffix);
        completion.insertion = module.read_signature(
            type_id,
            |function, module| {
                let parameters = module.types()?.parameters(function.parameters);

                Ok(CompletionInsertion::call(
                    &completion.label,
                    parameters.iter().map(|_| None),
                ))
            },
            self.program,
        )?;

        Ok(completion)
    }
}
