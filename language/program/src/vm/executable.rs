use serde::{Deserialize, Serialize};

use super::{FunctionTable, ResumeTable, SideTable, TypeTable};

/// Durable VM executable body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Executable {
    /// Lowered function bodies for the VM backend.
    pub functions: FunctionTable,
    /// Side table referenced by compact side records.
    pub side_table: SideTable,
    /// Resume recipes keyed by lowered VM program point.
    pub resume: ResumeTable,
    /// Compiled type layouts used by the VM executable.
    pub type_table: TypeTable,
}

impl Executable {
    /// Create one VM executable body.
    pub fn new(
        functions: FunctionTable,
        side_table: SideTable,
        resume: ResumeTable,
        type_table: TypeTable,
    ) -> Self {
        Self {
            functions,
            side_table,
            resume,
            type_table,
        }
    }

    /// Return the lowered VM functions.
    pub fn functions(&self) -> &FunctionTable {
        &self.functions
    }

    /// Return the compact VM side table.
    pub fn side_table(&self) -> &SideTable {
        &self.side_table
    }

    /// Return the VM resume table.
    pub fn resume(&self) -> &ResumeTable {
        &self.resume
    }

    /// Return the compiled type layout table.
    pub fn type_table(&self) -> &TypeTable {
        &self.type_table
    }

    /// Return all content ids referenced by this VM executable.
    pub fn content_ids(&self) -> Vec<destack_source::ContentId> {
        Vec::new()
    }
}
