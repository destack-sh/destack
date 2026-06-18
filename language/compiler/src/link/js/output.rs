use std::path::{Component, Path, PathBuf};

use crate::emit::js::{
    JsFormatOptions, Module as ScriptModule, PrintedJsModule,
    print_js_module as print_codegen_script_module,
};
use crate::link::{OutputLayout, SourceMapBuilder, SourceMapMarker};
use crate::{Compiler, CompilerError, CompilerResult, JsLinker};
use base64::Engine as _;
use destack_artifact::{BundleFile, BundleSection, EmitFormat, Script, SourceMap};
use destack_repository::{Module, ProviderContext, SourceMapMode, Target};
use destack_source::{Content, FileType, ModuleId, Uri};

/// One final JS text output policy derived from one target.
#[derive(Debug, Clone, Copy)]
struct JsTextOutputPolicy<'a> {
    /// The target that drives final output shaping.
    target: &'a Target,
}

impl<'a> JsTextOutputPolicy<'a> {
    /// Create one JS text output policy.
    fn new(target: &'a Target) -> Self {
        Self { target }
    }

    /// Build one final JavaScript or TypeScript content payload.
    fn js_content(self, file_type: FileType, code: String) -> Result<Content, String> {
        match file_type {
            FileType::JavaScript | FileType::TypeScript => {
                Ok(crate::Compiler::text_output_content(code))
            }
            other => Err(format!("unsupported JS text output: {other:?}")),
        }
    }

    /// Apply the target output policy before source map annotation.
    fn shape_js_text(self, mut code: String, file_type: FileType) -> Result<String, String> {
        if !matches!(file_type, FileType::JavaScript | FileType::TypeScript) {
            return Ok(code);
        }

        // final JS shaping
        code = self.apply_js_banner_and_footer(code);

        if self.target.should_minify_js_output() {
            // TODO #Incomplete: final JS minification is not implemented yet
        }

        Ok(code)
    }

    /// Append one source map reference when the target wants one.
    fn annotate_js_text_with_map(
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
            || matches!(self.target.source_map_mode(), Some(SourceMapMode::Hidden))
        {
            return Ok(code);
        }

        let Some(map_reference) = map_reference else {
            return Ok(code);
        };

        Ok(self.append_map_reference(code, &map_reference))
    }

    /// Apply configured banner and footer text to one final JS payload.
    fn apply_js_banner_and_footer(self, mut code: String) -> String {
        let banner_prefix = self.js_banner_prefix();

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
    fn js_banner_prefix(self) -> String {
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
    fn js_banner_prefix_byte_count(self) -> u32 {
        self.js_banner_prefix().len() as u32
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
        file_type: FileType,
        module: &ScriptModule,
        context: &dyn ProviderContext,
    ) -> CompilerResult<PrintedJsModule> {
        // source artifacts
        let parsed =
            self.artifacts
                .dir_parsed(module_id)
                .map_err(|error| CompilerError::Internal {
                    message: format!(
                        "missing committed parsed DIR artifact for module {module_id:?}: {error:?}"
                    ),
                })?;
        let source_module = self.compiler.module(context.revision(), module_id)?;
        let source_file = self.compiler.file(context, source_module.file_id)?;
        let options = if target.should_minify_js_output() {
            JsFormatOptions::minimal()
        } else {
            JsFormatOptions::pretty()
        }
        .with_file_type(file_type);

        print_codegen_script_module(options, &parsed, source_file.as_ref(), module).map_err(
            |error| CompilerError::Internal {
                message: format!("failed to print JS module: {error:?}"),
            },
        )
    }

    /// Build one source map builder for one linked JS module.
    fn script_module_map(
        &self,
        package_dir: &Path,
        module: &Module,
        printed: &PrintedJsModule,
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

    /// Link one printed JavaScript or TypeScript module into output files.
    fn link_printed_script_text_files(
        &self,
        target: &Target,
        package_dir: &Path,
        module: &Module,
        file_type: FileType,
        output_path: &Path,
        printed: PrintedJsModule,
        map_path: Option<&Path>,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Vec<BundleFile>> {
        // source map
        let map = self.script_module_map(package_dir, module, &printed, context)?;

        self.link_script_text_files(
            target,
            file_type,
            output_path,
            printed.code,
            Some(map),
            map_path,
        )
        .map_err(|message| CompilerError::Internal { message })
    }

    /// Link one printed JS module for one concrete output file type.
    fn link_printed_script_files(
        &self,
        module: &Module,
        artifact: &Script,
        target: &Target,
        package_dir: &Path,
        file_type: FileType,
        output_path: &Path,
        map_path: Option<&Path>,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Vec<BundleFile>> {
        // print once
        let Some(script) = artifact.ecmascript_module() else {
            return Err(CompilerError::Internal {
                message: format!("expected ECMAScript script for module {:?}", module.id),
            });
        };
        let printed = self.print_js_module(module.id, target, file_type, script, context)?;

        // JS text
        if matches!(file_type, FileType::JavaScript | FileType::TypeScript) {
            return self.link_printed_script_text_files(
                target,
                package_dir,
                module,
                file_type,
                output_path,
                printed,
                map_path,
                context,
            );
        }

        Err(CompilerError::Internal {
            message: format!("unsupported file type: {file_type:?}"),
        })
    }

    /// Link one declaration output file when the target requests one.
    fn link_script_declaration_file(
        &self,
        module: &Module,
        declaration_text: &str,
        target: &Target,
        package_dir: &Path,
        root_dir: Option<&Path>,
    ) -> Result<BundleFile, String> {
        let output_path = OutputLayout::module_output_path(
            package_dir,
            root_dir,
            target,
            module,
            FileType::TypeScriptDeclaration,
        )?;

        let content = Compiler::text_output_content(declaration_text.to_string());

        self.compiler
            .intern_output_file(
                BundleSection::Declaration,
                Uri::from_path(&output_path),
                FileType::TypeScriptDeclaration,
                content,
                None,
            )
            .map_err(|error| error.to_string())
    }

    /// Link one JS output into output files.
    pub(crate) fn link_js_output_files(
        &self,
        module: &Module,
        artifact: &Script,
        target: &Target,
        package_dir: &Path,
        root_dir: Option<&Path>,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Vec<BundleFile>> {
        let mut entries = Vec::new();
        let file_types = linked_script_file_types(target)
            .map_err(|message| CompilerError::Internal { message })?;
        let map_path = file_types
            .contains(&FileType::SourceMap)
            .then(|| {
                OutputLayout::module_output_path(
                    package_dir,
                    root_dir,
                    target,
                    module,
                    FileType::SourceMap,
                )
            })
            .transpose()
            .map_err(|message| CompilerError::Internal { message })?;

        // code files
        for file_type in &file_types {
            if *file_type == FileType::TypeScriptDeclaration {
                continue;
            }

            let output_path =
                OutputLayout::module_output_path(package_dir, root_dir, target, module, *file_type)
                    .map_err(|message| CompilerError::Internal { message })?;
            if *file_type == FileType::SourceMap {
                continue;
            }

            let files = self.link_printed_script_files(
                module,
                artifact,
                target,
                package_dir,
                *file_type,
                &output_path,
                map_path.as_deref(),
                context,
            )?;

            entries.extend(files);
        }

        // declarations
        if let Some(declaration) = &artifact.declaration
            && file_types.contains(&FileType::TypeScriptDeclaration)
        {
            let declaration = self
                .link_script_declaration_file(
                    module,
                    &declaration.text,
                    target,
                    package_dir,
                    root_dir,
                )
                .map_err(|message| CompilerError::Internal { message })?;

            entries.push(declaration);
        }

        Ok(entries)
    }

    /// Link one final JS text output and any related sidecars.
    pub(crate) fn link_script_text_files(
        &self,
        target: &Target,
        file_type: FileType,
        output_path: &Path,
        code: String,
        map: Option<SourceMapBuilder>,
        map_path: Option<&Path>,
    ) -> Result<Vec<BundleFile>, String> {
        let output_policy = JsTextOutputPolicy::new(target);
        let map_reference = map_path.map(|path| relative_map_reference(output_path, path));
        let shaped_code = output_policy.shape_js_text(code, file_type)?;
        let mut map = map;
        let has_map = map.is_some();

        // banner bytes shift every emitted marker forward in the final output
        if let Some(map) = &mut map {
            map.prepend_emitted_bytes(output_policy.js_banner_prefix_byte_count());
        }
        let map = map.map(|map| {
            map.build(
                &shaped_code,
                output_policy.map_annotation_line_count(has_map, map_reference.is_some()),
            )
        });

        let code =
            output_policy.annotate_js_text_with_map(shaped_code, map.as_ref(), map_reference)?;
        let content = output_policy.js_content(file_type, code)?;
        let section = self.bundle_section_for_file(file_type);
        let mut files = vec![
            self.compiler
                .intern_output_file(
                    section,
                    Uri::from_path(output_path),
                    file_type,
                    content,
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

        let content = Compiler::source_map_content(map)
            .map_err(|error| format!("failed to serialize source map: {error}"))?;

        files.push(
            self.compiler
                .intern_output_file(
                    BundleSection::SourceMap,
                    Uri::from_path(map_path),
                    FileType::SourceMap,
                    content,
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

/// Choose the linked file types for one JS target.
fn linked_script_file_types(target: &Target) -> Result<Vec<FileType>, String> {
    match target.emit {
        EmitFormat::Js => {
            let mut file_types = vec![FileType::JavaScript];
            if target.output.declaration {
                file_types.push(FileType::TypeScriptDeclaration);
            }
            if target.emits_source_map_output() {
                file_types.push(FileType::SourceMap);
            }

            Ok(file_types)
        }
        EmitFormat::Ts => {
            let mut file_types = vec![FileType::TypeScript];
            if target.emits_source_map_output() {
                file_types.push(FileType::SourceMap);
            }

            Ok(file_types)
        }
        other => Err(format!("expected JS or TS output, got {other:?}")),
    }
}
