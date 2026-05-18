use std::env;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use destack_query as query;
use destack_query::{DocumentSymbol, SymbolKind};
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::core::CaseResult;
use crate::query::runner::span::{compute_line_starts, offset_to_line_col, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a document symbol parity check via an external tsserver tool.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(_expectation) = expectation else {
        return CaseResult::Skipped {
            reason: "no parity expectation".to_string(),
        };
    };

    // allow running parity checks only when explicitly configured
    let Some(parity_bin) = env::var_os("DESTACK_TSSERVER_PARITY_BIN") else {
        return CaseResult::Skipped {
            reason: "DESTACK_TSSERVER_PARITY_BIN is not set".to_string(),
        };
    };

    // compute the destack snapshot for the primary file
    let ctx = session.primary_module_context();
    let symbols = query::document_symbols(&ctx);
    let source = source_for_file(session, session.file_id);
    let snapshot = format_symbols_snapshot(&symbols, source, 0).join("\n");

    // build the external parity request payload
    let file_path = session
        .files
        .values()
        .find(|file| file.file_id == session.file_id)
        .map(|file| session.root.join(&file.name))
        .unwrap_or_else(|| session.root.join("main.ds"));
    let request = ParityRequest {
        query: "document_symbols".to_string(),
        root_dir: session.root.display().to_string(),
        file_path: file_path.display().to_string(),
        source: source.to_string(),
        snapshot,
    };

    let response = match run_external_parity(Path::new(&parity_bin), &request) {
        Ok(response) => response,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    // compare the snapshots and report a focused diff on mismatch
    if response.snapshot != request.snapshot {
        return CaseResult::Failed {
            message: format!(
                "tsserver parity mismatch\n\nexternal:\n{}\n\ndestack:\n{}",
                response.snapshot, request.snapshot
            ),
        };
    }

    CaseResult::Passed
}

#[derive(Debug, Serialize)]
struct ParityRequest {
    query: String,
    root_dir: String,
    file_path: String,
    source: String,
    snapshot: String,
}

#[derive(Debug, Deserialize)]
struct ParityResponse {
    snapshot: String,
}

/// Run the external parity command with a JSON payload.
fn run_external_parity(path: &Path, request: &ParityRequest) -> Result<ParityResponse, String> {
    // spawn the external process with piped stdin and stdout
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn parity bin '{}': {error}", path.display()))?;

    // write the request payload to stdin
    if let Some(mut stdin) = child.stdin.take() {
        let payload = serde_json::to_vec(request)
            .map_err(|error| format!("failed to serialize parity request: {error}"))?;
        stdin
            .write_all(&payload)
            .map_err(|error| format!("failed to write parity request: {error}"))?;
    }

    // wait for completion and collect output
    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to run parity bin '{}': {error}", path.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "parity bin '{}' failed: {}",
            path.display(),
            stderr.trim()
        ));
    }

    // parse the response payload
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<ParityResponse>(&stdout)
        .map_err(|error| format!("failed to parse parity response: {error}"))
}

/// Format a document symbol tree into a protocol shaped snapshot.
fn format_symbols_snapshot(symbols: &[DocumentSymbol], source: &str, indent: usize) -> Vec<String> {
    // compute line starts once for consistent span formatting
    let line_starts = compute_line_starts(source);

    // allocate snapshot lines for each symbol
    let mut lines = Vec::new();

    // render each symbol into the protocol snapshot
    for symbol in symbols {
        let indent_prefix = " ".repeat(indent);
        let kind = symbol_kind_name(symbol.kind);
        let range = format_span(&line_starts, symbol.range);
        let selection = format_span(&line_starts, symbol.selection_range);
        lines.push(format!(
            "{indent_prefix}{}({kind}) range={range} selection={selection}",
            symbol.name
        ));
        lines.extend(format_symbols_snapshot(
            &symbol.children,
            source,
            indent + 2,
        ));
    }
    lines
}

/// Format a span as a 1 based line and column range.
fn format_span(line_starts: &[u32], span: Span) -> String {
    let start = offset_to_line_col(line_starts, span.start);
    let end = offset_to_line_col(line_starts, span.end);
    format!("{}:{}-{}:{}", start.0, start.1, end.0, end.1)
}

/// Format a symbol kind as a lowercase name.
fn symbol_kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::File => "file",
        SymbolKind::Module => "module",
        SymbolKind::Namespace => "namespace",
        SymbolKind::Package => "package",
        SymbolKind::Class => "class",
        SymbolKind::Method => "method",
        SymbolKind::Property => "property",
        SymbolKind::Field => "field",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Enum => "enum",
        SymbolKind::Interface => "interface",
        SymbolKind::Function => "function",
        SymbolKind::Variable => "variable",
        SymbolKind::Constant => "constant",
        SymbolKind::String => "string",
        SymbolKind::Number => "number",
        SymbolKind::Boolean => "boolean",
        SymbolKind::Array => "array",
        SymbolKind::Object => "object",
        SymbolKind::Key => "key",
        SymbolKind::Null => "null",
        SymbolKind::EnumMember => "enum_member",
        SymbolKind::Struct => "struct",
        SymbolKind::Event => "event",
        SymbolKind::Operator => "operator",
        SymbolKind::TypeParameter => "type_parameter",
    }
}
