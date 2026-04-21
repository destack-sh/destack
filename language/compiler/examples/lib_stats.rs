use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use clap::Parser;

use destack_ast::{
    Annotation, Argument, Declaration, Declarator, Decorator, DependencyItem, EnumField,
    Expression, MatchCase, Member, Parameter, Pattern, PatternField, Property, WhereClause,
};
use destack_builtin::{BuiltinLibrary, BuiltinLibraryKind, builtin_library};
use destack_parser::Parser as DestackParser;
use destack_source::{File, FileId, FileType, LanguageType, Uri};

const COLUMN_GAP: &str = "   ";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";

/// Command line arguments.
#[derive(Parser)]
#[command(name = "lib_stats", about = "Builtin lib parse stats")]
struct Args {
    /// Restrict the lib set, comma delimited.
    #[arg(long, value_delimiter = ',')]
    libs: Vec<String>,
    /// Print per module breakdowns.
    #[arg(long)]
    modules: bool,
    /// Limit per module output to the top N by lines.
    #[arg(long, default_value_t = 20)]
    module_top: usize,
    /// Disable ANSI color output.
    #[arg(long)]
    no_color: bool,
}

/// Output style controls.
#[derive(Clone, Copy)]
struct Style {
    /// Bold text start.
    bold: &'static str,
    /// Dim text start.
    dim: &'static str,
    /// Cyan text start.
    cyan: &'static str,
    /// Reset ANSI styling.
    reset: &'static str,
}

/// Aggregate parse counts.
#[derive(Debug, Default, Clone)]
struct Counts {
    /// Total expression nodes.
    expressions: usize,
    /// Total declaration nodes.
    declarations: usize,
    /// Total interface declarations.
    interfaces: usize,
    /// Total namespace declarations.
    namespaces: usize,
    /// Total annotation nodes.
    annotations: usize,
    /// Total member nodes.
    members: usize,
    /// Total property nodes.
    properties: usize,
    /// Total parameter nodes.
    parameters: usize,
    /// Total argument nodes.
    arguments: usize,
    /// Total pattern nodes.
    patterns: usize,
    /// Total pattern field nodes.
    pattern_fields: usize,
    /// Total declarator nodes.
    declarators: usize,
    /// Total enum field nodes.
    enum_fields: usize,
    /// Total match case nodes.
    match_cases: usize,
    /// Total where clause nodes.
    where_clauses: usize,
    /// Total dependency item nodes.
    dependency_items: usize,
    /// Total type declarations.
    type_decls: usize,
    /// Total enum declarations.
    enum_decls: usize,
    /// Total struct declarations.
    struct_decls: usize,
    /// Total class declarations.
    class_decls: usize,
    /// Total extension declarations.
    extension_decls: usize,
    /// Total function declarations.
    function_decls: usize,
    /// Total expression type literals.
    expr_type_literals: usize,
    /// Total template expressions.
    expr_templates: usize,
    /// Total tagged template expressions.
    expr_tagged_templates: usize,
    /// Total import expressions.
    expr_imports: usize,
    /// Total export expressions.
    expr_exports: usize,
    /// Total let expressions.
    expr_lets: usize,
    /// Total using expressions.
    expr_usings: usize,
    /// Total call expressions.
    expr_calls: usize,
    /// Total member expressions.
    expr_members: usize,
    /// Total index expressions.
    expr_indexes: usize,
    /// Total new expressions.
    expr_news: usize,
}

impl Counts {
    /// Add another counts instance to this one.
    fn add(&mut self, other: &Counts) {
        // aggregate node counts
        self.expressions += other.expressions;
        self.declarations += other.declarations;
        self.interfaces += other.interfaces;
        self.namespaces += other.namespaces;
        self.annotations += other.annotations;
        self.members += other.members;
        self.properties += other.properties;
        self.parameters += other.parameters;
        self.arguments += other.arguments;
        self.patterns += other.patterns;
        self.pattern_fields += other.pattern_fields;
        self.declarators += other.declarators;
        self.enum_fields += other.enum_fields;
        self.match_cases += other.match_cases;
        self.where_clauses += other.where_clauses;
        self.dependency_items += other.dependency_items;

        // aggregate declaration counts
        self.type_decls += other.type_decls;
        self.enum_decls += other.enum_decls;
        self.struct_decls += other.struct_decls;
        self.class_decls += other.class_decls;
        self.extension_decls += other.extension_decls;
        self.function_decls += other.function_decls;

        // aggregate expression shape counts
        self.expr_type_literals += other.expr_type_literals;
        self.expr_templates += other.expr_templates;
        self.expr_tagged_templates += other.expr_tagged_templates;
        self.expr_imports += other.expr_imports;
        self.expr_exports += other.expr_exports;
        self.expr_lets += other.expr_lets;
        self.expr_usings += other.expr_usings;
        self.expr_calls += other.expr_calls;
        self.expr_members += other.expr_members;
        self.expr_indexes += other.expr_indexes;
        self.expr_news += other.expr_news;
    }
}

/// Aggregate parse stats for a lib group.
#[derive(Debug, Default)]
struct LibStats {
    /// Total modules parsed.
    modules: usize,
    /// Total lines in the lib group.
    lines: usize,
    /// Total tokens (main + side, including whitespace).
    tokens: usize,
    /// Total non whitespace tokens.
    tokens_no_ws: usize,
    /// Aggregate counts for the lib group.
    counts: Counts,
    /// Per module stats for the lib group.
    module_stats: Vec<ModuleStats>,
}

/// Parse stats for a single module.
#[derive(Debug, Clone)]
struct ModuleStats {
    /// Module path.
    name: String,
    /// Total lines in the module.
    lines: usize,
    /// Total tokens (main + side, including whitespace).
    tokens: usize,
    /// Total non whitespace tokens.
    tokens_no_ws: usize,
    /// Aggregate counts for the module.
    counts: Counts,
}

fn main() {
    let args = Args::parse();
    let style = style_for_args(&args);

    // default lib set mirrors the current perf comparisons
    let libs = if args.libs.is_empty() {
        vec![
            "bun".to_string(),
            "node".to_string(),
            "deno".to_string(),
            "dom".to_string(),
            "es2022.full".to_string(),
        ]
    } else {
        args.libs.clone()
    };

    // compute stats per lib group
    let mut results = Vec::new();
    for lib_name in libs {
        let stats = collect_stats_for_lib(&lib_name, args.modules);
        results.push((lib_name, stats));
    }

    // print summary
    let summary_width = print_summary_table(&results, style);

    // print detail sections
    for (lib_name, stats) in &results {
        print_detail_section(lib_name, stats, style, &args, summary_width);
    }
}

/// Build output style from CLI arguments.
fn style_for_args(args: &Args) -> Style {
    if args.no_color {
        return Style {
            bold: "",
            dim: "",
            cyan: "",
            reset: "",
        };
    }

    Style {
        bold: BOLD,
        dim: DIM,
        cyan: CYAN,
        reset: RESET,
    }
}

/// Collect parse stats for a builtin lib and its dependencies.
fn collect_stats_for_lib(lib_name: &str, include_modules: bool) -> LibStats {
    // build the seed list used by the bench runner
    let seed_libs = seed_libs_for_bench(lib_name);
    let version_overrides = collect_library_version_overrides(&seed_libs);

    // collect dependency libs in order
    let mut ordered_libraries = Vec::new();
    let mut seen_libs = HashSet::new();
    for name in seed_libs {
        collect_library_dependencies(
            name,
            &version_overrides,
            &mut ordered_libraries,
            &mut seen_libs,
        );
    }

    // parse each unique source module
    let mut stats = LibStats::default();
    let mut seen_modules = HashSet::new();
    let mut next_file_id = 0u32;
    for lib in ordered_libraries {
        for source in lib.sources {
            if !seen_modules.insert(source.module_path()) {
                continue;
            }

            // parse the source
            let file_id = FileId::new(next_file_id as u64);
            next_file_id += 1;
            let file = file_from_source(file_id, *source);
            let language = LanguageType::from(file.ty);
            let mut parser = DestackParser::lex_file(Arc::new(file.clone()), language);
            parser.parse();

            // collect module level stats
            let (tokens, tokens_no_ws) = count_tokens(&mut parser);
            let counts = collect_counts(&parser);
            let module_stats = ModuleStats {
                name: source.module_path().to_string(),
                lines: file.line_count() as usize,
                tokens,
                tokens_no_ws,
                counts: counts.clone(),
            };

            // aggregate totals
            stats.modules += 1;
            stats.lines += module_stats.lines;
            stats.tokens += module_stats.tokens;
            stats.tokens_no_ws += module_stats.tokens_no_ws;
            stats.counts.add(&counts);

            // track per module stats when requested
            if include_modules {
                stats.module_stats.push(module_stats);
            }
        }
    }

    stats
}

/// Seed the lib list for a bench style run.
fn seed_libs_for_bench(lib_name: &str) -> Vec<&'static str> {
    // include the primary lib
    let lib = builtin_library(lib_name).unwrap_or_else(|| panic!("missing lib {lib_name}"));
    let mut libs = Vec::new();

    // include baseline es2020 for non es libs
    if lib.kind == BuiltinLibraryKind::Library
        && !lib.name.starts_with("es")
        && !lib.name.starts_with("decorators")
    {
        libs.push("es2020");
    }
    libs.push(lib.name);

    // include ambient roots
    if !libs.contains(&"globals") {
        libs.push("globals");
    }

    libs
}

/// Split a versioned lib name into base and version.
fn split_versioned_lib_name(name: &str) -> Option<(&str, &str)> {
    let (base, version) = name.rsplit_once(".v")?;
    let is_versioned = version.chars().next().is_some_and(|ch| ch.is_ascii_digit());
    if is_versioned {
        Some((base, version))
    } else {
        None
    }
}

/// Collect explicit lib version overrides from the seed list.
fn collect_library_version_overrides(libs: &[&str]) -> HashMap<String, String> {
    let mut overrides = HashMap::new();
    let mut seen = HashSet::new();

    for lib in libs {
        collect_library_version_overrides_for_lib(lib, &mut overrides, &mut seen);
    }

    overrides
}

/// Collect versioned libs that should override unversioned dependencies.
fn collect_library_version_overrides_for_lib(
    name: &str,
    overrides: &mut HashMap<String, String>,
    seen: &mut HashSet<String>,
) {
    // skip already visited libs
    if !seen.insert(name.to_string()) {
        return;
    }

    // record explicit versioned libs
    if let Some((base, _version)) = split_versioned_lib_name(name) {
        overrides
            .entry(base.to_string())
            .or_insert_with(|| name.to_string());
    }

    // load the lib for dependencies
    let lib = builtin_library(name).unwrap_or_else(|| panic!("missing lib {name}"));

    // walk dependencies only
    for &dependency in lib.dependencies {
        collect_library_version_overrides_for_lib(dependency, overrides, seen);
    }
}

/// Resolve a dependency name to a versioned override when available.
fn resolve_lib_dependency_name(name: &str, overrides: &HashMap<String, String>) -> String {
    if split_versioned_lib_name(name).is_some() {
        return name.to_string();
    }

    overrides
        .get(name)
        .cloned()
        .unwrap_or_else(|| name.to_string())
}

/// Collect builtin lib dependencies in order.
fn collect_library_dependencies(
    lib_name: &str,
    version_overrides: &HashMap<String, String>,
    ordered: &mut Vec<&'static BuiltinLibrary>,
    seen: &mut HashSet<String>,
) {
    // avoid duplicate work
    if !seen.insert(lib_name.to_string()) {
        return;
    }

    // resolve the lib
    let lib = builtin_library(lib_name).unwrap_or_else(|| panic!("missing lib {lib_name}"));

    // collect explicit dependencies
    for dependency in lib.dependencies {
        let dependency = resolve_lib_dependency_name(dependency, version_overrides);
        collect_library_dependencies(&dependency, version_overrides, ordered, seen);
    }

    // collect reference lib dependencies
    for &reference in lib.reference_libs {
        let reference = resolve_lib_dependency_name(reference, version_overrides);
        collect_library_dependencies(&reference, version_overrides, ordered, seen);
    }

    // append the lib once dependencies are recorded
    ordered.push(lib);
}

/// Build a File from a builtin lib source.
fn file_from_source(file_id: FileId, source: destack_builtin::BuiltinLibrarySource) -> File {
    // build a virtual path
    let module_path = source.module_path();
    let path = Path::new(&module_path).to_path_buf();
    let uri = Uri::from_path(&path);

    // infer file type from module path
    let file_type = FileType::from_path_or_unknown(&path);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("builtin")
        .to_string();

    File::from_text(
        file_id,
        name,
        uri,
        Some(path),
        file_type,
        source.content.to_string(),
    )
}

/// Count total and non whitespace tokens for a parsed file.
fn count_tokens(parser: &mut DestackParser) -> (usize, usize) {
    let (tokens, side_tokens) = parser.take_tokens();

    // aggregate tokens from both streams
    let token_count = tokens.len() + side_tokens.len();

    // filter out whitespace tokens
    let tokens_no_ws = tokens
        .iter()
        .chain(side_tokens.iter())
        .filter(|token| token.token.ty != destack_ast::TokenType::Whitespace)
        .count();

    (token_count, tokens_no_ws)
}

/// Collect node counts for a parsed module.
fn collect_counts(parser: &DestackParser) -> Counts {
    // count node categories
    let mut counts = Counts {
        expressions: parser.tree.get_nodes::<Expression>().len(),
        declarations: parser.tree.get_nodes::<Declaration>().len(),
        members: parser.tree.get_nodes::<Member>().len(),
        properties: parser.tree.get_nodes::<Property>().len(),
        parameters: parser.tree.get_nodes::<Parameter>().len(),
        arguments: parser.tree.get_nodes::<Argument>().len(),
        patterns: parser.tree.get_nodes::<Pattern>().len(),
        pattern_fields: parser.tree.get_nodes::<PatternField>().len(),
        declarators: parser.tree.get_nodes::<Declarator>().len(),
        enum_fields: parser.tree.get_nodes::<EnumField>().len(),
        match_cases: parser.tree.get_nodes::<MatchCase>().len(),
        where_clauses: parser.tree.get_nodes::<WhereClause>().len(),
        dependency_items: parser.tree.get_nodes::<DependencyItem>().len(),
        ..Default::default()
    };

    // count declaration kinds
    for declaration_id in parser.tree.get_nodes::<Declaration>() {
        match parser.tree.get(declaration_id) {
            Declaration::Interface { .. } => {
                counts.interfaces += 1;
            }
            Declaration::Namespace { .. } => {
                counts.namespaces += 1;
            }
            Declaration::Type { .. } => {
                counts.type_decls += 1;
            }
            Declaration::Enum { .. } => {
                counts.enum_decls += 1;
            }
            Declaration::Struct { .. } => {
                counts.struct_decls += 1;
            }
            Declaration::Class { .. } => {
                counts.class_decls += 1;
            }
            Declaration::Extension { .. } => {
                counts.extension_decls += 1;
            }
            Declaration::Function { .. } => {
                counts.function_decls += 1;
            }
            _ => {}
        }
    }

    // count annotation related nodes
    counts.annotations += parser.tree.get_nodes::<Annotation>().len();
    counts.annotations += parser.tree.comments().len();
    counts.annotations += parser.tree.get_nodes::<Decorator>().len();

    // count expression shapes
    for expression_id in parser.tree.get_nodes::<Expression>() {
        match parser.tree.get(expression_id) {
            Expression::TypeLiteral(_) => {
                counts.expr_type_literals += 1;
            }
            Expression::TemplateExpression { .. } => {
                counts.expr_templates += 1;
            }
            Expression::TaggedTemplateExpression { .. } => {
                counts.expr_tagged_templates += 1;
            }
            Expression::Import { .. } => {
                counts.expr_imports += 1;
            }
            Expression::Export { .. } => {
                counts.expr_exports += 1;
            }
            Expression::Let { .. } => {
                counts.expr_lets += 1;
            }
            Expression::Using { .. } => {
                counts.expr_usings += 1;
            }
            Expression::Call { .. } => {
                counts.expr_calls += 1;
            }
            Expression::Member { .. } | Expression::PrivateMember { .. } => {
                counts.expr_members += 1;
            }
            Expression::Index { .. } => {
                counts.expr_indexes += 1;
            }
            Expression::New { .. } => {
                counts.expr_news += 1;
            }
            _ => {}
        }
    }

    counts
}

/// Format a count with digit grouping.
fn format_count(value: usize) -> String {
    let raw = value.to_string();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (index, ch) in raw.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Render a separator line for a table.
fn render_separator(width: usize) -> String {
    "─".repeat(width)
}

/// Width hints for summary rows.
#[derive(Default)]
struct SummaryWidths {
    /// Lib name width.
    lib: usize,
    /// Module count width.
    modules: usize,
    /// Line count width.
    lines: usize,
    /// Token count width.
    tokens: usize,
    /// Non whitespace token width.
    tokens_no_ws: usize,
    /// Expression count width.
    expressions: usize,
    /// Declaration count width.
    declarations: usize,
    /// Interface count width.
    interfaces: usize,
    /// Namespace count width.
    namespaces: usize,
    /// Annotation count width.
    annotations: usize,
}

/// Summary row values.
struct SummaryRow {
    /// Lib name.
    lib: String,
    /// Module count.
    modules: String,
    /// Line count.
    lines: String,
    /// Token count.
    tokens: String,
    /// Non whitespace token count.
    tokens_no_ws: String,
    /// Expression count.
    expressions: String,
    /// Declaration count.
    declarations: String,
    /// Interface count.
    interfaces: String,
    /// Namespace count.
    namespaces: String,
    /// Annotation count.
    annotations: String,
}

/// Print the summary table and return its width.
fn print_summary_table(results: &[(String, LibStats)], style: Style) -> usize {
    let mut rows = Vec::with_capacity(results.len());
    for (lib, stats) in results {
        rows.push(SummaryRow {
            lib: lib.clone(),
            modules: format_count(stats.modules),
            lines: format_count(stats.lines),
            tokens: format_count(stats.tokens),
            tokens_no_ws: format_count(stats.tokens_no_ws),
            expressions: format_count(stats.counts.expressions),
            declarations: format_count(stats.counts.declarations),
            interfaces: format_count(stats.counts.interfaces),
            namespaces: format_count(stats.counts.namespaces),
            annotations: format_count(stats.counts.annotations),
        });
    }

    let mut widths = SummaryWidths {
        lib: "Lib".len(),
        modules: "Modules".len(),
        lines: "Lines".len(),
        tokens: "Tokens".len(),
        tokens_no_ws: "Tokens/ws".len(),
        expressions: "Expr".len(),
        declarations: "Decl".len(),
        interfaces: "Iface".len(),
        namespaces: "Namespace".len(),
        annotations: "Anno".len(),
    };

    for row in &rows {
        widths.lib = widths.lib.max(row.lib.len());
        widths.modules = widths.modules.max(row.modules.len());
        widths.lines = widths.lines.max(row.lines.len());
        widths.tokens = widths.tokens.max(row.tokens.len());
        widths.tokens_no_ws = widths.tokens_no_ws.max(row.tokens_no_ws.len());
        widths.expressions = widths.expressions.max(row.expressions.len());
        widths.declarations = widths.declarations.max(row.declarations.len());
        widths.interfaces = widths.interfaces.max(row.interfaces.len());
        widths.namespaces = widths.namespaces.max(row.namespaces.len());
        widths.annotations = widths.annotations.max(row.annotations.len());
    }

    let total_width = widths.lib
        + widths.modules
        + widths.lines
        + widths.tokens
        + widths.tokens_no_ws
        + widths.expressions
        + widths.declarations
        + widths.interfaces
        + widths.namespaces
        + widths.annotations
        + COLUMN_GAP.len() * 9;
    let separator = render_separator(total_width);

    println!(
        "{dim}{separator}{reset}",
        dim = style.dim,
        reset = style.reset
    );
    println!(
        "{bold}{lib:<lib_w$}{gap}{modules:>modules_w$}{gap}{lines:>lines_w$}{gap}{tokens:>tokens_w$}{gap}{tokens_no_ws:>tokens_no_ws_w$}{gap}{expr:>expr_w$}{gap}{decl:>decl_w$}{gap}{iface:>iface_w$}{gap}{ns:>ns_w$}{gap}{anno:>anno_w$}{reset}",
        bold = style.bold,
        reset = style.reset,
        gap = COLUMN_GAP,
        lib = "Lib",
        modules = "Modules",
        lines = "Lines",
        tokens = "Tokens",
        tokens_no_ws = "Tokens/ws",
        expr = "Expr",
        decl = "Decl",
        iface = "Iface",
        ns = "Namespace",
        anno = "Anno",
        lib_w = widths.lib,
        modules_w = widths.modules,
        lines_w = widths.lines,
        tokens_w = widths.tokens,
        tokens_no_ws_w = widths.tokens_no_ws,
        expr_w = widths.expressions,
        decl_w = widths.declarations,
        iface_w = widths.interfaces,
        ns_w = widths.namespaces,
        anno_w = widths.annotations,
    );
    println!(
        "{dim}{separator}{reset}",
        dim = style.dim,
        reset = style.reset
    );

    for row in rows {
        println!(
            "{cyan}{lib:<lib_w$}{reset}{gap}{modules:>modules_w$}{gap}{lines:>lines_w$}{gap}{tokens:>tokens_w$}{gap}{tokens_no_ws:>tokens_no_ws_w$}{gap}{expr:>expr_w$}{gap}{decl:>decl_w$}{gap}{iface:>iface_w$}{gap}{ns:>ns_w$}{gap}{anno:>anno_w$}",
            cyan = style.cyan,
            reset = style.reset,
            gap = COLUMN_GAP,
            lib = row.lib,
            modules = row.modules,
            lines = row.lines,
            tokens = row.tokens,
            tokens_no_ws = row.tokens_no_ws,
            expr = row.expressions,
            decl = row.declarations,
            iface = row.interfaces,
            ns = row.namespaces,
            anno = row.annotations,
            lib_w = widths.lib,
            modules_w = widths.modules,
            lines_w = widths.lines,
            tokens_w = widths.tokens,
            tokens_no_ws_w = widths.tokens_no_ws,
            expr_w = widths.expressions,
            decl_w = widths.declarations,
            iface_w = widths.interfaces,
            ns_w = widths.namespaces,
            anno_w = widths.annotations,
        );
    }

    println!(
        "{dim}{separator}{reset}",
        dim = style.dim,
        reset = style.reset
    );

    total_width
}

/// Print the detail section for a lib.
fn print_detail_section(
    lib: &str,
    stats: &LibStats,
    style: Style,
    args: &Args,
    summary_width: usize,
) {
    let header_text = format!("lib {lib}");
    let header = format!(
        "{dim}lib{reset} {cyan}{lib}{reset}",
        dim = style.dim,
        reset = style.reset,
        cyan = style.cyan,
        lib = lib,
    );
    let separator_width = summary_width.max(header_text.len());
    let separator = render_separator(separator_width);
    println!(
        "{dim}{separator}{reset}",
        dim = style.dim,
        reset = style.reset
    );
    println!("{header}");
    println!(
        "{dim}{separator}{reset}",
        dim = style.dim,
        reset = style.reset
    );

    // declaration counts
    println!(
        "{dim}decls{reset}: type={type_} enum={enum_} struct={struct_} class={class_} extension={extension} function={function}",
        dim = style.dim,
        reset = style.reset,
        type_ = format_count(stats.counts.type_decls),
        enum_ = format_count(stats.counts.enum_decls),
        struct_ = format_count(stats.counts.struct_decls),
        class_ = format_count(stats.counts.class_decls),
        extension = format_count(stats.counts.extension_decls),
        function = format_count(stats.counts.function_decls),
    );

    // node counts
    println!(
        "{dim}nodes{reset}: member={member} prop={prop} param={param} arg={arg} pattern={pattern} field={field} declr={declr} enumf={enumf} match={match_} where={where_} dep={dep}",
        dim = style.dim,
        reset = style.reset,
        member = format_count(stats.counts.members),
        prop = format_count(stats.counts.properties),
        param = format_count(stats.counts.parameters),
        arg = format_count(stats.counts.arguments),
        pattern = format_count(stats.counts.patterns),
        field = format_count(stats.counts.pattern_fields),
        declr = format_count(stats.counts.declarators),
        enumf = format_count(stats.counts.enum_fields),
        match_ = format_count(stats.counts.match_cases),
        where_ = format_count(stats.counts.where_clauses),
        dep = format_count(stats.counts.dependency_items),
    );

    // expression counts
    println!(
        "{dim}exprs{reset}: type_lit={type_lit} template={template} tagged={tagged} import={import} export={export} let={let_} using={using} call={call} member={member} index={index} new={new_}",
        dim = style.dim,
        reset = style.reset,
        type_lit = format_count(stats.counts.expr_type_literals),
        template = format_count(stats.counts.expr_templates),
        tagged = format_count(stats.counts.expr_tagged_templates),
        import = format_count(stats.counts.expr_imports),
        export = format_count(stats.counts.expr_exports),
        let_ = format_count(stats.counts.expr_lets),
        using = format_count(stats.counts.expr_usings),
        call = format_count(stats.counts.expr_calls),
        member = format_count(stats.counts.expr_members),
        index = format_count(stats.counts.expr_indexes),
        new_ = format_count(stats.counts.expr_news),
    );

    // module breakdowns
    if args.modules {
        print_module_table(stats, style, args.module_top);
    }
}

/// Print module breakdown table for a lib.
fn print_module_table(stats: &LibStats, style: Style, module_top: usize) {
    let mut modules = stats.module_stats.clone();
    modules.sort_by(|left, right| right.lines.cmp(&left.lines));

    let limit = if module_top == 0 {
        modules.len()
    } else {
        module_top.min(modules.len())
    };
    let total_modules = stats.module_stats.len();

    let mut rows = Vec::new();
    for module in modules.into_iter().take(limit) {
        rows.push(ModuleRow {
            module: module.name,
            lines: format_count(module.lines),
            tokens: format_count(module.tokens),
            tokens_no_ws: format_count(module.tokens_no_ws),
            expressions: format_count(module.counts.expressions),
            declarations: format_count(module.counts.declarations),
            interfaces: format_count(module.counts.interfaces),
            namespaces: format_count(module.counts.namespaces),
            annotations: format_count(module.counts.annotations),
        });
    }

    if rows.is_empty() {
        return;
    }

    let mut widths = ModuleWidths {
        module: "Module".len(),
        lines: "Lines".len(),
        tokens: "Tokens".len(),
        tokens_no_ws: "Tokens/ws".len(),
        expressions: "Expr".len(),
        declarations: "Decl".len(),
        interfaces: "Iface".len(),
        namespaces: "Namespace".len(),
        annotations: "Anno".len(),
    };

    for row in &rows {
        widths.module = widths.module.max(row.module.len());
        widths.lines = widths.lines.max(row.lines.len());
        widths.tokens = widths.tokens.max(row.tokens.len());
        widths.tokens_no_ws = widths.tokens_no_ws.max(row.tokens_no_ws.len());
        widths.expressions = widths.expressions.max(row.expressions.len());
        widths.declarations = widths.declarations.max(row.declarations.len());
        widths.interfaces = widths.interfaces.max(row.interfaces.len());
        widths.namespaces = widths.namespaces.max(row.namespaces.len());
        widths.annotations = widths.annotations.max(row.annotations.len());
    }

    let total_width = widths.module
        + widths.lines
        + widths.tokens
        + widths.tokens_no_ws
        + widths.expressions
        + widths.declarations
        + widths.interfaces
        + widths.namespaces
        + widths.annotations
        + COLUMN_GAP.len() * 8;
    let separator = render_separator(total_width);

    let label = if rows.len() == total_modules {
        format!("modules: {total_modules}")
    } else {
        format!("modules: showing {} of {total_modules}", rows.len())
    };
    println!("{dim}{label}{reset}", dim = style.dim, reset = style.reset);
    println!(
        "{dim}{separator}{reset}",
        dim = style.dim,
        reset = style.reset
    );
    println!(
        "{bold}{module:<module_w$}{gap}{lines:>lines_w$}{gap}{tokens:>tokens_w$}{gap}{tokens_no_ws:>tokens_no_ws_w$}{gap}{expr:>expr_w$}{gap}{decl:>decl_w$}{gap}{iface:>iface_w$}{gap}{ns:>ns_w$}{gap}{anno:>anno_w$}{reset}",
        bold = style.bold,
        reset = style.reset,
        gap = COLUMN_GAP,
        module = "Module",
        lines = "Lines",
        tokens = "Tokens",
        tokens_no_ws = "Tokens/ws",
        expr = "Expr",
        decl = "Decl",
        iface = "Iface",
        ns = "Namespace",
        anno = "Anno",
        module_w = widths.module,
        lines_w = widths.lines,
        tokens_w = widths.tokens,
        tokens_no_ws_w = widths.tokens_no_ws,
        expr_w = widths.expressions,
        decl_w = widths.declarations,
        iface_w = widths.interfaces,
        ns_w = widths.namespaces,
        anno_w = widths.annotations,
    );
    println!(
        "{dim}{separator}{reset}",
        dim = style.dim,
        reset = style.reset
    );

    for row in rows {
        println!(
            "{module:<module_w$}{gap}{lines:>lines_w$}{gap}{tokens:>tokens_w$}{gap}{tokens_no_ws:>tokens_no_ws_w$}{gap}{expr:>expr_w$}{gap}{decl:>decl_w$}{gap}{iface:>iface_w$}{gap}{ns:>ns_w$}{gap}{anno:>anno_w$}",
            gap = COLUMN_GAP,
            module = row.module,
            lines = row.lines,
            tokens = row.tokens,
            tokens_no_ws = row.tokens_no_ws,
            expr = row.expressions,
            decl = row.declarations,
            iface = row.interfaces,
            ns = row.namespaces,
            anno = row.annotations,
            module_w = widths.module,
            lines_w = widths.lines,
            tokens_w = widths.tokens,
            tokens_no_ws_w = widths.tokens_no_ws,
            expr_w = widths.expressions,
            decl_w = widths.declarations,
            iface_w = widths.interfaces,
            ns_w = widths.namespaces,
            anno_w = widths.annotations,
        );
    }
    println!(
        "{dim}{separator}{reset}",
        dim = style.dim,
        reset = style.reset
    );
}

/// Column widths for module rows.
#[derive(Default)]
struct ModuleWidths {
    /// Module name width.
    module: usize,
    /// Line count width.
    lines: usize,
    /// Token count width.
    tokens: usize,
    /// Non whitespace token width.
    tokens_no_ws: usize,
    /// Expression count width.
    expressions: usize,
    /// Declaration count width.
    declarations: usize,
    /// Interface count width.
    interfaces: usize,
    /// Namespace count width.
    namespaces: usize,
    /// Annotation count width.
    annotations: usize,
}

/// Module row values.
struct ModuleRow {
    /// Module name.
    module: String,
    /// Line count.
    lines: String,
    /// Token count.
    tokens: String,
    /// Non whitespace token count.
    tokens_no_ws: String,
    /// Expression count.
    expressions: String,
    /// Declaration count.
    declarations: String,
    /// Interface count.
    interfaces: String,
    /// Namespace count.
    namespaces: String,
    /// Annotation count.
    annotations: String,
}
