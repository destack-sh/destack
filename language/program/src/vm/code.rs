use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{FunctionTable, ResumeTable, SideTable};

/// Durable VM code body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Code {
    /// Lowered function bodies for the VM backend.
    pub functions: FunctionTable,
    /// Side table referenced by compact side records.
    pub side_table: SideTable,
    /// Resume states keyed by lowered VM program point.
    pub resume: ResumeTable,
}

impl Code {
    /// Create one VM code body.
    pub fn new(functions: FunctionTable, side_table: SideTable, resume: ResumeTable) -> Self {
        Self {
            functions,
            side_table,
            resume,
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

    /// Return all content ids referenced by this VM code.
    pub fn content_ids(&self) -> Vec<destack_source::ContentId> {
        Vec::new()
    }
}
