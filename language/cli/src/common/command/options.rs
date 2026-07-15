use destack_workspace::{
    CommandInput, CommandOptions, CommandTargetOverrides, JsonValue, ManifestOverride,
};
use serde_json::{Map, Value, json};

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
            manifest: program.manifest.clone(),
            target: None,
            target_overrides: None,
            profile: None,
            env: Vec::new(),
            overrides: overrides_from_program(program),
            watch: false,
            dry_run: false,
        };

        Ok(Self { options })
    }

    /// Set input sources for the command.
    pub(crate) fn inputs(mut self, inputs: Vec<CommandInput>) -> Self {
        self.options.inputs = inputs;
        self
    }

    /// Use destack.json sources when explicit inputs are empty.
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

    /// Add one optional manifest override.
    pub(crate) fn manifest_override(mut self, override_: Option<ManifestOverride>) -> Self {
        if let Some(override_) = override_ {
            self.options.overrides.push(override_);
        }

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
            value: JsonValue::from(value),
        });
    }

    // linter
    if let Some(value) = linter_override_value(&program.linter) {
        overrides.push(ManifestOverride {
            path: "linter".to_string(),
            value: JsonValue::from(value),
        });
    }

    overrides
}

/// Build one formatter override object from explicit CLI flags.
fn formatter_override_value(args: &FormatterOptionsArgs) -> Option<Value> {
    let mut object: Map<String, Value> = Map::new();

    // layout
    if let Some(indent_style) = args.indent_style {
        object.insert(
            "indentStyle".to_string(),
            json!(indent_style_override_value(indent_style)),
        );
    }
    if let Some(indent_width) = args.indent_width {
        object.insert("indentWidth".to_string(), json!(indent_width));
    }
    if let Some(line_ending) = args.line_ending {
        object.insert(
            "lineEnding".to_string(),
            json!(line_ending_override_value(line_ending)),
        );
    }
    if let Some(line_width) = args.line_width {
        object.insert("lineWidth".to_string(), json!(line_width));
    }

    if object.is_empty() {
        return None;
    }

    Some(Value::Object(object))
}

/// Build one linter override object from explicit CLI flags.
fn linter_override_value(args: &LinterOptionsArgs) -> Option<Value> {
    let mut object: Map<String, Value> = Map::new();
    let mut rules: Map<String, Value> = Map::new();

    // rule selection
    for rule in &args.allow {
        rules.insert(rule.clone(), json!("off"));
    }
    for rule in &args.warn {
        rules.insert(rule.clone(), json!("warning"));
    }
    for rule in &args.deny {
        rules.insert(rule.clone(), json!("error"));
    }

    if !rules.is_empty() {
        object.insert("rules".to_string(), Value::Object(rules));
    }

    if object.is_empty() {
        return None;
    }

    Some(Value::Object(object))
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
