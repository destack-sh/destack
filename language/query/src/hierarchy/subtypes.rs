use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;

use crate::{ProgramQueryContext, QueryError, QueryResult, TypeItem};

/// A subtypes request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SubtypesRequest {
    /// The type item to expand.
    pub item: TypeItem,
}

/// A subtypes response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SubtypesResponse {
    /// Type hierarchy items.
    pub items: Vec<TypeItem>,
}

impl ProgramQueryContext<'_> {
    /// Return the direct subtypes of a type item.
    pub fn subtypes(&self, request: SubtypesRequest) -> QueryResult<SubtypesResponse> {
        let item = request.item;
        let symbol_id = item.symbol_id;
        let target_id = self
            .symbol_target(symbol_id)?
            .ok_or(QueryError::invalid(format!(
                "type item symbol: {symbol_id:?}"
            )))?;
        let mut subtype_ids: Vec<dir::GlobalSymbolId> = Vec::new();

        // collect direct nominal edges
        for entry in self.base_heritage(target_id)? {
            subtype_ids.push(entry.derived);
        }
        subtype_ids.sort();
        subtype_ids.dedup();

        // collect every indexed type item
        let mut items = Vec::with_capacity(subtype_ids.len());
        for symbol_id in subtype_ids {
            let item = TypeItem::from_symbol(self, symbol_id)?.ok_or(QueryError::invalid(
                format!("type item symbol: {symbol_id:?}"),
            ))?;
            items.push(item);
        }
        items.sort_by(|left, right| left.order().cmp(&right.order()));

        Ok(SubtypesResponse { items })
    }
}
