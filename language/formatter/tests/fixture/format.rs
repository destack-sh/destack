use std::path::Path;
use std::sync::Arc;

use tspp_core::StringPool;
use tspp_dir::{NodeParentIndex, Tree};
use tspp_fir::format as fir_format;
use tspp_fir::format::Allocator;
use tspp_formatter::{TsppFormatContext, TsppFormatOptions, statement_list};
use tspp_parser::{CommentRetention, ParseOptions, Parser};
use tspp_repository::FormatterOptions;
use tspp_source::{
    DiagnosticSeverity, File, FileId, FileType, LanguageType, ModuleId, PackageId, Uri,
};

/// Format one complete source file and reject diagnostics at the requested severity.
pub(super) fn format_source(
    logical_path: &Path,
    physical_path: Option<&Path>,
    source: String,
    options: FormatterOptions,
    minimum_severity: DiagnosticSeverity,
) -> Result<String, String> {
    let file_type = FileType::from_path(logical_path)
        .ok_or_else(|| format!("unsupported formatter fixture '{}'", logical_path.display()))?;
    let language = LanguageType::try_from(file_type).map_err(|file_type| {
        format!(
            "formatter fixture '{}' has unsupported type {file_type:?}",
            logical_path.display()
        )
    })?;
    let file_id = FileId::from_logical_path(logical_path);
    let file_name = logical_path.to_string_lossy().into_owned();
    let uri = physical_path.map_or_else(
        || Uri::from_string(format!("/test/{}", logical_path.display())),
        Uri::from_path,
    );
    let file = File::from_text(
        file_id,
        file_name,
        uri,
        physical_path.map(Path::to_path_buf),
        file_type,
        source,
    )
    .map_err(|error| {
        format!(
            "failed to load formatter fixture '{}': {error}",
            logical_path.display()
        )
    })?;
    let file = Arc::new(file);

    // retain every comment because formatting owns their placement
    let module_id = ModuleId::new(PackageId::new(0), file.id.0);
    let parser = Parser::new(
        file.clone(),
        language,
        Tree::new(module_id),
        ParseOptions {
            comment_retention: CommentRetention::All,
            ..ParseOptions::default()
        },
    );
    let mut parse = parser.parse();
    let diagnostics = parse.diagnostics();
    let failures = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity >= minimum_severity)
        .map(|diagnostic| {
            format!(
                "{}[{}]: {}",
                diagnostic.severity.family_name(),
                diagnostic.id,
                diagnostic.message
            )
        })
        .collect::<Vec<_>>();
    if !failures.is_empty() {
        return Err(failures.join("\n"));
    }

    // build the formatting context from the complete parsed file
    let tokens = parse.take_token_spans();
    let decorators = parse.tree.decorator_span();
    let strings = StringPool::new();
    strings.extend(&parse.strings);
    let parents = NodeParentIndex::from_roots(&parse.tree, &parse.roots);
    let options = TsppFormatOptions::from_formatter_options(options, language);
    let context = TsppFormatContext::new(
        options,
        &file,
        &parse.tree,
        &tokens,
        &parse.comments,
        &decorators,
        &strings,
        &parents,
    );

    // print one canonical file with a final newline
    let allocator = Allocator::default();
    let document = fir_format!(&allocator, context, [statement_list(&parse.roots)])
        .map_err(|error| format!("failed to format '{}': {error}", logical_path.display()))?;
    let mut output = document
        .print()
        .map_err(|error| format!("failed to print '{}': {error}", logical_path.display()))?
        .as_str()
        .to_string();
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    Ok(output)
}
