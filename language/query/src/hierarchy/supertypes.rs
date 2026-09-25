use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{ProgramQueryContext, QueryError, QueryResult, TypeItem};

/// A supertypes request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SupertypesRequest {
    /// The type item to expand.
    pub item: TypeItem,
}

/// A supertypes response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SupertypesResponse {
    /// Type hierarchy items.
    pub items: Vec<TypeItem>,
}

impl ProgramQueryContext<'_> {
    /// Return the direct supertypes of a type item.
    pub fn supertypes(&self, request: SupertypesRequest) -> QueryResult<SupertypesResponse> {
        let item = request.item;
        let symbol_id = item.symbol_id;
        let target_id = self
            .symbol_target(symbol_id)?
            .ok_or(QueryError::invalid(format!(
                "type item symbol: {symbol_id:?}"
            )))?;
        let mut supertype_ids = Vec::new();

        // collect direct nominal edges declared by the target symbol
        for entry in self.derived_heritage(target_id)? {
            if !supertype_ids.contains(&entry.base) {
                supertype_ids.push(entry.base);
            }
        }

        // collect every indexed type item
        let mut items = Vec::with_capacity(supertype_ids.len());
        for symbol_id in supertype_ids {
            let item = TypeItem::from_symbol(self, symbol_id)?.ok_or(QueryError::invalid(
                format!("type item symbol: {symbol_id:?}"),
            ))?;
            items.push(item);
        }

        Ok(SupertypesResponse { items })
    }
}
