use serde_json as json;
use tspp_repository::TraceView;
use tspp_serde as serde;
use tspp_workspace::{CommandInput, CommandOptions, CommandTargetOverrides, ManifestOverride};

use crate::common::ProgramArgs;
use crate::common::program::{
    FormatterOptionsArgs, IndentStyleArg, LineEndingArg, LinterOptionsArgs,
};
use crate::diagnostic::ConsoleResult;

/// Builder for common workspace command options.
#[derive(Debug, Clone)]
pub(crate) struct CommandOptionsBuilder {
    options: CommandOptions,
}

impl CommandOptionsBuilder {
    /// Create a builder seeded with program defaults.
    pub(crate) fn new(program: &ProgramArgs) -> ConsoleResult<Self> {
        // build defaults from program settings
        let options = CommandOptions {
            inputs: Vec::new(),
            config_inputs: false,
            cwd: Some(program.effective_cwd()?),
            manifest: program.manifest_path()?,
            target: None,
            target_overrides: None,
            profile: None,
            env: Vec::new(),
            overrides: overrides_from_program(program),
            watch: false,
            dry_run: false,
            trace: program.timings.then_some(TraceView::Detailed),
        };

        Ok(Self { options })
    }

    /// Set input sources for the command.
    pub(crate) fn inputs(mut self, inputs: Vec<CommandInput>) -> Self {
        self.options.inputs = inputs;
        self
    }

    /// Use package.json sources when explicit inputs are empty.
    pub(crate) fn config_inputs(mut self, allow: bool) -> Self {
        self.options.config_inputs = allow;
        self
    }

    /// Set the target name override.
    pub(crate) fn target(mut self, target: Option<String>) -> Self {
        self.options.target = target;
        self
    }

    /// Set target overrides.
    pub(crate) fn target_overrides(mut self, overrides: Option<CommandTargetOverrides>) -> Self {
        self.options.target_overrides = overrides;
        self
    }

    /// Enable dry-run mode.
    pub(crate) fn dry_run(mut self, dry_run: bool) -> Self {
        self.options.dry_run = dry_run;
        self
    }

    /// Build the common command options.
    pub(crate) fn build(self) -> CommandOptions {
        self.options
    }
}

/// Build manifest overrides from explicit CLI formatter and linter options.
pub(crate) fn overrides_from_program(program: &ProgramArgs) -> Vec<ManifestOverride> {
    let mut overrides = Vec::new();

    // formatter
    if let Some(value) = formatter_override_value(&program.formatter) {
        overrides.push(ManifestOverride {
            path: "formatter".to_string(),
            value: serde::Value::from(value),
        });
    }

    // linter
    if let Some(value) = linter_override_value(&program.linter) {
        overrides.push(ManifestOverride {
            path: "linter".to_string(),
            value: serde::Value::from(value),
        });
    }

    overrides
}

/// Build one formatter override object from explicit CLI flags.
fn formatter_override_value(args: &FormatterOptionsArgs) -> Option<json::Value> {
    let mut object = json::Map::new();

    // layout
    if let Some(indent_style) = args.indent_style {
        object.insert(
            "indentStyle".to_string(),
            json::json!(indent_style_override_value(indent_style)),
        );
    }
    if let Some(indent_width) = args.indent_width {
        object.insert("indentWidth".to_string(), json::json!(indent_width));
    }
    if let Some(line_ending) = args.line_ending {
        object.insert(
            "lineEnding".to_string(),
            json::json!(line_ending_override_value(line_ending)),
        );
    }
    if let Some(line_width) = args.line_width {
        object.insert("lineWidth".to_string(), json::json!(line_width));
    }

    if object.is_empty() {
        return None;
    }

    Some(json::Value::Object(object))
}

/// Build one linter override object from explicit CLI flags.
fn linter_override_value(args: &LinterOptionsArgs) -> Option<json::Value> {
    let mut object = json::Map::new();
    let mut rules = json::Map::new();

    // select and enable the requested rules
    if !args.only.is_empty() {
        object.insert("only".to_string(), json::json!(args.only));

        for rule in &args.only {
            rules.insert(rule.clone(), json::json!("warning"));
        }
    }

    // apply explicit levels
    for rule in &args.allow {
        rules.insert(rule.clone(), json::json!("off"));
    }
    for rule in &args.warn {
        rules.insert(rule.clone(), json::json!("warning"));
    }
    for rule in &args.deny {
        rules.insert(rule.clone(), json::json!("error"));
    }

    if !rules.is_empty() {
        object.insert("rules".to_string(), json::Value::Object(rules));
    }

    if object.is_empty() {
        return None;
    }

    Some(json::Value::Object(object))
}

/// Convert one indent style argument to one config value.
fn indent_style_override_value(value: IndentStyleArg) -> &'static str {
    match value {
        IndentStyleArg::Tab => "tab",
        IndentStyleArg::Space => "space",
    }
}

/// Convert one line ending argument to one config value.
fn line_ending_override_value(value: LineEndingArg) -> &'static str {
    match value {
        LineEndingArg::Lf => "lf",
        LineEndingArg::Crlf => "crlf",
        LineEndingArg::Cr => "cr",
    }
}
