use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::{ModuleQueryContext, QueryContext, QueryPosition, QueryResult, Target};

/// Target of a rename query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTarget {
    /// The rename source.
    pub target: Target,
    /// The resolved rename symbols.
    pub symbols: Vec<dir::GlobalSymbolId>,
    /// The current name.
    pub placeholder: String,
}

/// Request the rename target at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTargetRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for rename target queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTargetResponse {
    /// Rename target, if available.
    pub target: Option<RenameTarget>,
}

impl ModuleQueryContext<'_> {
    /// Return the rename target at the given position.
    pub fn rename_target(
        &self,
        query: &QueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<RenameTarget>> {
        let Some(selection) = self.resolve_rename_target(query, file_id, offset)? else {
            return Ok(None);
        };
        let target = Target::new(self.module(), selection.occurrence.span);

        Ok(Some(RenameTarget {
            target,
            symbols: selection.symbols,
            placeholder: selection.placeholder,
        }))
    }
}
