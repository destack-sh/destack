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
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Vec<NavigationTarget>> {
        let Some(occurrence) = self.symbol_at_offset(file_id, offset)? else {
            return Ok(Vec::new());
        };
        let origin = QueryRange {
            module: self.module(),
            span: occurrence.span,
        };
        let mut targets = Vec::new();

        // resolve nominal declarations and checked result types for every exact target
        for symbol in occurrence.symbols {
            for canonical in program.canonical_symbols(symbol)? {
                let module = program.module(canonical.module_id)?;
                let declaration = module.symbols().get_symbol(canonical.local_id);
                if declaration.kind.is_type_definition() {
                    targets.push(module.navigation_target(canonical, origin)?);

                    continue;
                }

                let type_id =
                    module
                        .types()
                        .get_symbol_type_id(canonical)
                        .ok_or(QueryError::missing(format!(
                            "type definition type: {canonical:?}"
                        )))?;
                let mut type_symbols = module.type_definition_symbols(program, type_id)?;
                type_symbols.sort();
                type_symbols.dedup();

                // build every exact nominal target reached through the checked type
                for type_symbol in type_symbols {
                    for type_symbol in program.canonical_symbols(type_symbol)? {
                        let type_module = program.module(type_symbol.module_id)?;
                        let declaration = type_module.symbols().get_symbol(type_symbol.local_id);
                        if !declaration.kind.is_type_definition() {
                            return Err(QueryError::invalid(format!(
                                "type definition symbol: {type_symbol:?}"
                            )));
                        }

                        targets.push(type_module.navigation_target(type_symbol, origin)?);
                    }
                }
            }
        }

        sort_and_dedup_navigation_targets(&mut targets);

        Ok(targets)
    }

    /// Return nominal declarations beneath checked type forms.
    fn type_definition_symbols(
        &self,
        program: &ProgramQueryContext<'_>,
        type_id: dir::GlobalTypeId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let (mut symbols, nested) = program.read_type(type_id, |type_value, module| {
            let selected = match type_value {
                dir::Type::Reference(reference) => (vec![reference.symbol], Vec::new()),
                dir::Type::Application(application) => (vec![application.symbol], Vec::new()),
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
            symbols.extend(self.type_definition_symbols(program, nested_type_id)?);
        }

        Ok(symbols)
    }
}
