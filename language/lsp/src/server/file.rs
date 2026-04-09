use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use destack_ast::{Expression, LocalNodeId, NodeParentIndex};
use destack_compiler::CompilerOptions;
use destack_fir::format as fir_format;
use destack_formatter::{
    DestackFormatContext, DestackFormatOptions, format_file_source, statement_list,
};
use destack_parser::Parser;
use destack_service::{FileSnapshot, LanguageService as LspLanguageService, LanguageServiceError};
use destack_source::{
    DiagnosticSeverity, File, FileId, LanguageType, OverlayFileSystem, Span, WATCHABLE_FILE_TYPES,
};
use destack_workspace::{FormatterOptions, Repository, Revision};
use {destack_lsp_types as lsp, destack_query as query};

/// Globs for config files tracked by the LSP.
pub(super) const CONFIG_GLOBS: [&str; 2] = ["**/destack.json", "**/tsconfig*.json"];

/// Build a standalone file from a snapshot without mutating the repository.
pub(super) fn file_from_snapshot_for_diagnostics(snapshot: &FileSnapshot) -> Option<Arc<File>> {
    let content = snapshot.content.as_ref()?;
    let file = file_from_snapshot(snapshot, snapshot.id, content);

    Some(Arc::new(file))
}

/// Build a file from a snapshot payload.
fn file_from_snapshot(snapshot: &FileSnapshot, file_id: FileId, content: &str) -> File {
    File::from_text(
        file_id,
        snapshot.name.clone(),
        snapshot.uri.clone(),
        snapshot.path.clone(),
        snapshot.file_type,
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
    compiler_options: CompilerOptions,
) -> Result<LspLanguageService, LanguageServiceError> {
    LspLanguageService::with_options(
        repository,
        Some(overlay_fs),
        roots,
        compiler_options,
        None,
        None,
    )
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
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    diagnostics.len().hash(&mut hasher);
    for diagnostic in diagnostics {
        diagnostic.code.hash(&mut hasher);
        diagnostic.message.hash(&mut hasher);
        diagnostic.primary_span.span.start.hash(&mut hasher);
        diagnostic.primary_span.span.end.hash(&mut hasher);
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => 0u8,
            DiagnosticSeverity::Warning => 1u8,
            DiagnosticSeverity::Note => 2u8,
        };
        severity.hash(&mut hasher);
    }
    format!("{:x}", hasher.finish())
}

/// Format a file and return the formatted content.
/// Requires module AST state from the workspace graph.
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

    let language_type = LanguageType::from(file.ty);
    let format_options = DestackFormatOptions {
        language_type,
        ..formatter.into()
    };

    // resolve module state for this file
    let module_id = repository.module_id_for_file(revision, file_id).ok()??;
    repository.file(revision, file_id).ok().flatten()?;
    let ast = repository.ast(revision, module_id)?;

    // build format context from committed semantic state
    let side_span = Parser::compute_side_span_from_tree(&ast.tree);
    let strings = ast.strings.clone().into_immutable();
    let context = DestackFormatContext::new(
        format_options,
        file.as_ref(),
        &ast.tree,
        &ast.tokens,
        &ast.side_tokens,
        &side_span,
        &strings,
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
    let package = repository.package_for_path(revision, path).ok().flatten();
    if let Some(package) = package
        && let Ok(Some(package_options)) = repository.package_options(revision, package.id)
    {
        return package_options.formatter;
    }

    repository
        .workspace_options(revision)
        .ok()
        .flatten()
        .map(|options| options.package.formatter)
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
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file.clone(), language_type);
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
    let strings = parser.strings.into_immutable();
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
        &strings,
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

/// Build line start offsets for a string.
fn line_start_offsets(text: &str) -> Vec<u32> {
    let mut offsets = vec![0];
    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            offsets.push(index as u32 + 1);
        }
    }
    offsets
}

/// Convert an LSP position to a byte offset in raw text.
fn position_to_byte_in_text(
    text: &str,
    line_start_offsets: &[u32],
    position: &lsp::Position,
) -> Option<u32> {
    let line_index = position.line as usize;
    let line_start = *line_start_offsets
        .get(line_index)
        .unwrap_or(&(text.len() as u32));
    let next_start = line_start_offsets
        .get(line_index + 1)
        .copied()
        .unwrap_or(text.len() as u32);
    let slice = &text[line_start as usize..next_start as usize];

    // walk characters counting utf16 units until we reach target
    let mut utf16_units = 0u32;
    let mut byte_offset = 0usize;
    for ch in slice.chars() {
        if utf16_units >= position.character {
            break;
        }
        let ch_units = ch.len_utf16() as u32;
        if utf16_units + ch_units > position.character {
            break;
        }
        utf16_units += ch_units;
        byte_offset += ch.len_utf8();
    }

    // if we didn't reach the target character, clamp to end of line
    if utf16_units < position.character {
        return Some(next_start);
    }

    Some(line_start + byte_offset as u32)
}

/// Apply a batch of incremental text changes to the document text.
pub(super) fn apply_text_changes(
    text: &mut String,
    changes: &[lsp::TextDocumentContentChangeEvent],
) -> bool {
    for change in changes {
        let change_text = normalize_line_endings(change.text.clone());
        let Some(range) = &change.range else {
            *text = change_text;
            continue;
        };

        let line_offsets = line_start_offsets(text);
        let Some(start) = position_to_byte_in_text(text, &line_offsets, &range.start) else {
            return false;
        };
        let Some(end) = position_to_byte_in_text(text, &line_offsets, &range.end) else {
            return false;
        };
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let start = start as usize;
        let end = end as usize;
        if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            return false;
        }

        text.replace_range(start..end, &change_text);
    }

    true
}
