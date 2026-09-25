use std::path::{Component, Path, PathBuf};
use tspp_repository::ProviderContext;

use crate::emit::js;
use crate::link::{SourceMapBuilder, SourceMapMarker};
use crate::{CompilerResult, JsLinker};
use tspp_source::ModuleId;

impl JsLinker<'_> {
    /// Build one source map builder for linked JS modules.
    pub(crate) fn script_source_map_for_parts(
        &self,
        package_dir: &Path,
        emitted_source_map_path: &Path,
        parts: &[(ModuleId, js::PrintedJsModule)],
        is_minimal: bool,
        context: &dyn ProviderContext,
    ) -> CompilerResult<SourceMapBuilder> {
        let mut sources = Vec::new();
        let mut markers = Vec::new();
        let mut emitted_byte_offset = 0u32;

        // compose each printed module with one stable source index
        for (part_index, (module_id, printed)) in parts.iter().enumerate() {
            let module = self.compiler.module(context.revision(), *module_id)?;
            let source_file = self.compiler.file(context, module.file_id)?;
            let source_path = self.script_source_map_path(
                package_dir,
                emitted_source_map_path,
                *module_id,
                context,
            )?;
            let normalized_length = printed.code.trim_end().len() as u32;

            sources.push(source_path);

            for marker in &printed.markers {
                if marker.dest > normalized_length {
                    continue;
                }

                let Some(mut marker) =
                    SourceMapMarker::from_file_marker(part_index, source_file.as_ref(), *marker)
                else {
                    continue;
                };

                marker.emitted_byte += emitted_byte_offset;
                markers.push(marker);
            }

            emitted_byte_offset += normalized_length;

            // linked module text inserts one separator between parts
            if part_index + 1 < parts.len() {
                let separator = self.script_part_separator(&printed.code, is_minimal);
                emitted_byte_offset += separator.len() as u32;
            }
        }

        Ok(SourceMapBuilder::new(sources, markers))
    }

    /// Build one map-relative source path for one linked JS module.
    fn script_source_map_path(
        &self,
        package_dir: &Path,
        emitted_source_map_path: &Path,
        module_id: ModuleId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<String> {
        let module = self.compiler.module(context.revision(), module_id)?;

        let Some(source_path) = module.path.as_ref() else {
            return Ok(self
                .compiler
                .package_relative_uri_path(package_dir, &module.uri));
        };

        Ok(
            relative_output_path_between(emitted_source_map_path, source_path)
                .to_string_lossy()
                .replace('\\', "/"),
        )
    }
}

/// Return one relative path from one emitted output file to another path.
fn relative_output_path_between(from_output_path: &Path, to_output_path: &Path) -> PathBuf {
    let from_directory = from_output_path.parent().unwrap_or_else(|| Path::new(""));
    let from_components = from_directory.components().collect::<Vec<_>>();
    let to_components = to_output_path.components().collect::<Vec<_>>();
    let mut shared = 0;

    while shared < from_components.len()
        && shared < to_components.len()
        && from_components[shared] == to_components[shared]
    {
        shared += 1;
    }

    let mut relative_path = PathBuf::new();

    for component in &from_components[shared..] {
        if matches!(component, Component::Normal(_)) {
            relative_path.push("..");
        }
    }

    for component in &to_components[shared..] {
        if let Component::Normal(segment) = component {
            relative_path.push(segment);
        }
    }

    relative_path
}
