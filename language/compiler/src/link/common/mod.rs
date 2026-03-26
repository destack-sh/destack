mod layout;
mod manifest;
mod map;
mod output;

use crate::Compiler;

use destack_source::{ModuleId, Span};

pub(crate) use layout::{OutputLayout, OutputLocation, module_output_base_path};
pub(crate) use map::{SourceMapBuilder, SourceMapMarker};

impl Compiler {
    /// Return the anchor span for one linked module.
    pub(crate) fn module_anchor_span(&self, module_id: ModuleId) -> Span {
        let module = self.program.modules.get(module_id);

        Span::empty(module.file_id)
    }
}
