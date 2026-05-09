use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_ast::{Expression, LocalNodeId, NodeParentIndex};
use destack_core::{StableHasher, StringPool};
use destack_fir::format as fir_format;
use destack_formatter::{
    DestackFormatContext, DestackFormatOptions, format_file_source, statement_list,
};
use destack_parser::Parser;
use destack_service::{FileImage, LanguageService, LanguageServiceError};
use destack_source::{
    DiagnosticSeverity, File, FileId, FileType, LanguageType, OverlayFileSystem, Span,
    WATCHABLE_FILE_TYPES,
};
use destack_workspace::{FormatterOptions, Repository, Revision};
use {destack_lsp_types as lsp, destack_query as query};

/// Globs for config files tracked by the LSP.
pub(super) const CONFIG_GLOBS: [&str; 1] = ["**/destack.json"];

/// Build a standalone file from an image without mutating the repository.
pub(super) fn file_from_image_for_diagnostics(image: &FileImage) -> Option<Arc<File>> {
    let content = image.content.as_ref()?;
    let file = file_from_image(image, image.id, content);

    Some(Arc::new(file))
}

/// Build a file from an image payload.
fn file_from_image(image: &FileImage, file_id: FileId, content: &str) -> File {
    File::from_text(
        file_id,
        image.name.clone(),
        image.uri.clone(),
        image.path.clone(),
        image.file_type,
        content.to_string(),
    )
}

/// Build file watcher patterns for the client.
pub(super) fn build_file_watchers() -> Vec<lsp::FileSystemWatcher> {
    let mut watchers = Vec::new();
    for pattern in tracked_file_globs() {
        watchers.push(lsp::FileSystemWatcher {
            glob_pattern: pattern.to_string().into(),
            kind: None,
        });
    }

    watchers
}

/// Build the set of file globs tracked by the LSP.
pub(super) fn tracked_file_globs() -> Vec<&'static str> {
    let mut patterns = Vec::new();
    for file_type in WATCHABLE_FILE_TYPES {
        for pattern in file_type.globs() {
            if !patterns.contains(pattern) {
                patterns.push(pattern);
            }
        }
    }

    // append config globs
    for pattern in CONFIG_GLOBS {
        if !patterns.contains(&pattern) {
            patterns.push(pattern);
        }
    }

    patterns
}

/// Create the language service for one LSP repository.
pub(super) fn create_language_service(
    repository: Arc<Repository>,
    overlay_fs: Arc<OverlayFileSystem>,
    roots: Vec<PathBuf>,
    workers: usize,
) -> Result<LanguageService, LanguageServiceError> {
    LanguageService::new(repository, Some(overlay_fs), roots, workers, None)
}

/// Convert completion kind to LSP completion item kind.
pub(super) fn completion_kind_to_lsp(kind: query::CompletionKind) -> lsp::CompletionItemKind {
    match kind {
        query::CompletionKind::Text => lsp::CompletionItemKind::TEXT,
        query::CompletionKind::Method => lsp::CompletionItemKind::METHOD,
        query::CompletionKind::Function => lsp::CompletionItemKind::FUNCTION,
        query::CompletionKind::Constructor => lsp::CompletionItemKind::CONSTRUCTOR,
        query::CompletionKind::Field => lsp::CompletionItemKind::FIELD,
        query::CompletionKind::Variable => lsp::CompletionItemKind::VARIABLE,
        query::CompletionKind::Class => lsp::CompletionItemKind::CLASS,
        query::CompletionKind::Interface => lsp::CompletionItemKind::INTERFACE,
        query::CompletionKind::Module => lsp::CompletionItemKind::MODULE,
        query::CompletionKind::Property => lsp::CompletionItemKind::PROPERTY,
        query::CompletionKind::Unit => lsp::CompletionItemKind::UNIT,
        query::CompletionKind::Value => lsp::CompletionItemKind::VALUE,
        query::CompletionKind::Enum => lsp::CompletionItemKind::ENUM,
        query::CompletionKind::Keyword => lsp::CompletionItemKind::KEYWORD,
        query::CompletionKind::Snippet => lsp::CompletionItemKind::SNIPPET,
        query::CompletionKind::Color => lsp::CompletionItemKind::COLOR,
        query::CompletionKind::File => lsp::CompletionItemKind::FILE,
        query::CompletionKind::Reference => lsp::CompletionItemKind::REFERENCE,
        query::CompletionKind::Folder => lsp::CompletionItemKind::FOLDER,
        query::CompletionKind::EnumMember => lsp::CompletionItemKind::ENUM_MEMBER,
        query::CompletionKind::Constant => lsp::CompletionItemKind::CONSTANT,
        query::CompletionKind::Struct => lsp::CompletionItemKind::STRUCT,
        query::CompletionKind::Event => lsp::CompletionItemKind::EVENT,
        query::CompletionKind::Operator => lsp::CompletionItemKind::OPERATOR,
        query::CompletionKind::TypeParameter => lsp::CompletionItemKind::TYPE_PARAMETER,
    }
}

/// Compute a deterministic result id for a diagnostics payload.
pub(super) fn diagnostic_result_id(diagnostics: &[destack_source::Diagnostic]) -> String {
    let mut hasher = StableHasher::new();
    diagnostics.len().hash(&mut hasher);
    for diagnostic in diagnostics {
        diagnostic.code.hash(&mut hasher);
        diagnostic.message.hash(&mut hasher);
        let primary = diagnostic.primary_label();
        primary.content.hash(&mut hasher);
        let primary_span = primary.span;
        primary_span.start.hash(&mut hasher);
        primary_span.end.hash(&mut hasher);
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => 0u8,
            DiagnosticSeverity::Warning => 1u8,
            DiagnosticSeverity::Note => 2u8,
        };
        severity.hash(&mut hasher);
    }
    format!("{:x}", hasher.finish_u64())
}

/// Format a file and return the formatted content.
/// Requires module AST state from the repository graph.
pub(super) fn format_file(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    file: &Arc<File>,
    formatter: FormatterOptions,
) -> Option<String> {
    // dispatch non destack languages through the shared source formatter
    if matches!(file.ty, FileType::Css | FileType::Html) {
        return format_file_source(file.as_ref(), file.text(), formatter).ok();
    }

    let language_type = LanguageType::try_from(file.ty).ok()?;
    let format_options = DestackFormatOptions {
        language_type,
        ..formatter.into()
    };

    // resolve module state for this file
    let module_id = repository.module_id_for_file(revision, file_id).ok()??;
    repository.file(revision, file_id).ok().flatten()?;
    let ast_key = ArtifactKey::ast(module_id);
    let ast_version = repository
        .artifact_version(revision, &ast_key)
        .ok()
        .flatten()?;
    let ast = repository.artifact_store().ast(&ast_version)?;

    // build format context from committed semantic state
    let side_span = Parser::compute_side_span_from_tree(&ast.tree);
    let strings = repository.string_pool();
    let context = DestackFormatContext::new(
        format_options,
        file.as_ref(),
        &ast.tree,
        &ast.tokens,
        &ast.side_tokens,
        &side_span,
        strings.as_ref(),
        ast.parents.clone(),
    );

    format_expressions(&context, &ast.roots)
}

/// Format expressions and return the result string.
fn format_expressions<'ast>(
    context: &DestackFormatContext<'ast>,
    expressions: &'ast [LocalNodeId<Expression>],
) -> Option<String> {
    let mut result = if expressions.is_empty() {
        String::new()
    } else {
        let formatted = fir_format!(context.clone(), [statement_list(expressions)]).ok()?;
        let printed = formatted.print().ok()?;
        printed.as_str().to_string()
    };

    // ensure trailing newline
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    Some(result)
}

/// Resolve formatting options for one path in one revision.
pub(super) fn formatting_options_for_path(
    repository: &Repository,
    revision: Revision,
    path: &std::path::Path,
) -> FormatterOptions {
    let package = repository.nearest_package(revision, path).ok().flatten();
    if let Some(package) = package
        && let Ok(Some(config)) = repository.destack_config_for_package_id(revision, package.id)
    {
        return config.formatter.clone();
    }

    repository
        .destack_config_for_workspace(revision)
        .ok()
        .flatten()
        .map(|config| config.formatter.clone())
        .unwrap_or_default()
}

/// Format a range within a file and return the formatted content with the actual range.
pub(super) fn format_range(
    file: &Arc<File>,
    formatter: FormatterOptions,
    start_offset: u32,
    end_offset: u32,
) -> Option<(String, Span)> {
    // parse file
    let language_type = LanguageType::try_from(file.ty).ok()?;
    let mut parser = Parser::lex_file(file.clone(), language_type, Arc::new(StringPool::new()));
    let expressions = parser.parse();

    // bail if parse errors
    if parser
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return None;
    }

    // find expressions that overlap with the range
    let overlapping: Vec<_> = expressions
        .iter()
        .filter(|expr_id| {
            let span = parser.tree.get_span(**expr_id);
            span.start < end_offset && span.end > start_offset
        })
        .copied()
        .collect();

    if overlapping.is_empty() {
        return None;
    }

    // compute the actual range we're formatting (union of overlapping expressions)
    let first_span = parser.tree.get_span(overlapping[0]);
    let last_span = parser.tree.get_span(*overlapping.last().unwrap());
    let actual_range = Span::new(file.id, first_span.start, last_span.end);

    // build format context
    let side_span = parser.compute_side_span();
    let (tokens, side_tokens) = parser.take_tokens();
    let strings = parser.strings.as_ref();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let format_options = DestackFormatOptions {
        language_type,
        ..formatter.into()
    };
    let context = DestackFormatContext::new(
        format_options,
        file.as_ref(),
        &parser.tree,
        &tokens,
        &side_tokens,
        &side_span,
        strings,
        parents,
    );

    // format overlapping expressions
    let formatted = fir_format!(context.clone(), [statement_list(&overlapping)]).ok()?;
    let printed = formatted.print().ok()?;
    let mut result = printed.as_str().to_string();

    // ensure trailing newline if we're at end of file
    let is_at_end = last_span.end >= file.len.saturating_sub(1);
    if is_at_end && !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    Some((result, actual_range))
}

/// Normalize line endings to LF.
pub(super) fn normalize_line_endings(content: String) -> String {
    if content.contains('\r') {
        content.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        content
    }
}
