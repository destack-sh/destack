use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::{
    Formatter, ModuleQueryContext, QueryContext, QueryError, QueryPosition, QueryResult,
    SymbolKind, Target,
};

/// An item in the type hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeItem {
    /// The name of the type.
    pub name: String,
    /// The kind of type.
    pub kind: SymbolKind,
    /// The rendered generic parameters.
    pub detail: Option<String>,
    /// The target source.
    pub target: Target,
    /// The resolved type symbol.
    pub symbol_id: dir::GlobalSymbolId,
}

/// Request the type item at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeItemRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for type item queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeItemResponse {
    /// Type hierarchy item, if available.
    pub item: Option<TypeItem>,
}

/// Stable protocol ordering for one type item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TypeItemOrder<'a> {
    /// The source file id.
    file: u64,
    /// The item start offset.
    start: u32,
    /// The item end offset.
    end: u32,
    /// The selection start offset.
    selection_start: u32,
    /// The selection end offset.
    selection_end: u32,
    /// The item kind.
    kind: SymbolKind,
    /// The rendered item name.
    name: &'a str,
}

impl TypeItem {
    /// Build the hierarchy item for one type symbol.
    pub(crate) fn from_symbol(
        query: &QueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Self>> {
        let Some(canonical_id) = query.canonical_symbol(symbol_id)? else {
            return Ok(None);
        };
        let module = query.module(canonical_id.module_id)?;
        let symbol = module.symbols().get_symbol(canonical_id.local_id);
        let kind = match symbol.kind {
            dir::SymbolKind::Class
            | dir::SymbolKind::Struct
            | dir::SymbolKind::Interface
            | dir::SymbolKind::NewtypeInterface
            | dir::SymbolKind::Enum
            | dir::SymbolKind::Newtype => SymbolKind::try_from(symbol.kind)
                .map_err(|_| QueryError::invalid(format!("type item symbol: {canonical_id:?}")))?,
            _ => return Ok(None),
        };
        let name = query
            .symbol_name(canonical_id)?
            .ok_or(QueryError::invalid(format!(
                "type item symbol: {canonical_id:?}"
            )))?;

        // resolve source ranges around the declaration name
        let selection_range =
            query
                .symbol_definition_span(canonical_id)?
                .ok_or(QueryError::missing(format!(
                    "type item span: {canonical_id:?}"
                )))?;
        let range =
            module
                .symbol_local_declaration_span(canonical_id)?
                .ok_or(QueryError::missing(format!(
                    "type item span: {canonical_id:?}"
                )))?;

        let target = Target::new(module.module(), range).with_selection_span(selection_range)?;
        let detail = Formatter::new(module, query).symbol_generics(canonical_id)?;

        Ok(Some(Self {
            name,
            kind,
            detail,
            target,
            symbol_id: canonical_id,
        }))
    }

    /// Return the stable protocol ordering for this item.
    pub(crate) fn order(&self) -> TypeItemOrder<'_> {
        TypeItemOrder {
            file: self.target.span.file.0,
            start: self.target.span.start,
            end: self.target.span.end,
            selection_start: self.target.selection_span.start,
            selection_end: self.target.selection_span.end,
            kind: self.kind,
            name: self.name.as_str(),
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Return a type item at the given position.
    pub fn type_item(
        &self,
        query: &QueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<TypeItem>> {
        let Some(symbol_at) = self.symbol_at_offset(file_id, offset)? else {
            return Ok(None);
        };
        let Some(symbol_id) = symbol_at.symbol() else {
            return Ok(None);
        };

        TypeItem::from_symbol(query, symbol_id)
    }
}
