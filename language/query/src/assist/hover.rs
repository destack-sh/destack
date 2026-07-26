use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, Span};
use serde::{Deserialize, Serialize};

use crate::format::format_global_type;
use crate::{
    ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult, Target,
    format_symbol_signature,
};

/// Hover content for one declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HoverItem {
    /// The type or signature in code format.
    pub signature: String,
    /// The distinct checked type selected at the hovered occurrence.
    pub type_text: Option<String>,
    /// The checked documentation when available.
    pub documentation: Option<String>,
    /// The exact declaration target.
    pub target: Target,
}

/// Hover payload for a source position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Hover {
    /// The declarations named by the hovered occurrence.
    pub items: Vec<HoverItem>,
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
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<Hover>> {
        // resolve every exact declaration named by this occurrence
        let Some(occurrence) = self.symbol_at_offset(file_id, offset)? else {
            return Ok(None);
        };
        let selected_call = self.call_item(program, file_id, offset)?;
        let mut symbols = Vec::new();
        for symbol in &occurrence.symbols {
            symbols.extend(program.canonical_symbols(*symbol)?);
        }
        symbols.sort();
        symbols.dedup();
        let type_text = self.hover_occurrence_type(program, occurrence.type_id, &symbols)?;

        // format only recorded declaration shapes
        let mut items = Vec::new();
        for symbol in symbols {
            let module = program.module(symbol.module_id)?;
            let member = program
                .module_index(symbol.module_id)?
                .members
                .symbol_entry(symbol);
            let call_detail = selected_call
                .as_ref()
                .filter(|item| item.symbol_id == symbol)
                .and_then(|item| item.detail.as_deref());
            let signature = module
                .hover_symbol_signature(program, symbol, member, call_detail)?
                .ok_or(QueryError::missing(format!("hover signature: {symbol:?}")))?;
            let documentation = program.symbol_doc_text(symbol)?;
            let target = module.symbol_target(symbol)?;

            items.push(HoverItem {
                signature,
                type_text: type_text.clone(),
                documentation,
                target,
            });
        }
        if items.is_empty() {
            return Ok(None);
        }

        Ok(Some(Hover {
            items,
            range: occurrence.span,
        }))
    }

    /// Format a checked occurrence type when it differs from every declaration type.
    fn hover_occurrence_type(
        &self,
        program: &ProgramQueryContext<'_>,
        type_id: Option<dir::GlobalTypeId>,
        symbols: &[dir::GlobalSymbolId],
    ) -> QueryResult<Option<String>> {
        let Some(type_id) = type_id else {
            return Ok(None);
        };
        let text = format_global_type(type_id, self, program)?.ok_or(QueryError::invalid(
            format!("hover type formatting: {type_id:?}"),
        ))?;

        // omit a checked type already represented by a declaration type
        for symbol_id in symbols {
            let module = program.module(symbol_id.module_id)?;
            let Some(declared_type_id) = module.types().get_symbol_type_id(*symbol_id) else {
                continue;
            };
            let declared_text = format_global_type(declared_type_id, module, program)?.ok_or(
                QueryError::invalid(format!("hover type formatting: {declared_type_id:?}")),
            )?;
            if declared_text == text {
                return Ok(None);
            }
        }

        Ok(Some(text))
    }

    /// Format one symbol from its recorded declaration.
    fn hover_symbol_signature(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        member: Option<&dir::MemberEntry>,
        call_detail: Option<&str>,
    ) -> QueryResult<Option<String>> {
        if let Some(member) = member {
            return self.member_hover(program, member, call_detail);
        }

        let symbols = self.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let name = symbol
            .name()
            .map(|name| self.strings().get(name).to_string());
        let Some(declaration) = symbol.declaration else {
            return Ok(None);
        };

        match declaration.local_id.ty {
            dir::NodeType::Declaration => format_symbol_signature(program, symbol_id),
            dir::NodeType::Parameter => {
                let parameter_id = dir::LocalNodeId::<dir::Parameter>::new(declaration.local_id.id);

                self.parameter_hover(program, parameter_id)
            }
            dir::NodeType::Pattern => {
                let Some(name) = name.as_deref() else {
                    return Ok(None);
                };

                self.local_variable_hover(program, name, symbol_id)
            }
            dir::NodeType::Member | dir::NodeType::TypeMember | dir::NodeType::EnumField => Err(
                QueryError::missing(format!("hover signature: {symbol_id:?}")),
            ),
            _ => Ok(None),
        }
    }
}
