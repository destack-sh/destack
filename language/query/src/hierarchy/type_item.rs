use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;

use crate::{
    Formatter, ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult,
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
    pub generics: Option<String>,
    /// The target source.
    pub target: Target,
    /// The resolved type symbol.
    pub symbol_id: dir::GlobalSymbolId,
}

/// A type item request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeItemRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// A type item response.
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
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Self>> {
        let Some(target_id) = program.symbol_target(symbol_id)? else {
            return Ok(None);
        };
        let module = program.module(target_id.module_id)?;
        let symbol = module.bindings()?.get_symbol(target_id.local_id);
        let kind = match symbol.kind {
            dir::SymbolKind::Class
            | dir::SymbolKind::Struct
            | dir::SymbolKind::Interface
            | dir::SymbolKind::NewtypeInterface
            | dir::SymbolKind::Enum
            | dir::SymbolKind::Newtype => SymbolKind::try_from(symbol.kind)
                .map_err(|_| QueryError::invalid(format!("type item symbol: {target_id:?}")))?,
            _ => return Ok(None),
        };
        let name = program
            .symbol_name(target_id)?
            .ok_or(QueryError::invalid(format!(
                "type item symbol: {target_id:?}"
            )))?;

        // resolve source ranges around the declaration name
        let selection_range =
            program
                .symbol_definition_span(target_id)?
                .ok_or(QueryError::missing(format!(
                    "type item span: {target_id:?}"
                )))?;
        let range = module
            .symbol_local_declaration_span(program, target_id)?
            .ok_or(QueryError::missing(format!(
                "type item span: {target_id:?}"
            )))?;

        let target = Target::new(module.module(), range).with_selection_span(selection_range)?;
        let generics = Formatter::new(&module, program).symbol_generics(target_id)?;

        Ok(Some(Self {
            name,
            kind,
            generics,
            target,
            symbol_id: target_id,
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
        request: TypeItemRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<TypeItemResponse> {
        let position = request.position;
        let Some(symbol_at) = self
            .cursor(position.file_id, position.offset)?
            .symbol(program)?
        else {
            return Ok(TypeItemResponse { item: None });
        };
        let Some(symbol_id) = symbol_at.symbol() else {
            return Ok(TypeItemResponse { item: None });
        };
        let item = TypeItem::from_symbol(program, symbol_id)?;

        Ok(TypeItemResponse { item })
    }
}
