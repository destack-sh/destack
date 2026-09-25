use std::collections::HashSet;
use std::path::Path;

use clap::Args;
use tspp_parser::source_colorizer;
use tspp_source::{AnnotateOptions, AnnotateSpan, Span, Uri, annotate_file};
use tspp_workspace::{CommandRevision, QueryInput, QueryMatch, QueryPayload};

use crate::common::{
    CommandOptionsBuilder, CommandResult, DiagnosticFormat, FormatOptions, InputArgs, NodeTypeArg,
    ProgramArgs, ReportArgs, ReportFormat, command_error, ensure_no_watch_or_dev,
    finish_diagnostic_command, parse_required_command_payload, report_error,
    run_workspace_command_or_report,
};
use crate::console;

/// Arguments for the structural query command.
#[derive(Args, Debug, Clone)]
pub struct QueryArgs {
    /// Structural TS++ pattern.
    #[arg(value_name = "PATTERN")]
    pub pattern: String,

    /// Input arguments.
    #[command(flatten)]
    pub input: InputArgs,

    /// Semantic predicate applied to structural captures.
    #[arg(long = "where", value_name = "PREDICATE")]
    pub predicates: Vec<String>,

    /// Contextual pattern root type.
    #[arg(long, value_enum)]
    pub kind: Option<NodeTypeArg>,

    /// Print only the exact values of these named captures.
    #[arg(long = "capture", value_name = "NAME", conflicts_with = "count")]
    pub captures: Vec<String>,

    /// Prefix projected capture values with their source file.
    #[arg(short = 'H', long, requires = "captures", conflicts_with = "count")]
    pub with_filename: bool,

    /// Print only the number of selected roots.
    #[arg(long, conflicts_with_all = ["captures", "one_line"])]
    pub count: bool,

    /// Escape line terminators so every result occupies one physical line.
    #[arg(long, conflicts_with = "count")]
    pub one_line: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

impl QueryArgs {
    /// Validate output options that do not depend on query results.
    fn validate_output(&self) -> Result<(), String> {
        if self.report.is_json() && (self.count || self.one_line || !self.captures.is_empty()) {
            return Err(
                "--count, --capture, and --one-line are not supported with JSON output".to_string(),
            );
        }

        Ok(())
    }

    /// Render structural matches in the requested human or plain-text format.
    fn render_matches(
        &self,
        result: &CommandResult,
        payload: QueryPayload,
        captures: &[String],
    ) -> Result<String, String> {
        let cwd = self
            .program
            .effective_cwd()
            .map_err(|error| error.to_string())?;

        // count mode suppresses individual results
        if self.count {
            return match self.report.format() {
                ReportFormat::Human => Ok(Self::render_summary(&payload)),
                ReportFormat::Text => Ok(format!("{}\n", payload.matches.len())),
                ReportFormat::Json => Ok(String::new()),
            };
        }

        // capture selection projects values regardless of the default report format
        if !captures.is_empty() {
            return Self::render_text(payload, &cwd, captures, self.with_filename, self.one_line);
        }

        // one line mode is an explicit plain-text layout
        if self.one_line {
            return Self::render_text(payload, &cwd, captures, false, true);
        }

        match self.report.format() {
            ReportFormat::Human => Self::render_human(result, payload, &cwd),
            ReportFormat::Text => Self::render_text(payload, &cwd, captures, false, false),
            ReportFormat::Json => Ok(String::new()),
        }
    }

    /// Validate requested output projections against the compiled pattern.
    fn selected_captures(&self, payload: &QueryPayload) -> Result<Vec<String>, String> {
        let mut captures = Vec::with_capacity(self.captures.len());
        for capture in &self.captures {
            let capture = capture.trim_start_matches('$');
            if capture.is_empty() {
                return Err("capture name must not be empty".to_string());
            }
            if captures.iter().any(|selected| selected == capture) {
                return Err(format!("capture ${capture} was selected more than once"));
            }
            if !payload.captures.iter().any(|declared| declared == capture) {
                let available = payload
                    .captures
                    .iter()
                    .map(|capture| format!("${capture}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                if available.is_empty() {
                    return Err("pattern declares no named captures".to_string());
                }

                return Err(format!(
                    "pattern does not declare capture ${capture}; available captures: {available}"
                ));
            }
            captures.push(capture.to_string());
        }

        Ok(captures)
    }

    /// Render matches as syntax-colored source annotations.
    fn render_human(
        result: &CommandResult,
        payload: QueryPayload,
        cwd: &Path,
    ) -> Result<String, String> {
        let mut options = AnnotateOptions::new()
            .with_context_lines(0, 0)
            .with_colorizer(source_colorizer());
        options.use_color = console::color_enabled(console::Stream::Stdout);
        let summary = Self::render_summary(&payload);
        let mut output = String::new();

        // render every match against the exact source revision returned by Workspace
        for (index, pattern_match) in payload.matches.into_iter().enumerate() {
            let source = result.files.get(&pattern_match.span.file).ok_or_else(|| {
                format!(
                    "query result references missing source file {:?}",
                    pattern_match.span.file
                )
            })?;
            let mut source = source.as_ref().clone();
            source.uri = Uri::from_string(Self::display_path(&pattern_match, cwd));
            let anchor = Span::new(
                pattern_match.span.file,
                pattern_match.span.start,
                pattern_match.span.start,
            );
            let mut spans = vec![
                AnnotateSpan::primary(anchor, ""),
                AnnotateSpan::secondary(pattern_match.span, "match"),
            ];

            // label every capture
            for capture in &pattern_match.captures {
                for value in &capture.values {
                    spans.push(AnnotateSpan::primary(
                        value.span,
                        format!("${}", capture.name),
                    ));
                }
            }

            let annotation = annotate_file(&source, &spans, options.clone())
                .map_err(|error| format!("failed to render query match: {error}"))?;
            if index > 0 {
                output.push('\n');
            }
            output.push_str(&annotation);
        }

        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&summary);

        Ok(output)
    }

    /// Render matches as stable path, position, and source rows.
    pub(crate) fn render_text(
        payload: QueryPayload,
        cwd: &Path,
        captures: &[String],
        with_filename: bool,
        is_one_line: bool,
    ) -> Result<String, String> {
        let mut output = String::new();

        // project capture values in match and request order
        if !captures.is_empty() {
            for pattern_match in payload.matches {
                let path = with_filename.then(|| Self::display_path(&pattern_match, cwd));
                for name in captures {
                    let capture = pattern_match
                        .captures
                        .iter()
                        .find(|capture| capture.name == name.as_str())
                        .ok_or_else(|| format!("query match is missing capture ${name}"))?;
                    for value in &capture.values {
                        Self::write_text(&mut output, &value.text, path.as_deref(), is_one_line);
                    }
                }
            }

            return Ok(output);
        }

        // print complete selected roots with their source locations
        for pattern_match in payload.matches {
            let path = Self::display_path(&pattern_match, cwd);
            let location = format!("{path}:{}:{}", pattern_match.line, pattern_match.column);
            if is_one_line {
                let text = Self::escape_line_terminators(&pattern_match.text);
                output.push_str(&format!("{location}: {text}\n"));
            } else if pattern_match.text.contains('\n') {
                output.push_str(&location);
                output.push('\n');
                output.push_str(&pattern_match.text);
                if !pattern_match.text.ends_with('\n') {
                    output.push('\n');
                }
            } else {
                output.push_str(&format!("{location}: {}\n", pattern_match.text));
            }
        }

        Ok(output)
    }

    /// Append one projected source value in plain or one-line form.
    fn write_text(output: &mut String, text: &str, path: Option<&str>, is_one_line: bool) {
        if let Some(path) = path {
            output.push_str(path);
            output.push(':');
        }

        if is_one_line {
            output.push_str(&Self::escape_line_terminators(text));
            output.push('\n');
        } else {
            output.push_str(text);
            if !text.ends_with('\n') {
                output.push('\n');
            }
        }
    }

    /// Escape source line terminators without changing any other authored bytes.
    fn escape_line_terminators(text: &str) -> String {
        text.replace('\r', "\\r").replace('\n', "\\n")
    }

    /// Render the total selected roots and distinct source files.
    fn render_summary(payload: &QueryPayload) -> String {
        let match_count = payload.matches.len();
        let file_count = payload
            .matches
            .iter()
            .map(|pattern_match| pattern_match.span.file)
            .collect::<HashSet<_>>()
            .len();
        let match_noun = if match_count == 1 { "match" } else { "matches" };
        let file_noun = if file_count == 1 { "file" } else { "files" };
        let summary = format!("{match_count} {match_noun} in {file_count} {file_noun}");

        format!("{}\n", console::dim(&summary))
    }

    /// Return one query match path relative to the effective working directory.
    fn display_path(pattern_match: &QueryMatch, cwd: &Path) -> String {
        pattern_match
            .path
            .as_deref()
            .map(|path| path.strip_prefix(cwd).unwrap_or(path))
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| pattern_match.uri.to_string())
    }
}

/// Query source files with a structural pattern.
pub async fn run(args: &QueryArgs) -> i32 {
    let mut args = args.clone();
    if let Err(error) = args.validate_output() {
        return report_error("query", &args.report, &error);
    }

    match args.input.take_directory_root() {
        Ok(Some(root)) => {
            if let Err(error) = args.program.select_workspace_root(root) {
                return report_error("query", &args.report, &error.to_string());
            }
        }
        Ok(None) => {}
        Err(error) => return report_error("query", &args.report, &error.to_string()),
    }
    if let Some(code) = ensure_no_watch_or_dev("query", &args.program, &args.report) {
        return code;
    }

    // build the integrated workspace request
    let sources = match args.input.explicit_sources() {
        Ok(sources) => sources,
        Err(error) => return report_error("query", &args.report, &error.to_string()),
    };
    let inputs = match args.input.command_inputs(&sources) {
        Ok(inputs) => inputs,
        Err(error) => return report_error("query", &args.report, &error.to_string()),
    };
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common,
        Err(error) => return report_error("query", &args.report, &error.to_string()),
    }
    .inputs(inputs)
    .config_inputs(!args.input.has_input())
    .build();
    let include_sources = matches!(args.report.format(), ReportFormat::Human)
        && !args.count
        && !args.one_line
        && args.captures.is_empty();
    let request = QueryInput {
        pattern: std::mem::take(&mut args.pattern),
        kind: args.kind.map(Into::into),
        predicates: std::mem::take(&mut args.predicates),
        include_sources,
        ..(CommandRevision::Current, common).into()
    };

    // execute through local or remote Workspace uniformly
    let result = match run_workspace_command_or_report(
        "query",
        &args.report,
        &args.program,
        async |workspace, progress| {
            let output = workspace
                .query(request, progress)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(output)
        },
    )
    .await
    {
        Ok(result) => result,
        Err(code) => return code,
    };
    let (payload, payload_value) = match parse_required_command_payload::<QueryPayload>(
        "query",
        &args.report,
        Some(&result.response.data),
        "query",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };
    let captures = if result.response.exit_code == 0 {
        match args.selected_captures(&payload) {
            Ok(captures) => captures,
            Err(error) => return report_error("query", &args.report, &error),
        }
    } else {
        Vec::new()
    };

    // render diagnostics and the structured JSON payload
    let json_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..FormatOptions::default()
    };
    let text_options = FormatOptions::default();
    let exit_code = finish_diagnostic_command(
        "query",
        &args.report,
        &result,
        &json_options,
        &text_options,
        None,
        None,
        Some(payload_value),
    );
    if args.report.is_json() {
        return exit_code;
    }
    if exit_code != 0 {
        result.emit_timings(None);
        return exit_code;
    }

    // print structural results separately from command diagnostics
    let output = match args.render_matches(&result, payload, &captures) {
        Ok(output) => output,
        Err(error) => return report_error("query", &args.report, &error),
    };
    console::write(&output);
    result.emit_timings(None);

    exit_code
}
