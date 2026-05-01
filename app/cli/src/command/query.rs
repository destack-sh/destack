use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use clap::Args;
use destack_query::{
    QueryExecutionMode, QueryMethod, QueryMethodId, QueryRequest, QueryRequestEnvelope, assist,
    navigation, parse_query_request, query_method, query_methods, refactor,
};
use destack_source::Uri;
use serde_json::Value;

use crate::common::ProgramArgs;
use crate::console;
use crate::error::CliError;
use crate::pipeline::daemon::ProtocolDaemonClient;
use crate::pipeline::watch::watch_roots;

/// Arguments for the query command.
#[derive(Args, Debug, Clone)]
pub struct QueryArgs {
    /// Query command or method name.
    #[arg(value_name = "METHOD_OR_COMMAND")]
    pub method: Option<String>,

    /// Query command argument (for help or schema).
    #[arg(value_name = "ARG")]
    pub method_arg: Option<String>,

    /// Read query input from stdin.
    #[arg(long)]
    pub stdin: bool,

    /// Inline params JSON for method mode.
    #[arg(long)]
    pub params: Option<String>,

    /// Query input file path (JSON).
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Query file path for position-based requests.
    #[arg(long, value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// Query URI for position-based requests.
    #[arg(long, value_name = "URI")]
    pub uri: Option<String>,

    /// 1-based line for position-based requests.
    #[arg(long)]
    pub line: Option<u32>,

    /// 1-based column for position-based requests.
    #[arg(long)]
    pub column: Option<u32>,

    /// Byte offset for position-based requests.
    #[arg(long)]
    pub offset: Option<u32>,

    /// Exclude declaration in find references when using position args.
    #[arg(long)]
    pub exclude_declaration: bool,

    /// New name for rename when using position args.
    #[arg(long)]
    pub new_name: Option<String>,

    /// Pretty print the JSON response.
    #[arg(long)]
    pub pretty: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,
}

/// Serializable metadata for query methods.
#[derive(serde::Serialize)]
struct QueryMethodInfo {
    /// The canonical method name.
    name: &'static str,
    /// The alias names accepted by the CLI.
    aliases: Vec<&'static str>,
    /// The method category label.
    category: &'static str,
    /// The method summary.
    summary: &'static str,
    /// The params type name.
    params_type: &'static str,
    /// The result type name.
    result_type: &'static str,
}

/// Serializable schema metadata for query methods.
#[derive(serde::Serialize)]
struct QuerySchemaInfo {
    /// The canonical method name.
    name: &'static str,
    /// The params type name.
    params_type: &'static str,
    /// The result type name.
    result_type: &'static str,
}

/// Encode JSON output for query responses.
fn encode_output<T: serde::Serialize>(value: &T, pretty: bool) -> Result<String, String> {
    // serialize the response payload
    let output = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };

    output.map_err(|error| format!("failed to encode output: {error}"))
}

/// Build the metadata output for a query method.
fn method_info(method: &QueryMethod) -> QueryMethodInfo {
    // collect the alias list
    let aliases = method.aliases.to_vec();

    QueryMethodInfo {
        name: method.name,
        aliases,
        category: method.category.as_str(),
        summary: method.summary,
        params_type: method.params_type,
        result_type: method.result_type,
    }
}

/// Build the schema output for a query method.
fn method_schema(method: &QueryMethod) -> QuerySchemaInfo {
    QuerySchemaInfo {
        name: method.name,
        params_type: method.params_type,
        result_type: method.result_type,
    }
}

/// Normalize a query command name.
fn normalize_command_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

/// Parse JSON input into a value.
fn parse_json_value(label: &str, input: &str) -> Result<Value, String> {
    serde_json::from_str(input).map_err(|error| format!("invalid {label} json: {error}"))
}

/// Resolved position arguments for queries.
struct PositionArgs {
    uri: Uri,
    offset: u32,
}

/// Resolve the target URI for a query.
fn resolve_target_uri(args: &QueryArgs) -> Result<Option<Uri>, String> {
    let has_file = args.file.is_some();
    let has_uri = args.uri.is_some();
    if has_file && has_uri {
        return Err("use --file or --uri, not both".to_string());
    }

    if let Some(path) = args.file.as_ref() {
        return Ok(Some(Uri::from_path(path)));
    }

    if let Some(uri) = args.uri.as_ref() {
        return Ok(Some(Uri::from_string(uri)));
    }

    Ok(None)
}

/// Compute a byte offset from a 1-based line and column.
fn offset_from_line_column(content: &str, line: u32, column: u32) -> Result<u32, String> {
    if line == 0 || column == 0 {
        return Err("line and column must be >= 1".to_string());
    }

    let mut current_line = 1u32;
    let mut line_start = 0usize;
    for (idx, byte) in content.bytes().enumerate() {
        if byte == b'\n' {
            if current_line == line {
                break;
            }
            current_line += 1;
            line_start = idx + 1;
        }
    }

    if line > current_line {
        return Err(format!("line {line} is out of range"));
    }

    let line_end = content[line_start..]
        .find('\n')
        .map(|idx| line_start + idx)
        .unwrap_or_else(|| content.len());
    let mut line_len = line_end.saturating_sub(line_start);
    if line_len > 0 && content.as_bytes()[line_end.saturating_sub(1)] == b'\r' {
        line_len = line_len.saturating_sub(1);
    }

    let col_index = (column - 1) as usize;
    if col_index > line_len {
        return Err(format!("column {column} is out of range"));
    }

    Ok((line_start + col_index) as u32)
}

/// Resolve position arguments into a URI and offset.
fn resolve_position_args(args: &QueryArgs) -> Result<Option<PositionArgs>, String> {
    let uri = resolve_target_uri(args)?;

    let has_position = args.offset.is_some() || args.line.is_some() || args.column.is_some();
    if !has_position {
        return Ok(None);
    }

    let Some(uri) = uri else {
        return Err("position args require --file or --uri".to_string());
    };

    let offset = if let Some(offset) = args.offset {
        offset
    } else {
        let line = args
            .line
            .ok_or_else(|| "position args require --line".to_string())?;
        let column = args
            .column
            .ok_or_else(|| "position args require --column".to_string())?;
        let path = uri
            .to_path()
            .ok_or_else(|| "failed to resolve uri to a path".to_string())?;
        let contents = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        offset_from_line_column(&contents, line, column)?
    };

    Ok(Some(PositionArgs { uri, offset }))
}

/// Read query input from stdin or a file.
fn read_input_string(
    args: &QueryArgs,
    input_path: Option<&Path>,
) -> Result<Option<String>, String> {
    // validate input selection
    let is_input_conflict = args.stdin && input_path.is_some();
    if is_input_conflict {
        return Err("use --stdin or INPUT, not both".to_string());
    }

    // resolve stdin usage when piped
    let mut is_reading_stdin = args.stdin;
    if input_path.is_none() && !is_reading_stdin && !std::io::stdin().is_terminal() {
        is_reading_stdin = true;
    }

    // read from stdin
    if is_reading_stdin {
        let mut buffer = String::new();
        if let Err(error) = std::io::stdin().read_to_string(&mut buffer) {
            return Err(format!("failed to read stdin: {error}"));
        }
        return Ok(Some(buffer));
    }

    // read from the input file
    if let Some(path) = input_path {
        let contents = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

        return Ok(Some(contents));
    }

    Ok(None)
}

/// Resolve the input path for method params.
fn resolve_method_input_path(args: &QueryArgs) -> Result<Option<PathBuf>, String> {
    // validate input selection
    let is_input_conflict = args.input.is_some() && args.method_arg.is_some();
    if is_input_conflict {
        return Err("use INPUT or ARG for params, not both".to_string());
    }

    // prefer the explicit input path
    if let Some(path) = args.input.as_ref() {
        return Ok(Some(path.clone()));
    }

    // fall back to the method arg
    if let Some(path) = args.method_arg.as_ref() {
        return Ok(Some(PathBuf::from(path)));
    }

    Ok(None)
}

/// Resolve the input path for raw query input.
fn resolve_raw_input_path(
    args: &QueryArgs,
    method_name: Option<&str>,
) -> Result<Option<PathBuf>, String> {
    // validate unused arguments
    let is_extra_arg_present = method_name.is_some() && args.method_arg.is_some();
    if is_extra_arg_present {
        return Err("unknown query command or method".to_string());
    }

    // prefer the explicit input path
    if let Some(path) = args.input.as_ref() {
        return Ok(Some(path.clone()));
    }

    // fall back to the method slot
    if let Some(name) = method_name {
        return Ok(Some(PathBuf::from(name)));
    }

    Ok(None)
}

/// Run a handler with a daemon connection.
fn run_with_daemon<F>(args: &QueryArgs, handler: F) -> Result<String, String>
where
    F: FnOnce(&ProtocolDaemonClient, &Path) -> Result<String, String>,
{
    // initialize the daemon connection
    let session = args.program.setup();
    let roots = watch_roots(&args.program, &session);
    let Some(root) = roots.first().cloned() else {
        return Err("workspace roots are empty".to_string());
    };
    let daemon = ProtocolDaemonClient::new(
        session,
        args.program.workers as usize,
        None,
        roots,
        &args.program,
    )
    .map_err(|error| error.to_string())?;

    // execute the query handler
    let output = handler(&daemon, &root);

    // close the daemon connection
    daemon.shutdown();

    output
}

/// Execute a query request.
fn run_query_request(
    daemon: &ProtocolDaemonClient,
    root: &Path,
    mut request: QueryRequestEnvelope,
    pretty: bool,
) -> Result<String, String> {
    // provide the current revision when write queries omit a precondition
    if request.expected_revision.is_none()
        && request.request.execution_mode() == QueryExecutionMode::Write
    {
        let revision = daemon
            .run_current_revision(root)
            .map_err(|error: CliError| error.to_string())?;
        request.expected_revision = Some(revision);
    }

    // send the query request
    let response = daemon
        .run_query(root, request)
        .map_err(|error| error.to_string())?;

    // encode the response payload
    encode_output(&response, pretty)
}

/// Collect query batch indices that need an implicit write revision precondition.
fn missing_write_precondition_indices(requests: &[QueryRequestEnvelope]) -> Vec<usize> {
    requests
        .iter()
        .enumerate()
        .filter_map(|(index, request)| {
            if request.expected_revision.is_none()
                && request.request.execution_mode() == QueryExecutionMode::Write
            {
                return Some(index);
            }

            None
        })
        .collect()
}

/// Execute a batch of query requests.
fn run_query_batch_request(
    daemon: &ProtocolDaemonClient,
    root: &Path,
    mut requests: Vec<QueryRequestEnvelope>,
    pretty: bool,
) -> Result<String, String> {
    // collect write queries that are missing a revision precondition
    let missing_write_indices = missing_write_precondition_indices(&requests);

    // provide one shared revision precondition for implicit write requests
    if !missing_write_indices.is_empty() {
        let revision = daemon
            .run_current_revision(root)
            .map_err(|error: CliError| error.to_string())?;
        for index in missing_write_indices {
            requests[index].expected_revision = Some(revision);
        }
    }

    // send the query batch
    let response = daemon
        .run_query_batch(root, requests)
        .map_err(|error| error.to_string())?;

    // encode the response payload
    encode_output(&response, pretty)
}

/// Execute the list command.
fn run_list_mode(pretty: bool) -> Result<String, String> {
    // collect method metadata
    let methods: Vec<QueryMethodInfo> = query_methods().iter().map(method_info).collect();

    // encode the response payload
    encode_output(&methods, pretty)
}

/// Execute the help command.
fn run_help_mode(method_name: &str, pretty: bool) -> Result<String, String> {
    // resolve the query method
    let method =
        query_method(method_name).ok_or_else(|| format!("unknown query method: {method_name}"))?;

    // build the help output
    let info = method_info(method);

    // encode the response payload
    encode_output(&info, pretty)
}

/// Execute the schema command.
fn run_schema_mode(method_name: &str, pretty: bool) -> Result<String, String> {
    // resolve the query method
    let method =
        query_method(method_name).ok_or_else(|| format!("unknown query method: {method_name}"))?;

    // build the schema output
    let schema = method_schema(method);

    // encode the response payload
    encode_output(&schema, pretty)
}

/// Build a query request from a position-based CLI invocation.
fn build_position_request(
    method: &QueryMethod,
    position: PositionArgs,
    args: &QueryArgs,
) -> Result<QueryRequest, String> {
    let uri = position.uri;
    let offset = position.offset;

    let request = match method.id {
        QueryMethodId::Completion => QueryRequest::Completion(assist::CompletionRequest {
            uri,
            offset,
            trigger: assist::CompletionTrigger::Invoked,
            include_imports: true,
        }),
        QueryMethodId::Hover => QueryRequest::Hover(assist::HoverRequest { uri, offset }),
        QueryMethodId::SignatureHelp => {
            QueryRequest::SignatureHelp(assist::SignatureHelpRequest { uri, offset })
        }
        QueryMethodId::DocumentHighlight => {
            QueryRequest::DocumentHighlight(navigation::DocumentHighlightRequest { uri, offset })
        }
        QueryMethodId::GotoDefinition => {
            QueryRequest::GotoDefinition(navigation::GotoDefinitionRequest { uri, offset })
        }
        QueryMethodId::GotoDeclaration => {
            QueryRequest::GotoDeclaration(navigation::GotoDeclarationRequest { uri, offset })
        }
        QueryMethodId::GotoTypeDefinition => {
            QueryRequest::GotoTypeDefinition(navigation::GotoTypeDefinitionRequest { uri, offset })
        }
        QueryMethodId::GotoImplementation => {
            QueryRequest::GotoImplementation(navigation::GotoImplementationRequest { uri, offset })
        }
        QueryMethodId::FindReferences => {
            QueryRequest::FindReferences(navigation::FindReferencesRequest {
                uri,
                offset,
                include_declaration: !args.exclude_declaration,
            })
        }
        QueryMethodId::PrepareCallHierarchy => {
            QueryRequest::PrepareCallHierarchy(navigation::PrepareCallHierarchyRequest {
                uri,
                offset,
            })
        }
        QueryMethodId::PrepareTypeHierarchy => {
            QueryRequest::PrepareTypeHierarchy(navigation::PrepareTypeHierarchyRequest {
                uri,
                offset,
            })
        }
        QueryMethodId::PrepareRename => {
            QueryRequest::PrepareRename(refactor::PrepareRenameRequest { uri, offset })
        }
        QueryMethodId::Rename => {
            let new_name = args
                .new_name
                .clone()
                .ok_or_else(|| "rename requires --new-name when using position args".to_string())?;
            QueryRequest::Rename(refactor::RenameRequest {
                uri,
                offset,
                new_name,
            })
        }
        _ => {
            return Err("method requires explicit params, use --params or --stdin".to_string());
        }
    };

    Ok(request)
}

/// Build a query request from a document URI when no position is provided.
fn build_uri_request(method: &QueryMethod, uri: Uri) -> Result<QueryRequest, String> {
    let request = match method.id {
        QueryMethodId::DocumentSymbols => {
            QueryRequest::DocumentSymbols(navigation::DocumentSymbolsRequest { uri })
        }
        QueryMethodId::DocumentLinks => {
            QueryRequest::DocumentLinks(navigation::DocumentLinksRequest { uri })
        }
        QueryMethodId::CodeLenses => QueryRequest::CodeLenses(assist::CodeLensesRequest { uri }),
        QueryMethodId::FoldingRanges => {
            QueryRequest::FoldingRanges(assist::FoldingRangesRequest { uri })
        }
        QueryMethodId::SemanticTokens => {
            QueryRequest::SemanticTokens(assist::SemanticTokensRequest { uri })
        }
        _ => {
            return Err(
                "method requires position or explicit params, use --params or --stdin".to_string(),
            );
        }
    };

    Ok(request)
}

/// Execute a query method.
fn run_method_mode(
    args: &QueryArgs,
    method_name: &str,
    input_path: Option<&Path>,
) -> Result<String, String> {
    // validate params input selection
    let is_inline_params = args.params.is_some();
    let is_input_present = args.stdin || input_path.is_some();
    let has_position_args = args.file.is_some()
        || args.uri.is_some()
        || args.line.is_some()
        || args.column.is_some()
        || args.offset.is_some()
        || args.new_name.is_some()
        || args.exclude_declaration;
    if is_inline_params && is_input_present {
        return Err("use --params or --stdin or INPUT, not both".to_string());
    }
    if (is_inline_params || is_input_present) && has_position_args {
        return Err("use --params/--stdin or position args, not both".to_string());
    }

    // resolve the query method metadata
    let method =
        query_method(method_name).ok_or_else(|| format!("unknown query method: {method_name}"))?;

    // build request from inline or input params when provided
    let request = if is_inline_params || is_input_present {
        let params = if let Some(params) = args.params.as_ref() {
            parse_json_value("params", params)?
        } else {
            let input = read_input_string(args, input_path)?;
            let Some(input) = input else {
                return Err("no params provided, use --params or --stdin or INPUT".to_string());
            };
            parse_json_value("params", &input)?
        };
        parse_query_request(method_name, params).map_err(|error| error.to_string())?
    } else if let Some(position) = resolve_position_args(args)? {
        build_position_request(method, position, args)?
    } else if let Some(uri) = resolve_target_uri(args)? {
        build_uri_request(method, uri)?
    } else {
        return Err("no params provided, use --params/--stdin or position args".to_string());
    };
    let envelope = QueryRequestEnvelope {
        request,
        expected_revision: None,
    };

    // execute the query
    run_with_daemon(args, |daemon, root| {
        run_query_request(daemon, root, envelope, args.pretty)
    })
}

/// Execute raw query input.
fn run_raw_mode(args: &QueryArgs, input_path: Option<&Path>) -> Result<String, String> {
    // read the input string
    let input = read_input_string(args, input_path)?;

    // ensure input is available
    let Some(input) = input else {
        return Err("no input provided, use --stdin or INPUT".to_string());
    };

    // parse the input json
    let value = parse_json_value("input", &input)?;

    // execute the query
    run_with_daemon(args, |daemon, root| {
        // run the query batch
        if value.is_array() {
            let requests: Vec<QueryRequestEnvelope> = serde_json::from_value(value)
                .map_err(|error| format!("invalid query batch: {error}"))?;

            return run_query_batch_request(daemon, root, requests, args.pretty);
        }

        // otherwise run a single query
        let request: QueryRequestEnvelope = serde_json::from_value(value)
            .map_err(|error| format!("invalid query request: {error}"))?;

        run_query_request(daemon, root, request, args.pretty)
    })
}

/// Print the query output or an error.
fn finish_output(output: Result<String, String>) -> i32 {
    // handle output errors
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            console::error(&error);
            return 1;
        }
    };

    println!("{output}");

    0
}

/// Execute root queries.
pub fn run(args: &QueryArgs) -> i32 {
    // resolve the command name
    let method_name = args.method.as_deref();
    let command_name = method_name.map(normalize_command_name);

    // handle the list command
    if command_name.as_deref() == Some("list") {
        // validate unsupported input
        let is_input_present = args.stdin
            || args.params.is_some()
            || args.method_arg.is_some()
            || args.input.is_some()
            || args.file.is_some()
            || args.uri.is_some()
            || args.line.is_some()
            || args.column.is_some()
            || args.offset.is_some()
            || args.exclude_declaration
            || args.new_name.is_some();
        if is_input_present {
            console::error("list does not accept input");
            return 1;
        }

        // run the list command
        let output = run_list_mode(args.pretty);
        return finish_output(output);
    }

    // handle the help command
    if command_name.as_deref() == Some("help") {
        // validate unsupported input
        let is_input_present = args.stdin
            || args.params.is_some()
            || args.input.is_some()
            || args.file.is_some()
            || args.uri.is_some()
            || args.line.is_some()
            || args.column.is_some()
            || args.offset.is_some()
            || args.exclude_declaration
            || args.new_name.is_some();
        if is_input_present {
            console::error("help does not accept input");
            return 1;
        }

        // ensure a method name is provided
        let Some(target_method) = args.method_arg.as_deref() else {
            console::error("help requires a method name");
            return 1;
        };

        // run the help command
        let output = run_help_mode(target_method, args.pretty);
        return finish_output(output);
    }

    // handle the schema command
    if command_name.as_deref() == Some("schema") {
        // validate unsupported input
        let is_input_present = args.stdin
            || args.params.is_some()
            || args.input.is_some()
            || args.file.is_some()
            || args.uri.is_some()
            || args.line.is_some()
            || args.column.is_some()
            || args.offset.is_some()
            || args.exclude_declaration
            || args.new_name.is_some();
        if is_input_present {
            console::error("schema does not accept input");
            return 1;
        }

        // ensure a method name is provided
        let Some(target_method) = args.method_arg.as_deref() else {
            console::error("schema requires a method name");
            return 1;
        };

        // run the schema command
        let output = run_schema_mode(target_method, args.pretty);
        return finish_output(output);
    }

    // handle query method mode
    if let Some(method_name) = method_name {
        // resolve a known query method
        if query_method(method_name).is_some() {
            // resolve the params input path
            let input_path = match resolve_method_input_path(args) {
                Ok(path) => path,
                Err(error) => {
                    console::error(&error);
                    return 1;
                }
            };

            // run the query method
            let output = run_method_mode(args, method_name, input_path.as_deref());
            return finish_output(output);
        }
    }

    // reject params without a method
    if args.params.is_some() {
        console::error("--params requires a method name");
        return 1;
    }

    // reject position args without a method
    let has_position_args = args.file.is_some()
        || args.uri.is_some()
        || args.line.is_some()
        || args.column.is_some()
        || args.offset.is_some()
        || args.exclude_declaration
        || args.new_name.is_some();
    if has_position_args {
        console::error("position args require a method name");
        return 1;
    }

    // resolve raw input path
    let input_path = match resolve_raw_input_path(args, method_name) {
        Ok(path) => path,
        Err(error) => {
            console::error(&error);
            return 1;
        }
    };

    // run raw query input
    let output = run_raw_mode(args, input_path.as_deref());
    finish_output(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Normalizes query command names.
    #[test]
    fn test_query_normalize_command_name() {
        // normalize a mixed case name
        let normalized = normalize_command_name("  Completion ");

        // assert normalized output
        assert_eq!(normalized, "completion");
    }

    /// Resolves method input paths from args.
    #[test]
    fn test_query_resolve_method_input_path() {
        // build args with method arg only
        let args = QueryArgs {
            method: Some("completion".to_string()),
            method_arg: Some("params.json".to_string()),
            stdin: false,
            params: None,
            input: None,
            file: None,
            uri: None,
            line: None,
            column: None,
            offset: None,
            exclude_declaration: false,
            new_name: None,
            pretty: false,
            program: ProgramArgs::default(),
        };

        // resolve the input path
        let path = resolve_method_input_path(&args).expect("method input should resolve");

        // assert the path matches the method arg
        assert_eq!(path, Some(PathBuf::from("params.json")));
    }

    /// Rejects extra args for raw mode inputs.
    #[test]
    fn test_query_resolve_raw_input_path_rejects_extra_args() {
        // build args with extra method arg
        let args = QueryArgs {
            method: Some("raw.json".to_string()),
            method_arg: Some("extra.json".to_string()),
            stdin: false,
            params: None,
            input: None,
            file: None,
            uri: None,
            line: None,
            column: None,
            offset: None,
            exclude_declaration: false,
            new_name: None,
            pretty: false,
            program: ProgramArgs::default(),
        };

        // attempt to resolve raw input
        let result = resolve_raw_input_path(&args, args.method.as_deref());

        // assert the extra arg is rejected
        assert!(result.is_err());
    }

    /// Computes byte offsets from line and column.
    #[test]
    fn test_query_offset_from_line_column() {
        let content = "first\nsecond\nthird";

        let offset = offset_from_line_column(content, 1, 1).expect("offset");
        assert_eq!(offset, 0);

        let offset = offset_from_line_column(content, 2, 1).expect("offset");
        assert_eq!(offset, 6);

        let offset = offset_from_line_column(content, 2, 3).expect("offset");
        assert_eq!(offset, 8);
    }

    /// Collects only write requests without explicit revisions.
    #[test]
    fn test_query_missing_write_precondition_indices() {
        // build a mixed query batch
        let requests = vec![
            QueryRequestEnvelope {
                expected_revision: None,
                request: QueryRequest::Hover(assist::HoverRequest {
                    uri: Uri::from_string("/workspace/main.ds"),
                    offset: 1,
                }),
            },
            QueryRequestEnvelope {
                expected_revision: None,
                request: QueryRequest::RenameFiles(refactor::RenameFilesRequest {
                    renames: Vec::new(),
                }),
            },
            QueryRequestEnvelope {
                expected_revision: Some(destack_workspace::Revision::from_test_value(9)),
                request: QueryRequest::RenameFiles(refactor::RenameFilesRequest {
                    renames: Vec::new(),
                }),
            },
            QueryRequestEnvelope {
                expected_revision: None,
                request: QueryRequest::Rename(refactor::RenameRequest {
                    uri: Uri::from_string("/workspace/main.ds"),
                    offset: 1,
                    new_name: "next".to_string(),
                }),
            },
        ];

        // assert write requests without revisions are selected
        assert_eq!(missing_write_precondition_indices(&requests), vec![1, 3]);
    }
}
