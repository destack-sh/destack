use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::{
    ModuleQueryContext, NavigationTarget, ProgramQueryContext, QueryError, QueryPosition,
    QueryRange, QueryResult, sort_and_dedup_navigation_targets,
};

/// Request goto type definition at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoTypeDefinitionRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for goto type definition queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoTypeDefinitionResponse {
    /// Type definition targets.
    pub targets: Vec<NavigationTarget>,
}

impl ModuleQueryContext<'_> {
    /// Find the type definition of the symbol at one position.
    pub fn goto_type_definition(
        &self,
        query: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Vec<NavigationTarget>> {
        let occurrence = self.symbol_at_offset(file_id, offset)?;
        let (span, symbols, type_id) = if let Some(occurrence) = occurrence {
            (occurrence.span, occurrence.symbols, occurrence.type_id)
        } else if let Some(occurrence) = self.type_at_offset(file_id, offset)? {
            (occurrence.span, Vec::new(), Some(occurrence.type_id))
        } else {
            return Ok(Vec::new());
        };
        let origin = QueryRange {
            module: self.module(),
            span,
        };
        let mut definitions = Vec::new();

        // resolve nominal declarations and checked result types for every exact target
        for symbol in symbols {
            for canonical in query.canonical_symbols(symbol)? {
                let module = query.module(canonical.module_id)?;
                let declaration = module.symbols().get_symbol(canonical.local_id);
                if declaration.kind.is_type_definition() {
                    definitions.push(canonical);

                    continue;
                }

                let type_id =
                    module
                        .types()
                        .get_symbol_type_id(canonical)
                        .ok_or(QueryError::missing(format!(
                            "type definition type: {canonical:?}"
                        )))?;
                let mut type_symbols = module.type_definition_symbols(query, type_id)?;
                definitions.append(&mut type_symbols);
            }
        }

        // resolve primitive and structural type occurrences without declaration symbols
        if definitions.is_empty()
            && let Some(type_id) = type_id
        {
            definitions.extend(self.type_definition_symbols(query, type_id)?);
        }
        definitions.sort_unstable();
        definitions.dedup();

        // build every exact nominal target reached through the checked type
        let mut targets = Vec::new();
        for definition in definitions {
            for definition in query.canonical_symbols(definition)? {
                let module = query.module(definition.module_id)?;
                let symbol = module.symbols().get_symbol(definition.local_id);
                if !symbol.kind.is_type_definition() {
                    return Err(QueryError::invalid(format!(
                        "type definition symbol: {definition:?}"
                    )));
                }

                targets.push(module.navigation_target(definition, origin)?);
            }
        }

        sort_and_dedup_navigation_targets(&mut targets);

        Ok(targets)
    }

    /// Return nominal declarations beneath checked type forms.
    fn type_definition_symbols(
        &self,
        query: &ProgramQueryContext<'_>,
        type_id: dir::GlobalTypeId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let (mut symbols, nested) = query.read_type(type_id, |type_value, module| {
            let selected = match type_value {
                dir::Type::Reference(reference) => (vec![reference.symbol], Vec::new()),
                dir::Type::Application(application) => (vec![application.symbol], Vec::new()),
                dir::Type::Primitive(primitive) => {
                    let Some(item) = primitive.representation_item() else {
                        return Ok((Vec::new(), Vec::new()));
                    };
                    let symbol = query.global_environment()?.language.symbol(item).ok_or(
                        QueryError::missing(format!("primitive language item: {item:?}")),
                    )?;

                    (vec![symbol], Vec::new())
                }
                dir::Type::Form(form) => (Vec::new(), vec![form.value]),
                dir::Type::Union(union) => {
                    (Vec::new(), module.types().type_ids(union.elements).to_vec())
                }
                dir::Type::Intersection(intersection) => (
                    Vec::new(),
                    module.types().type_ids(intersection.elements).to_vec(),
                ),
                _ => (Vec::new(), Vec::new()),
            };

            Ok(selected)
        })?;

        // descend through transparent checked type forms
        for nested_type_id in nested {
            symbols.extend(self.type_definition_symbols(query, nested_type_id)?);
        }

        Ok(symbols)
    }
}
