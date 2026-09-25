use std::path::{Component, Path, PathBuf};

use crate::emit::js;
use crate::link::{OutputLayout, SourceMapBuilder, SourceMapMarker};
use crate::{Compiler, CompilerError, CompilerResult, JsLinker};
use base64::Engine;
use tspp_artifact::{BundleFile, BundleSection, DirParsed, Script, SourceMap};
use tspp_repository::{Module, ProviderContext, SourceMapMode, Target};
use tspp_source::{FileType, ModuleId, Uri};

/// One final JS text output derived from one target.
#[derive(Debug, Clone, Copy)]
struct TextOutput<'a> {
    /// The target that drives final output shaping.
    target: &'a Target,
}

impl<'a> TextOutput<'a> {
    /// Create one JS text output.
    fn new(target: &'a Target) -> Self {
        Self { target }
    }

    /// Append one source map reference when the target wants one.
    fn annotate(
        self,
        code: String,
        map: Option<&SourceMap>,
        map_reference: Option<String>,
    ) -> Result<String, String> {
        // inline source maps stay in the text payload only
        if self.target.uses_inline_source_maps() {
            let Some(map) = map else {
                return Ok(code);
            };
            let inline_map = self.inline_map_url(map)?;

            return Ok(self.append_map_reference(code, &inline_map));
        }

        // hidden maps emit sidecar outputs but do not annotate the text payload
        if !self.target.emits_source_map_output()
            || matches!(self.target.source_map, Some(SourceMapMode::Hidden))
        {
            return Ok(code);
        }

        let Some(map_reference) = map_reference else {
            return Ok(code);
        };

        Ok(self.append_map_reference(code, &map_reference))
    }

    /// Apply configured banner and footer text to one final JS payload.
    fn apply_banner_and_footer(self, mut code: String) -> String {
        let banner_prefix = self.banner_prefix();

        // prepend banner text before the emitted module body
        if !banner_prefix.is_empty() {
            let mut with_banner = banner_prefix;

            with_banner.push_str(&code);
            code = with_banner;
        }

        // append footer text before any source map reference
        if let Some(footer) = self.target.js.output.footer.as_deref() {
            if !code.is_empty() && !code.ends_with('\n') {
                code.push('\n');
            }

            code.push_str(footer);

            if !code.is_empty() && !code.ends_with('\n') {
                code.push('\n');
            }
        }

        code
    }

    /// Return the exact banner prefix inserted before mapped JS code.
    fn banner_prefix(self) -> String {
        let Some(banner) = self.target.js.output.banner.as_deref() else {
            return String::new();
        };
        let mut prefix = banner.to_string();

        if !prefix.is_empty() && !prefix.ends_with('\n') {
            prefix.push('\n');
        }

        prefix
    }

    /// Return the mapped byte offset introduced before emitted JS code.
    fn banner_prefix_byte_count(self) -> u32 {
        self.banner_prefix().len() as u32
    }

    /// Return the number of unmapped annotation lines appended after mapped JS code.
    fn map_annotation_line_count(self, has_map: bool, has_map_reference: bool) -> usize {
        if self.target.uses_inline_source_maps() && has_map {
            return 1;
        }

        if has_map_reference {
            return 1;
        }

        0
    }

    /// Build one inline source map data URL.
    fn inline_map_url(self, map: &SourceMap) -> Result<String, String> {
        let map = serde_json::to_string(map)
            .map_err(|error| format!("failed to serialize inline source map: {error}"))?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(map.as_bytes());

        Ok(format!(
            "data:application/json;charset=utf-8;base64,{encoded}"
        ))
    }

    /// Append one source map reference comment to one text payload.
    fn append_map_reference(self, mut code: String, reference: &str) -> String {
        if !code.is_empty() && !code.ends_with('\n') {
            code.push('\n');
        }

        code.push_str(&format!("//# sourceMappingURL={reference}\n"));
        code
    }
}

impl JsLinker<'_> {
    /// Print one JS module with one exact source map marker stream.
    pub(crate) fn print_js_module(
        &self,
        module_id: ModuleId,
        target: &Target,
        module: &js::Module,
        context: &dyn ProviderContext,
    ) -> CompilerResult<js::PrintedJsModule> {
        // source artifacts
        let parsed = self
            .artifacts
            .read::<DirParsed>(module_id)
            .map_err(|error| CompilerError::Internal {
                message: format!(
                    "missing committed parsed DIR artifact for module {module_id:?}: {error:?}"
                ),
            })?;
        let source_module = self.compiler.module(context.revision(), module_id)?;
        let source_file = self.compiler.file(context, source_module.file_id)?;
        let options = if target.should_minify_js_output() {
            js::Options::minimal()
        } else {
            js::Options::pretty()
        };

        js::print_js_module(options, &parsed, source_file.as_ref(), module).map_err(|error| {
            CompilerError::Internal {
                message: format!("failed to print JS module: {error:?}"),
            }
        })
    }

    /// Build one source map builder for one linked JS module.
    fn script_module_map(
        &self,
        package_dir: &Path,
        module: &Module,
        printed: &js::PrintedJsModule,
        context: &dyn ProviderContext,
    ) -> CompilerResult<SourceMapBuilder> {
        let source_path = self
            .compiler
            .package_relative_uri_path(package_dir, &module.uri);
        let source_file = self.compiler.file(context, module.file_id)?;
        let markers = printed
            .markers
            .iter()
            .copied()
            .filter_map(|marker| SourceMapMarker::from_file_marker(0, source_file.as_ref(), marker))
            .collect();

        Ok(SourceMapBuilder::new(vec![source_path], markers))
    }

    /// Link one JS output into output files.
    pub(crate) fn link_js_output_files(
        &self,
        module: &Module,
        script: &Script,
        target: &Target,
        package_dir: &Path,
        root_dir: Option<&Path>,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Vec<BundleFile>> {
        let map_path = target
            .emits_source_map_output()
            .then(|| OutputLayout::module_output_path(package_dir, root_dir, target, module, "map"))
            .transpose()
            .map_err(|message| CompilerError::Internal { message })?;
        let output_path =
            OutputLayout::module_output_path(package_dir, root_dir, target, module, "js")
                .map_err(|message| CompilerError::Internal { message })?;

        // print the script and build its source map
        let printed = self.print_js_module(module.id, target, script.module(), context)?;
        let map = self.script_module_map(package_dir, module, &printed, context)?;

        // link the script and optional source map file
        self.link_script_text_files(
            target,
            &output_path,
            printed.code,
            Some(map),
            map_path.as_deref(),
        )
        .map_err(|message| CompilerError::Internal { message })
    }

    /// Link one final JS text output and any related sidecars.
    pub(crate) fn link_script_text_files(
        &self,
        target: &Target,
        output_path: &Path,
        code: String,
        map: Option<SourceMapBuilder>,
        map_path: Option<&Path>,
    ) -> Result<Vec<BundleFile>, String> {
        let output = TextOutput::new(target);
        let map_reference = map_path.map(|path| relative_map_reference(output_path, path));
        let shaped_code = output.apply_banner_and_footer(code);
        let mut map = map;
        let has_map = map.is_some();

        // banner bytes shift every emitted marker forward in the final output
        if let Some(map) = &mut map {
            map.prepend_emitted_bytes(output.banner_prefix_byte_count());
        }
        let map = map.map(|map| {
            map.build(
                &shaped_code,
                output.map_annotation_line_count(has_map, map_reference.is_some()),
            )
        });

        let code = output.annotate(shaped_code, map.as_ref(), map_reference)?;
        let bytes = Compiler::encode_output_text(code);
        let section = if self.target.is_single_file() {
            BundleSection::Entry
        } else {
            BundleSection::Module
        };
        let mut files = vec![
            self.compiler
                .put_output_file(
                    section,
                    Uri::from_path(output_path),
                    FileType::JavaScript,
                    &bytes,
                    None,
                )
                .map_err(|error| error.to_string())?,
        ];

        let Some(map) = map.as_ref() else {
            return Ok(files);
        };

        let Some(map_path) = map_path else {
            return Ok(files);
        };

        let bytes = Compiler::encode_source_map(map)
            .map_err(|error| format!("failed to serialize source map: {error}"))?;

        files.push(
            self.compiler
                .put_output_file(
                    BundleSection::SourceMap,
                    Uri::from_path(map_path),
                    FileType::SourceMap,
                    &bytes,
                    None,
                )
                .map_err(|error| error.to_string())?,
        );

        Ok(files)
    }
}

/// Build one relative source map reference from one output path.
fn relative_map_reference(output_path: &Path, map_path: &Path) -> String {
    let relative = relative_path_between(output_path, map_path);
    let relative = relative.to_string_lossy().replace('\\', "/");

    if relative.starts_with('.') {
        return relative;
    }

    format!("./{relative}")
}

/// Return one relative path from one emitted output file to another.
fn relative_path_between(from_output_path: &Path, to_output_path: &Path) -> PathBuf {
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
