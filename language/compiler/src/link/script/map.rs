use std::path::Path;
use destack_workspace::ProviderContext;

use crate::link::{SourceMapBuilder, SourceMapMarker};
use crate::{Compiler};
use destack_codegen_js::PrintedScriptModule;
use destack_source::ModuleId;

impl Compiler {
    /// Build one source map builder for linked script modules.
    pub(crate) fn linked_script_source_map_for_parts(
        &self,
        package_dir: &Path,
        parts: &[(ModuleId, PrintedScriptModule)],
        context: &dyn ProviderContext,
    ) -> SourceMapBuilder {
        let mut sources = Vec::new();
        let mut markers = Vec::new();
        let mut generated_byte_offset = 0u32;

        // compose each printed module with one stable source index
        for (part_index, (module_id, printed)) in parts.iter().enumerate() {
            let module = self.module(context.revision(), *module_id);
            let source_file = self.file(context, module.file_id);
            let source_path = self.package_relative_module_path(package_dir, *module_id, context);
            let normalized_length = printed.code.trim_end().len() as u32;

            sources.push(source_path);

            for marker in &printed.markers {
                if marker.dest > normalized_length {
                    continue;
                }

                let Some(mut marker) =
                    SourceMapMarker::from_file_marker(part_index, &source_file, *marker)
                else {
                    continue;
                };

                marker.generated_byte += generated_byte_offset;
                markers.push(marker);
            }

            generated_byte_offset += normalized_length;

            // linked module text inserts one blank separator line between parts
            if part_index + 1 < parts.len() {
                generated_byte_offset += 2;
            }
        }

        SourceMapBuilder::new(sources, markers)
    }
}
