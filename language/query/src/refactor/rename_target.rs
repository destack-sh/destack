use destack_dir as dir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{ModuleQueryContext, ProgramQueryContext, QueryPosition, QueryResult, Target};

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

/// A rename target request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTargetRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// A rename target response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTargetResponse {
    /// Rename target, if available.
    pub target: Option<RenameTarget>,
}

impl ModuleQueryContext<'_> {
    /// Return the rename target at the given position.
    pub fn rename_target(
        &self,
        request: RenameTargetRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<RenameTargetResponse> {
        let position = request.position;
        let Some(selection) =
            self.resolve_rename_target(program, position.file_id, position.offset)?
        else {
            return Ok(RenameTargetResponse { target: None });
        };
        let target = Target::new(self.module(), selection.occurrence.span);
        let target = RenameTarget {
            target,
            symbols: selection.symbols,
            placeholder: selection.placeholder,
        };

        Ok(RenameTargetResponse {
            target: Some(target),
        })
    }
}
