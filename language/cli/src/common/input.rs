use std::io::Read;
use std::path::{Path, PathBuf};

use clap::Args;
use destack_source::FileType;
use destack_workspace::CommandInput;

use crate::diagnostic::{ConsoleError, ConsoleResult};

/// Input source for compilation commands.
#[derive(Debug, Clone)]
pub enum InputSource {
    /// Physical file from disk.
    File(PathBuf),
    /// Inline code via --eval or --module.
    Inline { code: String, name: String },
    /// Read from stdin.
    Stdin { name: String },
}

/// Single input arguments for debug commands (file OR eval, not multiple).
#[derive(Args, Debug, Clone, Default)]
pub struct SingleInputArgs {
    /// Input file.
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Evaluate inline code.
    #[arg(short = 'e', long = "eval")]
    pub eval: Option<String>,

    /// File format (ds|ts|tsx|js|jsx, default: ds).
    #[arg(id = "file_type", long = "type", value_name = "TYPE")]
    pub file_type: Option<String>,
}

impl SingleInputArgs {
    /// Convert to InputSource, returning an error message if no input provided.
    pub fn to_source(&self) -> Result<InputSource, &'static str> {
        if let Some(ref path) = self.file {
            Ok(InputSource::File(path.clone()))
        } else if let Some(ref code) = self.eval {
            let extension = self.file_type.as_deref().unwrap_or("ds");
            Ok(InputSource::Inline {
                code: code.clone(),
                name: format!("<eval>.{extension}"),
            })
        } else {
            Err("no input provided (use FILE or --eval)")
        }
    }

    /// Get the file type from format argument.
    pub fn file_type(&self) -> FileType {
        let format_name = self.file_type.as_deref().unwrap_or("ds");
        FileType::from_extension_or_unknown(format_name)
    }
}

/// Common input arguments for compilation commands.
#[derive(Args, Debug, Clone, Default)]
pub struct InputArgs {
    /// Input files or directories.
    #[arg(value_name = "FILES")]
    pub files: Vec<PathBuf>,

    /// Evaluate inline code (can be specified multiple times).
    #[arg(short = 'e', long = "eval")]
    pub eval: Vec<String>,

    /// Named module as name:code (can be specified multiple times).
    /// Example: --module 'foo:export const x = 1' creates foo.ds
    /// Include extension to override: --module 'bar.ts:const x: number = 1'
    #[arg(short = 'm', long = "module")]
    pub module: Vec<String>,

    /// Read from stdin.
    #[arg(long)]
    pub stdin: bool,

    /// File format for --eval/--stdin (ds|ts|tsx|js|jsx, default: ds).
    #[arg(id = "file_type", long = "type", value_name = "TYPE")]
    pub file_type: Option<String>,
}

impl InputArgs {
    /// Check if any input was provided.
    pub fn has_input(&self) -> bool {
        !self.files.is_empty() || !self.eval.is_empty() || !self.module.is_empty() || self.stdin
    }

    /// Split one directory positional out as the workspace root.
    ///
    /// A directory argument means "run against this package or workspace",
    /// so it selects the root while remaining paths stay module files.
    pub fn take_directory_root(&mut self) -> ConsoleResult<Option<PathBuf>> {
        let (directories, files): (Vec<_>, Vec<_>) =
            self.files.drain(..).partition(|path| path.is_dir());
        self.files = files;

        match directories.as_slice() {
            [] => Ok(None),
            [directory] => Ok(Some(directory.clone())),
            _ => Err(ConsoleError::message(format!(
                "expected at most one directory argument, got {}",
                directories.len()
            ))),
        }
    }

    /// Convert arguments to input sources.
    pub fn to_sources(&self) -> ConsoleResult<Vec<InputSource>> {
        let mut sources = Vec::new();
        let default_extension = self.file_type.as_deref().unwrap_or("ds");

        // files first
        for path in &self.files {
            sources.push(InputSource::File(path.clone()));
        }

        // then --eval (anonymous, simple)
        for (index, code) in self.eval.iter().enumerate() {
            let name = format!("<eval{index}>.{default_extension}");
            sources.push(InputSource::Inline {
                code: code.clone(),
                name,
            });
        }

        // then --module (named, for cross-imports)
        for module_str in &self.module {
            let (name, code) = parse_module_arg(module_str, default_extension)?;
            sources.push(InputSource::Inline { code, name });
        }

        // then --stdin
        if self.stdin {
            let name = format!("<stdin>.{default_extension}");
            sources.push(InputSource::Stdin { name });
        }

        if sources.is_empty() {
            return Err(ConsoleError::message("no input provided"));
        }

        Ok(sources)
    }

    /// Resolve explicitly provided input sources.
    pub fn explicit_sources(&self) -> ConsoleResult<Vec<InputSource>> {
        if self.has_input() {
            self.to_sources()
        } else {
            Ok(Vec::new())
        }
    }

    /// Get the file type from format argument.
    pub fn file_type(&self) -> FileType {
        let format_name = self.file_type.as_deref().unwrap_or("ds");
        FileType::from_extension_or_unknown(format_name)
    }
}

/// Parse a --module argument in `name:code` format.
///
/// Appends the default extension when none is provided.
/// Returns an error when the format is invalid.
///
/// Examples:
/// - `foo:export const x = 1` becomes `("foo.ds", "export const x = 1")`
/// - `bar.ds:const x: number = 1` becomes `("bar.ds", "const x: number = 1")`
fn parse_module_arg(arg: &str, default_extension: &str) -> ConsoleResult<(String, String)> {
    let colon_pos = arg.find(':').ok_or_else(|| {
        ConsoleError::message(format!(
            "invalid --module format: expected 'name:code', got '{arg}'"
        ))
    })?;

    let name_part = &arg[..colon_pos];
    let code = arg[colon_pos + 1..].to_string();

    if name_part.is_empty() {
        return Err(ConsoleError::message(
            "invalid --module format: name cannot be empty",
        ));
    }

    // validate the module name
    let valid_name = name_part
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.');
    if !valid_name {
        return Err(ConsoleError::message(format!(
            "invalid --module name: '{name_part}' contains invalid characters"
        )));
    }

    // append the default extension when absent
    let name = if name_part.contains('.') {
        name_part.to_string()
    } else {
        format!("{name_part}.{default_extension}")
    };

    Ok((name, code))
}

/// Convert input sources into workspace command inputs.
pub(crate) fn command_inputs_from_sources(
    sources: &[InputSource],
    default_file_type: FileType,
) -> ConsoleResult<Vec<CommandInput>> {
    let mut inputs = Vec::new();
    for source in sources {
        match source {
            InputSource::File(path) => inputs.push(CommandInput::File { path: path.clone() }),
            InputSource::Inline { code, name } => {
                let file_type = FileType::from_path(Path::new(name)).unwrap_or(default_file_type);
                inputs.push(CommandInput::Inline {
                    name: name.clone(),
                    content: code.clone(),
                    file_type,
                });
            }
            InputSource::Stdin { name } => {
                let mut content = String::new();
                std::io::stdin()
                    .read_to_string(&mut content)
                    .map_err(|error| {
                        ConsoleError::message(format!("failed to read stdin: {error}"))
                    })?;
                let file_type = FileType::from_path(Path::new(name)).unwrap_or(default_file_type);
                inputs.push(CommandInput::Stdin {
                    name: name.clone(),
                    content,
                    file_type,
                });
            }
        }
    }

    Ok(inputs)
}
