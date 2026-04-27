use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use crate::link::{OutputLayout, SourceMapBuilder, SourceMapMarker};
use crate::{Compiler, CompilerContext};
use base64::Engine as _;
use destack_artifact::{
    DirPatched, EmitFormat, OutputContent, OutputFile, ScriptArtifact, SourceMapArtifact,
};
use destack_codegen_js::{
    JsFormatOptions, PrintedScriptModule, ScriptModule,
    print_script_module as print_codegen_script_module,
};
use destack_source::{FileType, ModuleId, TargetId, Uri};
use destack_workspace::{Module, SourceMapMode, Target};

/// One final script text output policy derived from one target.
#[derive(Debug, Clone, Copy)]
struct ScriptTextOutputPolicy<'a> {
    /// The target that drives final output shaping.
    target: &'a Target,
}

impl<'a> ScriptTextOutputPolicy<'a> {
    /// Create one script text output policy.
    fn new(target: &'a Target) -> Self {
        Self { target }
    }

    /// Build one final JavaScript or TypeScript content payload.
    fn script_content(self, file_type: FileType, code: String) -> Result<OutputContent, String> {
        match file_type {
            FileType::JavaScript => Ok(OutputContent::javascript(code)),
            FileType::TypeScript => Ok(OutputContent::typescript(code)),
            other => Err(format!("unsupported script text output: {other:?}")),
        }
    }

    /// Apply the target output policy before source map annotation.
    fn shape_script_text(self, mut code: String, file_type: FileType) -> Result<String, String> {
        if !matches!(file_type, FileType::JavaScript | FileType::TypeScript) {
            return Ok(code);
        }

        // final script shaping
        code = self.apply_script_banner_and_footer(code);

        if self.target.should_minify_bundle_output() {
            // TODO #Incomplete: final JS minification is not implemented yet
        }

        Ok(code)
    }

    /// Append one source map reference when the target wants one.
    fn annotate_script_text_with_source_map(
        self,
        code: String,
        source_map: Option<&SourceMapArtifact>,
        source_map_reference: Option<String>,
    ) -> Result<String, String> {
        // inline source maps stay in the text payload only
        if self.target.uses_inline_source_maps() {
            let Some(source_map) = source_map else {
                return Ok(code);
            };
            let inline_map = self.inline_source_map_url(source_map)?;

            return Ok(self.append_source_map_reference(code, &inline_map));
        }

        // hidden maps emit sidecar outputs but do not annotate the text payload
        if !self.target.emits_source_map_output()
            || matches!(self.target.source_map_mode(), Some(SourceMapMode::Hidden))
        {
            return Ok(code);
        }

        let Some(source_map_reference) = source_map_reference else {
            return Ok(code);
        };

        Ok(self.append_source_map_reference(code, &source_map_reference))
    }

    /// Apply configured banner and footer text to one final script payload.
    fn apply_script_banner_and_footer(self, mut code: String) -> String {
        let banner_prefix = self.script_banner_prefix();

        // prepend banner text before the emitted module body
        if !banner_prefix.is_empty() {
            let mut with_banner = banner_prefix;

            with_banner.push_str(&code);
            code = with_banner;
        }

        // append footer text before any source map reference
        if let Some(footer) = self.target.bundle_output.footer.as_deref() {
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

    /// Return the exact banner prefix inserted before mapped script code.
    fn script_banner_prefix(self) -> String {
        let Some(banner) = self.target.bundle_output.banner.as_deref() else {
            return String::new();
        };
        let mut prefix = banner.to_string();

        if !prefix.is_empty() && !prefix.ends_with('\n') {
            prefix.push('\n');
        }

        prefix
    }

    /// Return the mapped byte offset introduced before generated script code.
    fn script_banner_prefix_byte_count(self) -> u32 {
        self.script_banner_prefix().len() as u32
    }

    /// Return the number of unmapped annotation lines appended after mapped script code.
    fn source_map_annotation_line_count(
        self,
        has_source_map: bool,
        has_source_map_reference: bool,
    ) -> usize {
        if self.target.uses_inline_source_maps() && has_source_map {
            return 1;
        }

        if has_source_map_reference {
            return 1;
        }

        0
    }

    /// Build one inline source map data URL.
    fn inline_source_map_url(self, source_map: &SourceMapArtifact) -> Result<String, String> {
        let source_map = serde_json::to_string(source_map)
            .map_err(|error| format!("failed to serialize inline source map: {error}"))?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(source_map.as_bytes());

        Ok(format!(
            "data:application/json;charset=utf-8;base64,{encoded}"
        ))
    }

    /// Append one source map reference comment to one text payload.
    fn append_source_map_reference(self, mut code: String, reference: &str) -> String {
        if !code.is_empty() && !code.ends_with('\n') {
            code.push('\n');
        }

        code.push_str(&format!("//# sourceMappingURL={reference}\n"));
        code
    }
}

impl Compiler {
    /// Print one script module with one exact source map marker stream.
    pub(crate) fn print_script_module(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
        target: &Target,
        file_type: FileType,
        module: &ScriptModule,
        context: &CompilerContext<'_>,
    ) -> Result<PrintedScriptModule, String> {
        // source artifacts
        let ast = self
            .ast(module_id)
            .ok_or_else(|| format!("missing committed AST artifact for module {module_id:?}"))?;
        let dir = self.script_dir(module_id, target_id, context)?;
        let source_module = context.module(module_id);
        let source_file = context.file(source_module.file_id);
        let options = if target.should_minify_bundle_output() {
            JsFormatOptions::minimal()
        } else {
            JsFormatOptions::pretty()
        }
        .with_file_type(file_type);

        print_codegen_script_module(options, &ast, dir.as_ref(), source_file.as_ref(), module)
            .map_err(|error| format!("failed to print script module: {error:?}"))
    }

    /// Return the patched DIR artifact for one script module target.
    fn script_dir(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
        context: &CompilerContext<'_>,
    ) -> Result<Arc<DirPatched>, String> {
        // target profile
        let profile_id = context
            .profile_id_for_target(module_id, target_id)
            .ok_or_else(|| {
                format!(
                    "profile not found for target '{}'",
                    self.target_name_for_revision(context.revision(), target_id)
                )
            })?;

        // patched dir
        let dir = self.dir_patched(module_id, profile_id).ok_or_else(|| {
            format!(
                "missing patched DIR artifact for module {module_id:?} target '{}'",
                self.target_name_for_revision(context.revision(), target_id)
            )
        })?;

        Ok(dir)
    }

    /// Build one source map builder for one linked script module.
    fn script_module_source_map(
        &self,
        package_dir: &Path,
        module: &Module,
        printed: &PrintedScriptModule,
        context: &CompilerContext<'_>,
    ) -> SourceMapBuilder {
        let source_path = self.package_relative_uri_path(package_dir, &module.uri);
        let source_file = context.file(module.file_id);
        let markers = printed
            .markers
            .iter()
            .copied()
            .filter_map(|marker| SourceMapMarker::from_file_marker(0, source_file.as_ref(), marker))
            .collect();

        SourceMapBuilder::new(vec![source_path], markers)
    }

    /// Link one printed JavaScript or TypeScript module into output files.
    fn link_printed_script_text_files(
        &self,
        target: &Target,
        package_dir: &Path,
        module: &Module,
        file_type: FileType,
        output_path: &Path,
        printed: PrintedScriptModule,
        source_map_path: Option<&Path>,
        context: &CompilerContext<'_>,
    ) -> Result<Vec<OutputFile>, String> {
        // source map
        let source_map = self.script_module_source_map(package_dir, module, &printed, context);

        self.link_script_text_files(
            target,
            file_type,
            output_path,
            printed.code,
            Some(source_map),
            source_map_path,
        )
    }

    /// Link one printed script module for one concrete output file type.
    fn link_printed_script_files(
        &self,
        module: &Module,
        artifact: &ScriptArtifact,
        target_id: &TargetId,
        target: &Target,
        package_dir: &Path,
        file_type: FileType,
        output_path: &Path,
        source_map_path: Option<&Path>,
        context: &CompilerContext<'_>,
    ) -> Result<Vec<OutputFile>, String> {
        // print once
        let printed = self.print_script_module(
            module.id,
            target_id,
            target,
            file_type,
            &artifact.module,
            context,
        )?;

        // script text
        if matches!(file_type, FileType::JavaScript | FileType::TypeScript) {
            return self.link_printed_script_text_files(
                target,
                package_dir,
                module,
                file_type,
                output_path,
                printed,
                source_map_path,
                context,
            );
        }

        if file_type == FileType::Html {
            return Err("FUGU #Incomplete".to_string());
        }

        Err(format!("unsupported file type: {file_type:?}"))
    }

    /// Link one declaration output file when the target requests one.
    fn link_script_declaration_file(
        &self,
        module: &Module,
        declaration_text: &str,
        target: &Target,
        package_dir: &Path,
        root_dir: Option<&Path>,
    ) -> Result<OutputFile, String> {
        let output_path = OutputLayout::module_output_path(
            package_dir,
            root_dir,
            target,
            module,
            FileType::TypeScriptDeclaration,
        )?;

        Ok(OutputFile {
            uri: Uri::from_path(&output_path),
            content: OutputContent::declaration(declaration_text.to_string()),
            source: None,
        })
    }

    /// Link one script artifact into output files.
    pub(crate) fn link_script_artifact_files(
        &self,
        module: &Module,
        artifact: &ScriptArtifact,
        target_id: &TargetId,
        target: &Target,
        package_dir: &Path,
        root_dir: Option<&Path>,
        context: &CompilerContext<'_>,
    ) -> Result<Vec<OutputFile>, String> {
        let mut entries = Vec::new();
        let file_types = linked_script_file_types(target)?;
        let source_map_path = file_types
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
            .transpose()?;

        // code files
        for file_type in &file_types {
            if *file_type == FileType::TypeScriptDeclaration {
                continue;
            }

            let output_path = OutputLayout::module_output_path(
                package_dir,
                root_dir,
                target,
                module,
                *file_type,
            )?;
            if *file_type == FileType::SourceMap {
                continue;
            }

            let files = self.link_printed_script_files(
                module,
                artifact,
                target_id,
                target,
                package_dir,
                *file_type,
                &output_path,
                source_map_path.as_deref(),
                context,
            )?;

            entries.extend(files);
        }

        // declarations
        if let Some(declaration) = &artifact.declaration
            && file_types.contains(&FileType::TypeScriptDeclaration)
        {
            let declaration = self.link_script_declaration_file(
                module,
                &declaration.text,
                target,
                package_dir,
                root_dir,
            )?;

            entries.push(declaration);
        }

        Ok(entries)
    }

    /// Link one final script text output and any related sidecars.
    pub(crate) fn link_script_text_files(
        &self,
        target: &Target,
        file_type: FileType,
        output_path: &Path,
        code: String,
        source_map: Option<SourceMapBuilder>,
        source_map_path: Option<&Path>,
    ) -> Result<Vec<OutputFile>, String> {
        let output_policy = ScriptTextOutputPolicy::new(target);
        let source_map_reference =
            source_map_path.map(|path| relative_source_map_reference(output_path, path));
        let shaped_code = output_policy.shape_script_text(code, file_type)?;
        let mut source_map = source_map;
        let has_source_map = source_map.is_some();

        // banner bytes shift every generated marker forward in the final output
        if let Some(source_map) = &mut source_map {
            source_map.prepend_generated_bytes(output_policy.script_banner_prefix_byte_count());
        }
        let source_map = source_map.map(|source_map| {
            source_map.build(
                &shaped_code,
                output_policy.source_map_annotation_line_count(
                    has_source_map,
                    source_map_reference.is_some(),
                ),
            )
        });

        let code = output_policy.annotate_script_text_with_source_map(
            shaped_code,
            source_map.as_ref(),
            source_map_reference,
        )?;
        let content = output_policy.script_content(file_type, code)?;
        let mut files = vec![OutputFile {
            uri: Uri::from_path(output_path),
            content,
            source: None,
        }];

        let Some(source_map) = source_map.as_ref() else {
            return Ok(files);
        };

        let Some(source_map_path) = source_map_path else {
            return Ok(files);
        };

        let content = OutputContent::source_map(source_map)
            .map_err(|error| format!("failed to serialize source map: {error}"))?;

        files.push(OutputFile {
            uri: Uri::from_path(source_map_path),
            content,
            source: None,
        });

        Ok(files)
    }
}

/// Build one relative source map reference from one output path.
fn relative_source_map_reference(output_path: &Path, source_map_path: &Path) -> String {
    let relative = relative_path_between(output_path, source_map_path);
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

/// Choose the linked file types for one script target.
fn linked_script_file_types(target: &Target) -> Result<Vec<FileType>, String> {
    match target.emit {
        EmitFormat::Js => {
            let mut file_types = vec![FileType::JavaScript];
            if target.declaration {
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
        EmitFormat::Html => Err("FUGU #Incomplete".to_string()),
        other => Err(format!("expected JS, TS, or HTML output, got {other:?}")),
    }
}
