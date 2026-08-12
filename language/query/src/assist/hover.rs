use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::{
    Formatter, ModuleQueryContext, ProgramQueryContext, QueryPosition, QueryResult, Target,
};

/// Hover content for one declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HoverItem {
    /// The declaration rendered as Destack source.
    pub declaration: String,
    /// The distinct type selected at the hovered occurrence.
    pub selected_type: Option<String>,
    /// The declaration documentation when available.
    pub documentation: Option<String>,
    /// The exact declaration target.
    pub target: Target,
}

/// Hover payload for a source position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Hover {
    /// The declarations named by the hovered occurrence.
    pub items: Vec<HoverItem>,
    /// Documentation attached to the authored node at the hovered position.
    pub documentation: Option<String>,
    /// The range of the hovered occurrence.
    pub range: Span,
}

/// A hover request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HoverRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// A hover response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HoverResponse {
    /// Hover content, if available.
    pub hover: Option<Hover>,
}

impl ModuleQueryContext<'_> {
    /// Return hover content for the symbol at the given position.
    pub fn hover(
        &self,
        request: HoverRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<HoverResponse> {
        // FUGU #Incomplete: check rejects overload families read as values, see infer_name_expression
        // resolve source documentation and every exact named declaration
        let position = request.position;
        let file_id = position.file_id;
        let offset = position.offset;
        let documentation = self.documentation_at_offset(program, file_id, offset)?;
        let occurrence = self.symbol_at_offset(program, file_id, offset)?;
        if occurrence.is_none() && documentation.is_none() {
            return Ok(HoverResponse { hover: None });
        }

        let mut symbols = Vec::new();
        if let Some(occurrence) = &occurrence {
            for symbol in &occurrence.symbols {
                symbols.extend(program.symbol_targets(*symbol)?);
            }
        }
        symbols.sort();
        symbols.dedup();
        let selected_type = match &occurrence {
            Some(occurrence) => self.format_distinct_type(program, occurrence.type_id, &symbols)?,
            None => None,
        };

        // format only recorded declaration shapes
        let mut items = Vec::new();
        let mut is_documentation_in_items = false;
        for symbol in symbols {
            let module = program.module(symbol.module_id)?;
            let declaration = Formatter::new(&module, program).symbol_signature(symbol)?;
            let item_documentation = program.symbol_documentation(symbol)?;
            let target = module.declaration_target(program, symbol)?;

            // avoid repeating documentation on its declaration occurrence
            if let Some(documentation) = &documentation {
                let declaration = module.bindings()?.get_symbol(symbol.local_id).declaration;
                is_documentation_in_items |= declaration == Some(documentation.node_id);
            }

            items.push(HoverItem {
                declaration,
                selected_type: selected_type.clone(),
                documentation: item_documentation,
                target,
            });
        }
        if items.is_empty() && documentation.is_none() {
            return Ok(HoverResponse { hover: None });
        }

        let range = match (&occurrence, &documentation) {
            (Some(occurrence), _) => occurrence.span,
            (None, Some(documentation)) => documentation.span,
            (None, None) => return Ok(HoverResponse { hover: None }),
        };
        let documentation = documentation
            .filter(|_| !is_documentation_in_items)
            .map(|documentation| documentation.markdown);

        let hover = Hover {
            items,
            documentation,
            range,
        };

        Ok(HoverResponse { hover: Some(hover) })
    }

    /// Format an occurrence type when it differs from every declaration type.
    fn format_distinct_type(
        &self,
        program: &ProgramQueryContext<'_>,
        type_id: Option<dir::GlobalTypeId>,
        symbols: &[dir::GlobalSymbolId],
    ) -> QueryResult<Option<String>> {
        let Some(type_id) = type_id else {
            return Ok(None);
        };

        // retain callable parameter names only when every declaration agrees
        let mut parameter_names = match symbols.first() {
            Some(symbol) => program.symbol_parameter_names(*symbol)?,
            None => None,
        };
        for symbol in symbols.iter().skip(1) {
            let next_names = program.symbol_parameter_names(*symbol)?;
            if next_names != parameter_names {
                parameter_names = None;
                break;
            }
        }

        let formatter = Formatter::new(self, program);
        let text = match parameter_names {
            Some(parameter_names) => formatter.callable_type(type_id, Some(&parameter_names))?,
            None => formatter.global_type(type_id)?,
        };

        // omit a type already represented by a declaration
        for symbol_id in symbols {
            let module = program.module(symbol_id.module_id)?;
            let declared_text = Formatter::new(&module, program).symbol_type(*symbol_id)?;
            if declared_text == text {
                return Ok(None);
            }
        }

        Ok(Some(text))
    }
}
