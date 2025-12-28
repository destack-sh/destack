use std::num::NonZero;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use clap::{Args, ValueEnum};
use destack_resolver::{ResolveOptions, Resolver};
use destack_source::{FileSystem, IndentStyle, LineEnding, PhysicalFileSystem};
use destack_workspace::{
    ArrowParentheses, FormatterOptions, ImportSortOrder, LintPreset, LintSeverity, LinterOptions,
    OrganizeImports, QuoteProperty, QuoteStyle, Session, TrailingComma, TsConfigRegistry,
};

/// Get the default number of worker threads (available parallelism, or 1 if unknown).
pub fn default_workers() -> u16 {
    thread::available_parallelism()
        .unwrap_or(NonZero::new(1).unwrap())
        .get() as u16
}

/// The indent style to use.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum IndentStyleArg {
    /// Use tabs for indentation.
    #[default]
    Tab,
    /// Use spaces for indentation.
    Space,
}

impl From<IndentStyleArg> for IndentStyle {
    fn from(style: IndentStyleArg) -> Self {
        match style {
            IndentStyleArg::Tab => IndentStyle::Tab,
            IndentStyleArg::Space => IndentStyle::Space,
        }
    }
}

/// The line ending style to use.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum LineEndingArg {
    /// Unix-style line endings (LF).
    #[default]
    Lf,
    /// Windows-style line endings (CRLF).
    Crlf,
    /// Classic Mac-style line endings (CR).
    Cr,
}

impl From<LineEndingArg> for LineEnding {
    fn from(ending: LineEndingArg) -> Self {
        match ending {
            LineEndingArg::Lf => LineEnding::LineFeed,
            LineEndingArg::Crlf => LineEnding::CarriageReturnLineFeed,
            LineEndingArg::Cr => LineEnding::CarriageReturn,
        }
    }
}

/// Whether to organize imports.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum OrganizeImportsArg {
    /// Organize imports: sort statements by group and specifiers alphabetically.
    On,
    /// Don't reorder imports (preserve original order).
    #[default]
    Off,
}

impl From<OrganizeImportsArg> for OrganizeImports {
    fn from(value: OrganizeImportsArg) -> Self {
        match value {
            OrganizeImportsArg::On => OrganizeImports::On,
            OrganizeImportsArg::Off => OrganizeImports::Off,
        }
    }
}

/// Sort order for import specifiers.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ImportSortOrderArg {
    /// Natural sort: numbers ordered as integers (a1 < a2 < a10).
    #[default]
    Natural,
    /// Alphabetical/lexicographic sort (a1 < a10 < a2).
    Alphabetical,
}

impl From<ImportSortOrderArg> for ImportSortOrder {
    fn from(value: ImportSortOrderArg) -> Self {
        match value {
            ImportSortOrderArg::Natural => ImportSortOrder::Natural,
            ImportSortOrderArg::Alphabetical => ImportSortOrder::Alphabetical,
        }
    }
}

/// Quote style for string literals.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum QuoteStyleArg {
    /// Use double quotes for strings: `"hello"`.
    #[default]
    Double,
    /// Use single quotes for strings: `'hello'`.
    Single,
    /// Use single quotes for single characters, double quotes for strings.
    Semantic,
}

impl From<QuoteStyleArg> for QuoteStyle {
    fn from(value: QuoteStyleArg) -> Self {
        match value {
            QuoteStyleArg::Double => QuoteStyle::Double,
            QuoteStyleArg::Single => QuoteStyle::Single,
            QuoteStyleArg::Semantic => QuoteStyle::Semantic,
        }
    }
}

/// Trailing comma policy.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum TrailingCommaArg {
    /// Add trailing commas everywhere valid in ES2017+.
    #[default]
    All,
    /// Add trailing commas where valid in ES5 (not function params).
    Es5,
    /// Never add trailing commas.
    None,
}

impl From<TrailingCommaArg> for TrailingComma {
    fn from(value: TrailingCommaArg) -> Self {
        match value {
            TrailingCommaArg::All => TrailingComma::All,
            TrailingCommaArg::Es5 => TrailingComma::Es5,
            TrailingCommaArg::None => TrailingComma::None,
        }
    }
}

/// Arrow function parentheses policy.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ArrowParenthesesArg {
    /// Always include parentheses: `(x) => x`.
    #[default]
    Always,
    /// Omit parentheses when possible: `x => x`.
    Avoid,
}

impl From<ArrowParenthesesArg> for ArrowParentheses {
    fn from(value: ArrowParenthesesArg) -> Self {
        match value {
            ArrowParenthesesArg::Always => ArrowParentheses::Always,
            ArrowParenthesesArg::Avoid => ArrowParentheses::Avoid,
        }
    }
}

/// Object property quote style.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum QuotePropertyArg {
    /// Only quote properties when required.
    #[default]
    AsNeeded,
    /// Quote all properties consistently if any require quotes.
    Consistent,
    /// Preserve the original quoting from source.
    Preserve,
}

impl From<QuotePropertyArg> for QuoteProperty {
    fn from(value: QuotePropertyArg) -> Self {
        match value {
            QuotePropertyArg::AsNeeded => QuoteProperty::AsNeeded,
            QuotePropertyArg::Consistent => QuoteProperty::Consistent,
            QuotePropertyArg::Preserve => QuoteProperty::Preserve,
        }
    }
}

/// Lint rule preset.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum LintPresetArg {
    /// No rules enabled by default.
    None,
    /// Recommended rules enabled (default).
    #[default]
    Recommended,
    /// All rules enabled.
    All,
}

impl From<LintPresetArg> for LintPreset {
    fn from(value: LintPresetArg) -> Self {
        match value {
            LintPresetArg::None => LintPreset::None,
            LintPresetArg::Recommended => LintPreset::Recommended,
            LintPresetArg::All => LintPreset::All,
        }
    }
}

/// Arguments for configuring linter options.
#[derive(Args, Debug, Clone, Default)]
pub struct LinterOptionsArgs {
    /// Lint rule preset (none|recommended|all, default: recommended).
    #[arg(long = "lint-preset", value_enum)]
    pub preset: Option<LintPresetArg>,

    /// Allow specific lint rules (set to note severity).
    #[arg(long = "allow", value_name = "RULE")]
    pub allow: Vec<String>,

    /// Warn on specific lint rules (set to warning severity).
    #[arg(long = "warn", value_name = "RULE")]
    pub warn: Vec<String>,

    /// Deny specific lint rules (set to error severity).
    #[arg(long = "deny", value_name = "RULE")]
    pub deny: Vec<String>,

    /// Maximum cyclomatic complexity (default: 40).
    #[arg(long = "max-complexity")]
    pub max_complexity: Option<usize>,

    /// Maximum function parameters (default: 4).
    #[arg(long = "max-params")]
    pub max_params: Option<usize>,

    /// Maximum nesting depth (default: 4).
    #[arg(long = "max-depth")]
    pub max_depth: Option<usize>,

    /// Maximum lines per file (default: 500).
    #[arg(long = "max-lines")]
    pub max_lines: Option<usize>,
}

impl From<LinterOptionsArgs> for LinterOptions {
    fn from(args: LinterOptionsArgs) -> Self {
        let mut options = LinterOptions::default();
        if let Some(preset) = args.preset {
            options.preset = preset.into();
        }
        // apply rule overrides
        for rule in args.allow {
            options.overrides.insert(rule, LintSeverity::Note);
        }
        for rule in args.warn {
            options.overrides.insert(rule, LintSeverity::Warning);
        }
        for rule in args.deny {
            options.overrides.insert(rule, LintSeverity::Error);
        }
        // complexity thresholds
        if let Some(max_complexity) = args.max_complexity {
            options.max_cyclomatic_complexity = max_complexity;
        }
        if let Some(max_params) = args.max_params {
            options.max_params = max_params;
        }
        if let Some(max_depth) = args.max_depth {
            options.max_depth = max_depth;
        }
        if let Some(max_lines) = args.max_lines {
            options.max_lines = max_lines;
        }
        options
    }
}

/// Arguments for configuring formatter options.
#[derive(Args, Debug, Clone, Default)]
pub struct FormatterOptionsArgs {
    /// The indent style (tab|space, default: space).
    #[arg(long = "indent-style", value_enum)]
    pub indent_style: Option<IndentStyleArg>,

    /// The indent width in spaces (default: 4).
    #[arg(long = "indent-width")]
    pub indent_width: Option<u8>,

    /// The line ending style (lf|crlf|cr, default: lf).
    #[arg(long = "line-ending", value_enum)]
    pub line_ending: Option<LineEndingArg>,

    /// The maximum line width (default: 100).
    #[arg(long = "line-width")]
    pub line_width: Option<u16>,

    /// Quote style for strings (double|single|semantic, default: semantic).
    #[arg(long = "quote-style", value_enum)]
    pub quote_style: Option<QuoteStyleArg>,

    /// Trailing comma policy (all|es5|none, default: all).
    #[arg(long = "trailing-comma", value_enum)]
    pub trailing_comma: Option<TrailingCommaArg>,

    /// Include spaces inside object braces (default: true).
    #[arg(long = "bracket-spacing")]
    pub bracket_spacing: Option<bool>,

    /// Arrow function parentheses (always|avoid, default: always).
    #[arg(long = "arrow-parens", value_enum)]
    pub arrow_parens: Option<ArrowParenthesesArg>,

    /// Object property quoting (as-needed|consistent|preserve, default: as-needed).
    #[arg(long = "quote-props", value_enum)]
    pub quote_props: Option<QuotePropertyArg>,

    /// Put closing bracket on same line as last attribute in JSX/trees.
    #[arg(long = "bracket-same-line")]
    pub bracket_same_line: Option<bool>,

    /// Force each JSX/tree attribute onto its own line.
    #[arg(long = "single-attribute-per-line")]
    pub single_attribute_per_line: Option<bool>,

    /// Whether to organize imports (on|off, default: off).
    #[arg(long = "organize-imports", value_enum)]
    pub organize_imports: Option<OrganizeImportsArg>,

    /// Sort order for import specifiers (natural|alphabetical, default: natural).
    #[arg(long = "import-sort-order", value_enum)]
    pub import_sort_order: Option<ImportSortOrderArg>,
}

impl From<FormatterOptionsArgs> for FormatterOptions {
    fn from(args: FormatterOptionsArgs) -> Self {
        let mut options = FormatterOptions::default();
        // layout
        if let Some(style) = args.indent_style {
            options.indent_style = style.into();
        }
        if let Some(width) = args.indent_width {
            options.indent_width = width;
        }
        if let Some(ending) = args.line_ending {
            options.line_ending = ending.into();
        }
        if let Some(width) = args.line_width {
            options.line_width = width;
        }
        // syntax
        if let Some(quote_style) = args.quote_style {
            options.quote_style = quote_style.into();
        }
        if let Some(trailing_comma) = args.trailing_comma {
            options.trailing_comma = trailing_comma.into();
        }
        if let Some(bracket_spacing) = args.bracket_spacing {
            options.bracket_spacing = bracket_spacing;
        }
        if let Some(arrow_parens) = args.arrow_parens {
            options.arrow_parentheses = arrow_parens.into();
        }
        if let Some(quote_props) = args.quote_props {
            options.quote_property = quote_props.into();
        }
        // tree/jsx
        if let Some(bracket_same_line) = args.bracket_same_line {
            options.bracket_same_line = bracket_same_line;
        }
        if let Some(single_attribute_per_line) = args.single_attribute_per_line {
            options.single_attribute_per_line = single_attribute_per_line;
        }
        // imports
        if let Some(organize) = args.organize_imports {
            options.organize_imports = organize.into();
        }
        if let Some(sort_order) = args.import_sort_order {
            options.import_sort_order = sort_order.into();
        }
        options
    }
}

/// Arguments for configuring program setup.
#[derive(Args, Debug, Clone, Default)]
pub struct ProgramArgs {
    /// The working directory (default: current directory).
    #[arg(long = "cwd")]
    pub cwd: Option<PathBuf>,

    /// Path to dsconfig.json configuration file.
    #[arg(long = "config", short = 'c')]
    pub config: Option<PathBuf>,

    /// The number of worker threads to use (default: number of CPU cores).
    #[arg(long = "workers", short = 'j', default_value_t = default_workers())]
    pub workers: u16,

    /// Skip loading standard library types (es*, dom, etc.).
    #[arg(long = "no-libs", visible_alias = "no-lib")]
    pub no_libs: bool,

    /// Skip injecting prelude items (Add, Type, deprecated, etc.).
    #[arg(long = "no-prelude")]
    pub no_prelude: bool,

    /// Don't follow imports automatically.
    #[arg(long = "no-follow-imports")]
    pub no_follow_imports: bool,

    /// Libraries to load (e.g., es2020, dom, node). Overrides automatic detection.
    #[arg(long = "lib", value_delimiter = ',')]
    pub lib: Vec<String>,

    /// The formatter options.
    #[command(flatten)]
    pub formatter: FormatterOptionsArgs,

    /// The linter options.
    #[command(flatten)]
    pub linter: LinterOptionsArgs,
}

impl ProgramArgs {
    /// Create a Session from these arguments.
    pub fn setup(&self) -> Arc<Session> {
        let cwd = self
            .cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let formatter_options: FormatterOptions = self.formatter.clone().into();
        let linter_options: LinterOptions = self.linter.clone().into();
        let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());

        // create session (builtins are always loaded)
        let session = Session::new(cwd.clone())
            .with_fs(fs.clone())
            .with_formatter(formatter_options)
            .with_linter(linter_options);

        // create shared tsconfig registry for resolver
        let tsconfigs = Arc::new(TsConfigRegistry::new());

        // discover workspace
        let resolver = Resolver::new(
            session.fs.clone(),
            session.files.clone(),
            session.packages.clone(),
            tsconfigs.clone(),
            ResolveOptions::default(),
        );
        let workspace = resolver
            .discover_workspace(&cwd)
            .unwrap_or_else(|_| destack_workspace::Workspace::single_package(cwd.clone()));
        let root = workspace.root.clone();

        tracing::trace!(?cwd, ?root, workspace_kind = ?workspace.kind, workers = self.workers, "program.setup");

        // add the root to create a program
        session.add_root(root);

        Arc::new(session)
    }
}
