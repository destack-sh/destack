#![allow(clippy::derivable_impls)]

use napi_derive::napi;

use super::source::{IndentStyle, LineEnding};

/// The transpilation mode.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranspilerMode {
    /// Retain the original file structure.
    Retained,
    /// Combine all files.
    Combined,
}

impl Default for TranspilerMode {
    fn default() -> Self {
        Self::Retained
    }
}

impl From<TranspilerMode> for destack_codegen_js::TranspilerMode {
    fn from(mode: TranspilerMode) -> Self {
        match mode {
            TranspilerMode::Retained => destack_codegen_js::TranspilerMode::Retained,
            TranspilerMode::Combined => destack_codegen_js::TranspilerMode::Combined,
        }
    }
}

/// The target language for transpiling.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranspileTarget {
    /// Plain JavaScript (`.js`).
    JavaScript,
    /// TypeScript (`.ts`).
    TypeScript,
    /// Plain JavaScript with TypeScript declarations (.js and .d.ts).
    JavaScriptWithTypeScriptDeclarations,
}

impl Default for TranspileTarget {
    fn default() -> Self {
        Self::TypeScript
    }
}

impl From<TranspileTarget> for destack_codegen_js::TranspileTarget {
    fn from(target: TranspileTarget) -> Self {
        match target {
            TranspileTarget::JavaScript => destack_codegen_js::TranspileTarget::JavaScript,
            TranspileTarget::TypeScript => destack_codegen_js::TranspileTarget::TypeScript,
            TranspileTarget::JavaScriptWithTypeScriptDeclarations => {
                destack_codegen_js::TranspileTarget::JavaScriptWithTypeScriptDeclarations
            }
        }
    }
}

/// The target language for transpiling.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranspilerLanguage {
    /// Plain JavaScript (like `.js`).
    JavaScript,
    /// TypeScript (like `.ts`).
    TypeScript,
    /// TypeScript declarations (like `.d.ts`).
    TypeScriptDeclaration,
}

impl Default for TranspilerLanguage {
    fn default() -> Self {
        Self::TypeScript
    }
}

impl From<TranspilerLanguage> for destack_codegen_js::TranspilerLanguage {
    fn from(language: TranspilerLanguage) -> Self {
        match language {
            TranspilerLanguage::JavaScript => destack_codegen_js::TranspilerLanguage::JavaScript,
            TranspilerLanguage::TypeScript => destack_codegen_js::TranspilerLanguage::TypeScript,
            TranspilerLanguage::TypeScriptDeclaration => {
                destack_codegen_js::TranspilerLanguage::TypeScriptDeclaration
            }
        }
    }
}

/// The ECMAScript level.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcmaScriptVersion {
    /// ECMAScript 2022.
    ES2022,
}

impl Default for EcmaScriptVersion {
    fn default() -> Self {
        Self::ES2022
    }
}

impl From<EcmaScriptVersion> for destack_codegen_js::EcmaScriptVersion {
    fn from(version: EcmaScriptVersion) -> Self {
        match version {
            EcmaScriptVersion::ES2022 => destack_codegen_js::EcmaScriptVersion::ES2022,
        }
    }
}

/// The TypeScript version.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeScriptVersion {
    /// TypeScript 5.0.
    TS5_0,
}

impl Default for TypeScriptVersion {
    fn default() -> Self {
        Self::TS5_0
    }
}

impl From<TypeScriptVersion> for destack_codegen_js::TypeScriptVersion {
    fn from(version: TypeScriptVersion) -> Self {
        match version {
            TypeScriptVersion::TS5_0 => destack_codegen_js::TypeScriptVersion::TS5_0,
        }
    }
}

/// The formatting mode.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatMode {
    /// Pretty.
    Pretty,
    /// Minimal.
    Minimal,
}

impl Default for FormatMode {
    fn default() -> Self {
        Self::Pretty
    }
}

impl From<FormatMode> for destack_codegen_js::FormatMode {
    fn from(mode: FormatMode) -> Self {
        match mode {
            FormatMode::Pretty => destack_codegen_js::FormatMode::Pretty,
            FormatMode::Minimal => destack_codegen_js::FormatMode::Minimal,
        }
    }
}

/// The JavaScript format options.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct FormatOptions {
    /// The formatting mode.
    pub mode: FormatMode,
    /// The language target.
    pub language: TranspilerLanguage,
    /// The type of line ending to apply to the printed input.
    pub line_ending: LineEnding,
    /// The indent style.
    pub indent_style: IndentStyle,
    /// Spaces per indent.
    pub indent_width: u8,
    /// Maximum line length (best effort).
    pub line_width: u8,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            mode: FormatMode::Pretty,
            language: TranspilerLanguage::JavaScript,
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
        }
    }
}

impl From<FormatOptions> for destack_codegen_js::JavaScriptFormatOptions {
    fn from(options: FormatOptions) -> Self {
        Self {
            mode: options.mode.into(),
            language: options.language.into(),
            line_ending: options.line_ending.into(),
            indent_style: options.indent_style.into(),
            indent_width: options.indent_width,
            line_width: options.line_width,
        }
    }
}

/// The transpilation options.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct TranspileOptions {
    /// The transpilation mode.
    pub mode: TranspilerMode,
    /// The target language.
    pub target: TranspileTarget,
    /// The ECMAScript level.
    pub es_version: EcmaScriptVersion,
    /// The TypeScript version.
    pub ts_version: TypeScriptVersion,
    /// The formatting options.
    pub formatting: FormatOptions,
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self {
            mode: TranspilerMode::Retained,
            target: TranspileTarget::TypeScript,
            es_version: EcmaScriptVersion::ES2022,
            ts_version: TypeScriptVersion::TS5_0,
            formatting: FormatOptions::default(),
        }
    }
}

impl From<TranspileOptions> for destack_codegen_js::TranspileOptions {
    fn from(options: TranspileOptions) -> Self {
        Self {
            diagnostic: destack_source::DiagnosticOptions::default(),
            workers: destack_codegen_js::default_workers(),
            mode: options.mode.into(),
            target: options.target.into(),
            es_version: options.es_version.into(),
            ts_version: options.ts_version.into(),
            formatting: options.formatting.into(),
        }
    }
}

/// Get the default transpiler options.
#[napi(js_name = "defaultTranspileOptions")]
pub fn default_transpiler_options() -> TranspileOptions {
    TranspileOptions::default()
}

/// The result of transpiling a file.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct TranspileResult {
    /// The transpiled code.
    pub code: String,
    /// The source map (if generated).
    pub source_map: Option<String>,
    /// Any diagnostics/warnings.
    pub diagnostics: Vec<String>,
}

/// Transpile Destack source code to TypeScript/JavaScript.
/// This is a stateless function for simple one-off transpilation.
#[napi(js_name = "transpileSource")]
pub fn transpile_source(
    content: String,
    options: Option<TranspileOptions>,
) -> napi::Result<TranspileResult> {
    transpile_file_impl("<source>".to_string(), content, options)
}

/// Transpile a Destack file to TypeScript/JavaScript.
/// This is a stateless function for simple one-off transpilation.
#[napi(js_name = "transpileFile")]
pub fn transpile_file(
    path: String,
    content: String,
    options: Option<TranspileOptions>,
) -> napi::Result<TranspileResult> {
    transpile_file_impl(path, content, options)
}

/// Internal implementation of file transpilation.
/// TODO #Architecture: use a persistent daemon/workspace across transpile files?
fn transpile_file_impl(
    path: String,
    content: String,
    options: Option<TranspileOptions>,
) -> napi::Result<TranspileResult> {
    use std::path::PathBuf;
    use std::sync::Arc;

    use destack_codegen_js::Transpiler;
    use destack_compiler::{CompileOptions, Compiler, ImportTask};
    use destack_source::{
        DiagnosticSeverity, FileRegistry, FileType, LanguageOptions, PhysicalFileSystem, Uri,
    };
    use destack_workspace::Program;

    let options = options.unwrap_or_default();
    let transpile_options: destack_codegen_js::TranspileOptions = options.into();

    // set up the program
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let fs: Arc<dyn destack_source::FileSystem> = Arc::new(PhysicalFileSystem);
    let files = Arc::new(FileRegistry::new());
    let program = Arc::new(Program::new(
        LanguageOptions::default(),
        cwd,
        fs,
        files.clone(),
    ));

    // register module with inline content on program
    let uri = Uri::from_string(&path);
    let module_id = program.register_inline_module(uri.clone(), content, FileType::Destack);

    // compile the file
    let compile_options = CompileOptions::default();
    let compiler = Compiler::new(program.clone(), compile_options);
    compiler.enqueue(ImportTask::ImportModule { module: module_id });
    compiler.compile();

    // collect any diagnostics
    let diagnostic_list = program.diagnostics.iter();
    let diagnostics: Vec<String> = diagnostic_list.iter().map(|d| d.message.clone()).collect();

    // check for errors
    let has_errors = program
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error);
    if has_errors {
        return Err(napi::Error::from_reason(format!(
            "compilation failed with {} error(s): {}",
            diagnostics.len(),
            diagnostics.join("; ")
        )));
    }

    // create the transpiler and transpile
    let transpiler = Transpiler::new(program.clone(), transpile_options);
    transpiler.transpile();

    // get the artifact for our file
    let artifact_uri = uri.without_extension().with_extension("ts");
    let artifact = transpiler.artifacts.get(&artifact_uri);
    let code = match artifact {
        Some(artifact) => match &artifact.content {
            destack_source::FileContent::Text { content } => content.clone(),
            _ => {
                return Err(napi::Error::from_reason(
                    "Artifact is not text content".to_string(),
                ));
            }
        },
        None => {
            // try without extension change for combined mode
            let artifact = transpiler.artifacts.iter().next();
            match artifact {
                Some(entry) => match &entry.value().content {
                    destack_source::FileContent::Text { content } => content.clone(),
                    _ => {
                        return Err(napi::Error::from_reason(
                            "Artifact is not text content".to_string(),
                        ));
                    }
                },
                None => {
                    return Err(napi::Error::from_reason(
                        "No transpiled artifact found".to_string(),
                    ));
                }
            }
        }
    };

    Ok(TranspileResult {
        code,
        source_map: None,
        diagnostics,
    })
}
