use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;

use crate::{
    ModuleQueryContext, NavigationTarget, ProgramQueryContext, QueryError, QueryPosition,
    QueryRange, QueryResult, sort_and_dedup_navigation_targets,
};

/// A goto type definition request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoTypeDefinitionRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// A goto type definition response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoTypeDefinitionResponse {
    /// Type definition targets.
    pub targets: Vec<NavigationTarget>,
}

impl ModuleQueryContext<'_> {
    /// Find the type definition of the symbol at one position.
    pub fn goto_type_definition(
        &self,
        request: GotoTypeDefinitionRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<GotoTypeDefinitionResponse> {
        let position = request.position;
        let cursor = self.cursor(position.file_id, position.offset)?;
        let occurrence = cursor.symbol(program)?;
        let (span, symbols, type_id) = if let Some(occurrence) = occurrence {
            (occurrence.span, occurrence.symbols, occurrence.type_id)
        } else if let Some(occurrence) = cursor.ty()? {
            (occurrence.span, Vec::new(), Some(occurrence.type_id))
        } else {
            return Ok(GotoTypeDefinitionResponse {
                targets: Vec::new(),
            });
        };
        let origin = QueryRange {
            module: self.module(),
            span,
        };
        let mut definitions = Vec::new();

        // resolve nominal declarations and result types for every exact target
        for symbol in symbols {
            for target_symbol in program.symbol_targets(symbol)? {
                let module = program.module(target_symbol.module_id)?;
                let declaration = module.bindings()?.get_symbol(target_symbol.local_id);
                if declaration.kind.is_type_definition() {
                    definitions.push(target_symbol);

                    continue;
                }

                let type_id = module.types()?.get_symbol_type_id(target_symbol).ok_or(
                    QueryError::missing(format!("type definition type: {target_symbol:?}")),
                )?;
                let mut type_symbols = module.type_definition_symbols(program, type_id)?;
                definitions.append(&mut type_symbols);
            }
        }

        // resolve primitive and structural type occurrences without declaration symbols
        if definitions.is_empty()
            && let Some(type_id) = type_id
        {
            definitions.extend(self.type_definition_symbols(program, type_id)?);
        }
        definitions.sort_unstable();
        definitions.dedup();

        // build every exact nominal target reached through the resolved type
        let mut targets = Vec::new();
        for definition in definitions {
            for definition in program.symbol_targets(definition)? {
                let module = program.module(definition.module_id)?;
                let symbol = module.bindings()?.get_symbol(definition.local_id);
                if !symbol.kind.is_type_definition() {
                    return Err(QueryError::invalid(format!(
                        "type definition symbol: {definition:?}"
                    )));
                }

                targets.push(module.navigation_target(program, definition, origin)?);
            }
        }

        sort_and_dedup_navigation_targets(&mut targets);

        Ok(GotoTypeDefinitionResponse { targets })
    }

    /// Return nominal declarations beneath one type.
    fn type_definition_symbols(
        &self,
        program: &ProgramQueryContext<'_>,
        type_id: dir::GlobalTypeId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let (mut symbols, nested) = program.read_type(type_id, |type_value, module| {
            let selected = match type_value {
                dir::Type::Application(application) => (vec![application.symbol], Vec::new()),
                dir::Type::Primitive(primitive) => {
                    let Some(item) = primitive.representation_item() else {
                        return Ok((Vec::new(), Vec::new()));
                    };
                    let symbol = program.environment_bound()?.language.symbol(item).ok_or(
                        QueryError::missing(format!("primitive language item: {item:?}")),
                    )?;

                    (vec![symbol], Vec::new())
                }
                dir::Type::Form(form) => (Vec::new(), vec![form.value]),
                dir::Type::Union(union) => (
                    Vec::new(),
                    module.types()?.type_ids(union.elements).to_vec(),
                ),
                dir::Type::Intersection(intersection) => (
                    Vec::new(),
                    module.types()?.type_ids(intersection.elements).to_vec(),
                ),
                _ => (Vec::new(), Vec::new()),
            };

            Ok(selected)
        })?;

        // descend through transparent type forms
        for nested_type_id in nested {
            symbols.extend(self.type_definition_symbols(program, nested_type_id)?);
        }

        Ok(symbols)
    }
}
