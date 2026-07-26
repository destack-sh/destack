use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{ProgramQueryContext, QueryError, QueryResult, TypeItem};

/// Request the direct supertypes of a type item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SupertypesRequest {
    /// The type item to expand.
    pub item: TypeItem,
}

/// Response payload for supertype queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SupertypesResponse {
    /// Type hierarchy items.
    pub items: Vec<TypeItem>,
}

impl ProgramQueryContext<'_> {
    /// Return the direct supertypes of a type item.
    pub fn supertypes(&self, item: &TypeItem) -> QueryResult<Vec<TypeItem>> {
        let symbol_id = item.symbol_id;
        let canonical_id = self
            .canonical_symbol(symbol_id)?
            .ok_or(QueryError::invalid(format!(
                "type item symbol: {symbol_id:?}"
            )))?;
        let mut supertype_ids = Vec::new();

        // collect direct nominal edges declared by the target symbol
        for entry in self.derived_heritage(canonical_id)? {
            if !supertype_ids.contains(&entry.base) {
                supertype_ids.push(entry.base);
            }
        }

        // transcribe every exact indexed type item
        let mut items = Vec::with_capacity(supertype_ids.len());
        for symbol_id in supertype_ids {
            let module = self.module(symbol_id.module_id)?;
            let item =
                module
                    .type_item_from_symbol(self, symbol_id)?
                    .ok_or(QueryError::invalid(format!(
                        "type item symbol: {symbol_id:?}"
                    )))?;
            items.push(item);
        }

        Ok(items)
    }
}
