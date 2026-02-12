use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use napi::Error;
use napi_derive::napi;
use {
    destack_compiler as compiler, destack_workspace as workspace,
    destack_workspace_service as workspace_service,
};

use super::{CompilerOptions, Diagnostic, diagnostics_have_errors, diagnostics_have_warnings};

/// Linter preset.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinterPreset {
    /// No rules enabled by default.
    None,
    /// Recommended rules enabled.
    Recommended,
    /// All rules enabled.
    All,
}

impl From<LinterPreset> for workspace::LintPreset {
    fn from(preset: LinterPreset) -> Self {
        match preset {
            LinterPreset::None => workspace::LintPreset::None,
            LinterPreset::Recommended => workspace::LintPreset::Recommended,
            LinterPreset::All => workspace::LintPreset::All,
        }
    }
}

impl From<workspace::LintPreset> for LinterPreset {
    fn from(preset: workspace::LintPreset) -> Self {
        match preset {
            workspace::LintPreset::None => LinterPreset::None,
            workspace::LintPreset::Recommended => LinterPreset::Recommended,
            workspace::LintPreset::All => LinterPreset::All,
        }
    }
}

/// Linter category.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinterCategory {
    /// Correctness category.
    Correctness,
    /// Suspicious category.
    Suspicious,
    /// Performance category.
    Performance,
    /// Style category.
    Style,
    /// Security category.
    Security,
    /// Complexity category.
    Complexity,
    /// Restriction category.
    Restriction,
}

impl From<LinterCategory> for workspace::LintCategory {
    fn from(category: LinterCategory) -> Self {
        match category {
            LinterCategory::Correctness => workspace::LintCategory::Correctness,
            LinterCategory::Suspicious => workspace::LintCategory::Suspicious,
            LinterCategory::Performance => workspace::LintCategory::Performance,
            LinterCategory::Style => workspace::LintCategory::Style,
            LinterCategory::Security => workspace::LintCategory::Security,
            LinterCategory::Complexity => workspace::LintCategory::Complexity,
            LinterCategory::Restriction => workspace::LintCategory::Restriction,
        }
    }
}

impl From<workspace::LintCategory> for LinterCategory {
    fn from(category: workspace::LintCategory) -> Self {
        match category {
            workspace::LintCategory::Correctness => LinterCategory::Correctness,
            workspace::LintCategory::Suspicious => LinterCategory::Suspicious,
            workspace::LintCategory::Performance => LinterCategory::Performance,
            workspace::LintCategory::Style => LinterCategory::Style,
            workspace::LintCategory::Security => LinterCategory::Security,
            workspace::LintCategory::Complexity => LinterCategory::Complexity,
            workspace::LintCategory::Restriction => LinterCategory::Restriction,
        }
    }
}

/// Linter severity.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinterSeverity {
    /// Rule is disabled.
    Off,
    /// Rule produces notes.
    Note,
    /// Rule produces warnings.
    Warning,
    /// Rule produces errors.
    Error,
}

impl From<LinterSeverity> for workspace::LintSeverity {
    fn from(severity: LinterSeverity) -> Self {
        match severity {
            LinterSeverity::Off => workspace::LintSeverity::Off,
            LinterSeverity::Note => workspace::LintSeverity::Note,
            LinterSeverity::Warning => workspace::LintSeverity::Warning,
            LinterSeverity::Error => workspace::LintSeverity::Error,
        }
    }
}

impl From<workspace::LintSeverity> for LinterSeverity {
    fn from(severity: workspace::LintSeverity) -> Self {
        match severity {
            workspace::LintSeverity::Off => LinterSeverity::Off,
            workspace::LintSeverity::Note => LinterSeverity::Note,
            workspace::LintSeverity::Warning => LinterSeverity::Warning,
            workspace::LintSeverity::Error => LinterSeverity::Error,
        }
    }
}

/// Preferred array type syntax.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinterArrayTypeStyle {
    /// Prefer T[] syntax.
    Array,
    /// Prefer Array<T> syntax.
    Generic,
}

impl From<LinterArrayTypeStyle> for workspace::ArrayTypeStyle {
    fn from(style: LinterArrayTypeStyle) -> Self {
        match style {
            LinterArrayTypeStyle::Array => workspace::ArrayTypeStyle::Array,
            LinterArrayTypeStyle::Generic => workspace::ArrayTypeStyle::Generic,
        }
    }
}

impl From<workspace::ArrayTypeStyle> for LinterArrayTypeStyle {
    fn from(style: workspace::ArrayTypeStyle) -> Self {
        match style {
            workspace::ArrayTypeStyle::Array => LinterArrayTypeStyle::Array,
            workspace::ArrayTypeStyle::Generic => LinterArrayTypeStyle::Generic,
        }
    }
}

/// Preferred type definition syntax.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinterTypeDefinitionStyle {
    /// Prefer type aliases.
    Type,
    /// Prefer interface declarations.
    Interface,
}

impl From<LinterTypeDefinitionStyle> for workspace::TypeDefinitionStyle {
    fn from(style: LinterTypeDefinitionStyle) -> Self {
        match style {
            LinterTypeDefinitionStyle::Type => workspace::TypeDefinitionStyle::Type,
            LinterTypeDefinitionStyle::Interface => workspace::TypeDefinitionStyle::Interface,
        }
    }
}

impl From<workspace::TypeDefinitionStyle> for LinterTypeDefinitionStyle {
    fn from(style: workspace::TypeDefinitionStyle) -> Self {
        match style {
            workspace::TypeDefinitionStyle::Type => LinterTypeDefinitionStyle::Type,
            workspace::TypeDefinitionStyle::Interface => LinterTypeDefinitionStyle::Interface,
        }
    }
}

/// Filename case style.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinterFilenameCase {
    /// kebab case style.
    Kebab,
    /// snake case style.
    Snake,
    /// camel case style.
    Camel,
    /// pascal case style.
    Pascal,
}

impl From<LinterFilenameCase> for workspace::FilenameCase {
    fn from(style: LinterFilenameCase) -> Self {
        match style {
            LinterFilenameCase::Kebab => workspace::FilenameCase::Kebab,
            LinterFilenameCase::Snake => workspace::FilenameCase::Snake,
            LinterFilenameCase::Camel => workspace::FilenameCase::Camel,
            LinterFilenameCase::Pascal => workspace::FilenameCase::Pascal,
        }
    }
}

impl From<workspace::FilenameCase> for LinterFilenameCase {
    fn from(style: workspace::FilenameCase) -> Self {
        match style {
            workspace::FilenameCase::Kebab => LinterFilenameCase::Kebab,
            workspace::FilenameCase::Snake => LinterFilenameCase::Snake,
            workspace::FilenameCase::Camel => LinterFilenameCase::Camel,
            workspace::FilenameCase::Pascal => LinterFilenameCase::Pascal,
        }
    }
}

/// Category level severity override.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct LinterCategoryOverride {
    /// Category to override.
    pub category: LinterCategory,
    /// Severity for this category.
    pub severity: LinterSeverity,
}

/// Rule level severity override.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct LinterRuleOverride {
    /// Rule id to override.
    pub rule: String,
    /// Severity for this rule.
    pub severity: LinterSeverity,
}

/// Linter rule and threshold options.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct LinterOptions {
    /// Whether linting is enabled.
    pub enabled: bool,
    /// Base preset.
    pub preset: LinterPreset,
    /// Category level severity overrides.
    pub categories: Vec<LinterCategoryOverride>,
    /// Individual rule severity overrides.
    pub overrides: Vec<LinterRuleOverride>,
    /// Include declaration files when evaluating declaration gated rules.
    pub include_declaration_files: bool,
    /// Allow explicit void to intentionally discard promise results.
    pub allow_void_discard: bool,
    /// Check callback positions in no-misused-promises.
    pub check_misused_promises_in_callbacks: bool,
    /// Check conditionals in no-misused-promises.
    pub check_misused_promises_in_conditionals: bool,
    /// Parameter name prefixes ignored by no-unused-parameters.
    pub ignored_unused_parameter_prefixes: Vec<String>,
    /// Maximum boolean parameters or fields.
    pub max_booleans: u32,
    /// Maximum branches in a single conditional.
    pub max_branching_factor: u32,
    /// Maximum cognitive complexity.
    pub max_cognitive_complexity: u32,
    /// Maximum cyclomatic complexity.
    pub max_cyclomatic_complexity: u32,
    /// Maximum nesting depth.
    pub max_depth: u32,
    /// Maximum static parameters.
    pub max_static_params: u32,
    /// Maximum lines per file.
    pub max_lines: u32,
    /// Maximum lines per function.
    pub max_lines_per_function: u32,
    /// Maximum callback nesting.
    pub max_nested_callbacks: u32,
    /// Maximum function parameters.
    pub max_params: u32,
    /// Maximum statements per function.
    pub max_statements: u32,
    /// Maximum return statements per function.
    pub max_return_statements: u32,
    /// Maximum switch cases per switch statement.
    pub max_switch_cases: u32,
    /// Maximum variants in a union type or enum.
    pub max_type_variants: u32,
    /// Maximum fields in a struct, class, or interface.
    pub max_type_fields: u32,
    /// Maximum type complexity.
    pub max_type_complexity: u32,
    /// Maximum occurrences of the same string literal before warning.
    pub max_duplicate_string_occurrences: u32,
    /// Minimum lines required for duplicate code checks.
    pub min_duplicate_code_lines: u32,
    /// Minimum tokens required for duplicate code checks.
    pub min_duplicate_code_tokens: u32,
    /// Minimum similarity percent for near duplicate checks.
    pub min_duplicate_code_near_similarity: u8,
    /// Maximum statements in a try block.
    pub max_try_block_statements: u32,
    /// Preferred array type syntax.
    pub array_type: LinterArrayTypeStyle,
    /// Preferred type definition syntax.
    pub type_definition_style: LinterTypeDefinitionStyle,
    /// Required catch clause error name.
    pub catch_error_name: String,
    /// Required filename case style.
    pub filename_case: LinterFilenameCase,
    /// Allowed uppercase keyword prefixes for inline comments.
    pub comment_keywords: Vec<String>,
    /// Allowed tags for keyword comments.
    pub comment_keyword_tags: Vec<String>,
    /// Minimum non empty lines required for separator heading comments.
    pub comment_separator_heading_min_lines: u32,
    /// Magic numbers to allow.
    pub allowed_magic_numbers: Vec<f64>,
    /// Globals to restrict.
    pub restricted_globals: Vec<String>,
    /// Import paths to restrict.
    pub restricted_imports: Vec<String>,
    /// Comment terms to warn on.
    pub warning_comment_terms: Vec<String>,
}

impl Default for LinterOptions {
    fn default() -> Self {
        workspace::LinterOptions::default().into()
    }
}

impl From<workspace::LinterOptions> for LinterOptions {
    fn from(options: workspace::LinterOptions) -> Self {
        Self {
            enabled: options.enabled,
            preset: options.preset.into(),
            categories: options
                .categories
                .into_iter()
                .map(|(category, severity)| LinterCategoryOverride {
                    category: category.into(),
                    severity: severity.into(),
                })
                .collect(),
            overrides: options
                .overrides
                .into_iter()
                .map(|(rule, severity)| LinterRuleOverride {
                    rule,
                    severity: severity.into(),
                })
                .collect(),
            include_declaration_files: options.include_declaration_files,
            allow_void_discard: options.allow_void_discard,
            check_misused_promises_in_callbacks: options.check_misused_promises_in_callbacks,
            check_misused_promises_in_conditionals: options.check_misused_promises_in_conditionals,
            ignored_unused_parameter_prefixes: options.ignored_unused_parameter_prefixes,
            max_booleans: saturating_u32_from_usize(options.max_booleans),
            max_branching_factor: saturating_u32_from_usize(options.max_branching_factor),
            max_cognitive_complexity: saturating_u32_from_usize(options.max_cognitive_complexity),
            max_cyclomatic_complexity: saturating_u32_from_usize(options.max_cyclomatic_complexity),
            max_depth: saturating_u32_from_usize(options.max_depth),
            max_static_params: saturating_u32_from_usize(options.max_static_params),
            max_lines: saturating_u32_from_usize(options.max_lines),
            max_lines_per_function: saturating_u32_from_usize(options.max_lines_per_function),
            max_nested_callbacks: saturating_u32_from_usize(options.max_nested_callbacks),
            max_params: saturating_u32_from_usize(options.max_params),
            max_statements: saturating_u32_from_usize(options.max_statements),
            max_return_statements: saturating_u32_from_usize(options.max_return_statements),
            max_switch_cases: saturating_u32_from_usize(options.max_switch_cases),
            max_type_variants: saturating_u32_from_usize(options.max_type_variants),
            max_type_fields: saturating_u32_from_usize(options.max_type_fields),
            max_type_complexity: saturating_u32_from_usize(options.max_type_complexity),
            max_duplicate_string_occurrences: saturating_u32_from_usize(
                options.max_duplicate_string_occurrences,
            ),
            min_duplicate_code_lines: saturating_u32_from_usize(options.min_duplicate_code_lines),
            min_duplicate_code_tokens: saturating_u32_from_usize(options.min_duplicate_code_tokens),
            min_duplicate_code_near_similarity: options.min_duplicate_code_near_similarity,
            max_try_block_statements: saturating_u32_from_usize(options.max_try_block_statements),
            array_type: options.array_type.into(),
            type_definition_style: options.type_definition_style.into(),
            catch_error_name: options.catch_error_name,
            filename_case: options.filename_case.into(),
            comment_keywords: options.comment_keywords,
            comment_keyword_tags: options.comment_keyword_tags,
            comment_separator_heading_min_lines: saturating_u32_from_usize(
                options.comment_separator_heading_min_lines,
            ),
            allowed_magic_numbers: options.allowed_magic_numbers,
            restricted_globals: options.restricted_globals,
            restricted_imports: options.restricted_imports,
            warning_comment_terms: options.warning_comment_terms,
        }
    }
}

impl From<LinterOptions> for workspace::LinterOptions {
    fn from(options: LinterOptions) -> Self {
        let mut core = workspace::LinterOptions::default();
        core.enabled = options.enabled;
        core.preset = options.preset.into();
        core.categories = options
            .categories
            .into_iter()
            .map(|entry| (entry.category.into(), entry.severity.into()))
            .collect();
        core.overrides = options
            .overrides
            .into_iter()
            .map(|entry| (entry.rule, entry.severity.into()))
            .collect();
        core.include_declaration_files = options.include_declaration_files;
        core.allow_void_discard = options.allow_void_discard;
        core.check_misused_promises_in_callbacks = options.check_misused_promises_in_callbacks;
        core.check_misused_promises_in_conditionals =
            options.check_misused_promises_in_conditionals;
        core.ignored_unused_parameter_prefixes = options.ignored_unused_parameter_prefixes;
        core.max_booleans = options.max_booleans as usize;
        core.max_branching_factor = options.max_branching_factor as usize;
        core.max_cognitive_complexity = options.max_cognitive_complexity as usize;
        core.max_cyclomatic_complexity = options.max_cyclomatic_complexity as usize;
        core.max_depth = options.max_depth as usize;
        core.max_static_params = options.max_static_params as usize;
        core.max_lines = options.max_lines as usize;
        core.max_lines_per_function = options.max_lines_per_function as usize;
        core.max_nested_callbacks = options.max_nested_callbacks as usize;
        core.max_params = options.max_params as usize;
        core.max_statements = options.max_statements as usize;
        core.max_return_statements = options.max_return_statements as usize;
        core.max_switch_cases = options.max_switch_cases as usize;
        core.max_type_variants = options.max_type_variants as usize;
        core.max_type_fields = options.max_type_fields as usize;
        core.max_type_complexity = options.max_type_complexity as usize;
        core.max_duplicate_string_occurrences = options.max_duplicate_string_occurrences as usize;
        core.min_duplicate_code_lines = options.min_duplicate_code_lines as usize;
        core.min_duplicate_code_tokens = options.min_duplicate_code_tokens as usize;
        core.min_duplicate_code_near_similarity = options.min_duplicate_code_near_similarity;
        core.max_try_block_statements = options.max_try_block_statements as usize;
        core.array_type = options.array_type.into();
        core.type_definition_style = options.type_definition_style.into();
        core.catch_error_name = options.catch_error_name;
        core.filename_case = options.filename_case.into();
        core.comment_keywords = options.comment_keywords;
        core.comment_keyword_tags = options.comment_keyword_tags;
        core.comment_separator_heading_min_lines =
            options.comment_separator_heading_min_lines as usize;
        core.allowed_magic_numbers = options.allowed_magic_numbers;
        core.restricted_globals = options.restricted_globals;
        core.restricted_imports = options.restricted_imports;
        core.warning_comment_terms = options.warning_comment_terms;
        core
    }
}

/// Request options for lint operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct LinterRequestOptions {
    /// The current working directory.
    pub cwd: String,
    /// The workspace roots to open initially.
    pub roots: Vec<String>,
    /// Compiler options for lint diagnostics.
    pub compiler: CompilerOptions,
    /// Linter options for lint diagnostics.
    pub linter: LinterOptions,
}

impl Default for LinterRequestOptions {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            roots: Vec::new(),
            compiler: CompilerOptions::default(),
            linter: LinterOptions::default(),
        }
    }
}

/// Result payload for lint operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct LinterResult {
    /// Diagnostics emitted by this lint pass.
    pub diagnostics: Vec<Diagnostic>,
    /// Number of diagnostics.
    pub diagnostic_count: u32,
    /// Whether diagnostics include at least one error.
    pub has_errors: bool,
    /// Whether diagnostics include at least one warning.
    pub has_warnings: bool,
}

/// Get the default linter options.
#[napi(js_name = "defaultLinterOptions")]
pub fn default_linter_options() -> LinterOptions {
    LinterOptions::default()
}

/// Get the default lint request options.
#[napi(js_name = "defaultLinterRequestOptions")]
pub fn default_linter_request_options() -> LinterRequestOptions {
    LinterRequestOptions::default()
}

/// Lint a file synchronously.
#[napi(js_name = "lintSync")]
pub fn lint_sync(
    path: String,
    content: String,
    options: Option<LinterRequestOptions>,
) -> napi::Result<LinterResult> {
    // normalize inputs
    let options = options.unwrap_or_default();
    let path = PathBuf::from(path);

    // build workspace service with linter session options
    let cwd = PathBuf::from(&options.cwd);
    let roots = lint_roots_from_options(&options, &cwd);
    let session = Arc::new(workspace::Session::new(cwd).with_linter(options.linter.into()));
    let service =
        workspace_service::WorkspaceService::with_options(session, roots, options.compiler.into())
            .map_err(napi_error_from_lint)?;

    // collect update diagnostics
    let update_result = service
        .update_virtual_file(&path, content)
        .map_err(napi_error_from_lint)?;
    let mut diagnostics = lint_diagnostics_from_workspace_result(update_result);

    // collect module lint diagnostics
    let lint_diagnostics = service
        .with_program_for_path(&path, |program, compiler| {
            lint_module_diagnostics(&path, &program, &compiler)
        })
        .map_err(Error::from_reason)?;
    diagnostics.extend(lint_diagnostics);

    // dedupe repeated diagnostics
    diagnostics = dedupe_lint_diagnostics(diagnostics);

    // assemble response
    Ok(LinterResult {
        diagnostic_count: diagnostics.len() as u32,
        has_errors: diagnostics_have_errors(&diagnostics),
        has_warnings: diagnostics_have_warnings(&diagnostics),
        diagnostics,
    })
}

/// Build workspace roots for a lint request.
fn lint_roots_from_options(options: &LinterRequestOptions, cwd: &Path) -> Vec<PathBuf> {
    if options.roots.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        options.roots.iter().map(PathBuf::from).collect()
    }
}

/// Run linter compilation for the module owning a path.
fn lint_module_diagnostics(
    path: &Path,
    program: &workspace::Program,
    compiler: &compiler::Compiler,
) -> Result<Vec<Diagnostic>, String> {
    // resolve module and profile ids
    let module_id = compiler
        .resolve_path_to_module(&path.to_path_buf())
        .map_err(|error| format!("failed to resolve module for '{}': {error}", path.display()))?;
    let profile_id = program.default_profile_id_for_module(module_id);

    // enqueue lint task and compile
    let _ = program.diagnostics.drain();
    let module = compiler.module_stamp(module_id);
    let profile = compiler.profile_stamp(profile_id);
    compiler.enqueue(compiler::LintTask::LintModule { module, profile });
    compiler.compile();

    // collect diagnostics
    let diagnostics = program.diagnostics.collect();
    Ok(diagnostics.iter().into_iter().map(Into::into).collect())
}

/// Collect diagnostics from workspace update records.
fn lint_diagnostics_from_workspace_result(
    result: workspace_service::WorkspaceServiceResult,
) -> Vec<Diagnostic> {
    result
        .updates
        .into_iter()
        .flat_map(|update| update.diagnostics)
        .map(Into::into)
        .collect()
}

/// Remove duplicate diagnostics by stable location and message keys.
fn dedupe_lint_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for diagnostic in diagnostics {
        let key = (
            diagnostic.code.clone(),
            diagnostic.file_id,
            diagnostic.primary_span.span.start,
            diagnostic.primary_span.span.end,
            diagnostic.message.clone(),
        );

        if seen.insert(key) {
            deduped.push(diagnostic);
        }
    }

    deduped
}

/// Convert `usize` values into a saturating `u32`.
fn saturating_u32_from_usize(value: usize) -> u32 {
    if value > u32::MAX as usize {
        u32::MAX
    } else {
        value as u32
    }
}

/// Convert lint errors into NAPI errors.
fn napi_error_from_lint(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}
