use destack_source::{ModuleId, ProvenanceTable, TextMap};

use crate::Module;

use super::{PrintError, Printer};

/// A printed ECMAScript module.
#[derive(Debug, Clone)]
pub struct PrintedModule {
    /// The module identity.
    pub id: ModuleId,
    /// The emitted ECMAScript text.
    pub text: String,
    /// The provenance extents over the emitted text.
    pub map: TextMap,
    /// The provenance graph referenced by the emitted extents.
    pub provenance: ProvenanceTable,
}

impl Module {
    /// Print compact ECMAScript text and retain its provenance.
    pub fn print(self) -> Result<PrintedModule, PrintError> {
        let (text, map) = Printer::print(&self)?;

        Ok(PrintedModule {
            id: self.id,
            text,
            map,
            provenance: self.provenance,
        })
    }
}
