use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, Span};
use serde::{Deserialize, Serialize};

use crate::{
    Formatter, ModuleQueryContext, ProgramQueryContext, QueryPosition, QueryResult, Target,
};

/// Hover content for one declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HoverItem {
    /// The type or signature in code format.
    pub signature: String,
    /// The distinct type selected at the hovered occurrence.
    pub type_text: Option<String>,
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

/// Request hover content at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HoverRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for hover queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HoverResponse {
    /// Hover content, if available.
    pub hover: Option<Hover>,
}

impl ModuleQueryContext<'_> {
    /// Return hover content for the symbol at the given position.
    pub fn hover(
        &self,
        query: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<Hover>> {
        // resolve source documentation and every exact named declaration
        let documentation = self.documentation_at_offset(file_id, offset)?;
        let occurrence = self.symbol_at_offset(query, file_id, offset)?;
        if occurrence.is_none() && documentation.is_none() {
            return Ok(None);
        }

        let mut symbols = Vec::new();
        if let Some(occurrence) = &occurrence {
            for symbol in &occurrence.symbols {
                symbols.extend(query.canonical_symbols(*symbol)?);
            }
        }
        symbols.sort();
        symbols.dedup();
        let type_text = match &occurrence {
            Some(occurrence) => self.format_distinct_type(query, occurrence.type_id, &symbols)?,
            None => None,
        };

        // format only recorded declaration shapes
        let mut items = Vec::new();
        let mut is_documentation_in_items = false;
        for symbol in symbols {
            let module = query.module(symbol.module_id)?;
            let signature = Formatter::new(&module, query).symbol_signature(symbol)?;
            let item_documentation = query.symbol_documentation(symbol)?;
            let target = module.symbol_target(query, symbol)?;

            // avoid repeating documentation on its declaration occurrence
            if let Some(documentation) = &documentation {
                let declaration = module.bindings()?.get_symbol(symbol.local_id).declaration;
                is_documentation_in_items |= declaration == Some(documentation.node_id);
            }

            items.push(HoverItem {
                signature,
                type_text: type_text.clone(),
                documentation: item_documentation,
                target,
            });
        }
        if items.is_empty() && documentation.is_none() {
            return Ok(None);
        }

        let range = match (&occurrence, &documentation) {
            (Some(occurrence), _) => occurrence.span,
            (None, Some(documentation)) => documentation.span,
            (None, None) => return Ok(None),
        };
        let documentation = documentation
            .filter(|_| !is_documentation_in_items)
            .map(|documentation| documentation.text);

        Ok(Some(Hover {
            items,
            documentation,
            range,
        }))
    }

    /// Format an occurrence type when it differs from every declaration type.
    fn format_distinct_type(
        &self,
        query: &ProgramQueryContext<'_>,
        type_id: Option<dir::GlobalTypeId>,
        symbols: &[dir::GlobalSymbolId],
    ) -> QueryResult<Option<String>> {
        let Some(type_id) = type_id else {
            return Ok(None);
        };
        // retain callable parameter names only when every declaration agrees
        let mut parameter_names = match symbols.first() {
            Some(symbol) => query.symbol_parameter_names(*symbol)?,
            None => None,
        };
        for symbol in symbols.iter().skip(1) {
            let next_names = query.symbol_parameter_names(*symbol)?;
            if next_names != parameter_names {
                parameter_names = None;
                break;
            }
        }

        let formatter = Formatter::new(self, query);
        let text = match parameter_names {
            Some(parameter_names) => formatter.callable_type(type_id, Some(&parameter_names))?,
            None => formatter.global_type(type_id)?,
        };

        // omit a type already represented by a declaration
        for symbol_id in symbols {
            let module = query.module(symbol_id.module_id)?;
            let declared_text = Formatter::new(&module, query).symbol_type(*symbol_id)?;
            if declared_text == text {
                return Ok(None);
            }
        }

        Ok(Some(text))
    }
}
