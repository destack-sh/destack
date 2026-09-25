use std::io::Read;
use std::path::{Path, PathBuf};

use clap::Args;
use tspp_source::LanguageType;
use tspp_workspace::CommandInput;

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

/// Common input arguments for compilation commands.
#[derive(Args, Debug, Clone, Default)]
pub struct InputArgs {
    /// Input files and at most one package or Workspace directory.
    #[arg(value_name = "PATHS")]
    pub files: Vec<PathBuf>,

    /// Evaluate inline code (can be specified multiple times).
    #[arg(short = 'e', long = "eval")]
    pub eval: Vec<String>,

    /// Named module as name:code (can be specified multiple times).
    /// Example: --module 'foo:export const x = 1' creates foo.tspp
    /// Include the declaration extension when needed: --module 'bar.d.tspp:export type X = int32'
    #[arg(short = 'm', long = "module")]
    pub module: Vec<String>,

    /// Read from stdin.
    #[arg(long)]
    pub stdin: bool,

    /// File format for `--eval` and `--stdin` (`tspp` or `d.tspp`, default: `tspp`).
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
        // collect sources in stable command order
        let mut sources = Vec::new();
        let default_extension = self.file_type.as_deref().unwrap_or("tspp");

        // add physical files first
        for path in &self.files {
            sources.push(InputSource::File(path.clone()));
        }

        // add anonymous evaluations
        for (index, code) in self.eval.iter().enumerate() {
            let name = format!("<eval{index}>.{default_extension}");
            sources.push(InputSource::Inline {
                code: code.clone(),
                name,
            });
        }

        // add named modules
        for module_argument in &self.module {
            sources.push(InputSource::from_module_argument(
                module_argument,
                default_extension,
            )?);
        }

        // add standard input last
        if self.stdin {
            let name = format!("<stdin>.{default_extension}");
            sources.push(InputSource::Stdin { name });
        }

        // require at least one source
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

    /// Return the source language selected by the format argument.
    fn language_type(&self) -> ConsoleResult<LanguageType> {
        let format_name = self.file_type.as_deref().unwrap_or("tspp");
        let language = LanguageType::from_extension(format_name).ok_or_else(|| {
            ConsoleError::message(format!("unsupported input file type '{format_name}'"))
        })?;

        Ok(language)
    }

    /// Convert resolved input sources into workspace command inputs.
    pub(crate) fn command_inputs(
        &self,
        sources: &[InputSource],
    ) -> ConsoleResult<Vec<CommandInput>> {
        let default_language = self.language_type()?;

        sources
            .iter()
            .map(|source| source.to_command_input(default_language))
            .collect()
    }
}

impl InputSource {
    /// Parse one `--module` argument into an inline source.
    fn from_module_argument(argument: &str, default_extension: &str) -> ConsoleResult<Self> {
        // split the module name and code
        let colon_position = argument.find(':').ok_or_else(|| {
            ConsoleError::message(format!(
                "invalid --module format: expected 'name:code', got '{argument}'"
            ))
        })?;
        let name = &argument[..colon_position];
        let code = argument[colon_position + 1..].to_string();

        // require a module name
        if name.is_empty() {
            return Err(ConsoleError::message(
                "invalid --module format: name cannot be empty",
            ));
        }

        // require an identifier shaped module name
        let is_valid_name = name
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, '_' | '-' | '.'));
        if !is_valid_name {
            return Err(ConsoleError::message(format!(
                "invalid --module name: '{name}' contains invalid characters"
            )));
        }

        // append the selected extension when absent
        let name = if name.contains('.') {
            name.to_string()
        } else {
            format!("{name}.{default_extension}")
        };

        Ok(Self::Inline { code, name })
    }

    /// Convert this input source into one workspace command input.
    fn to_command_input(&self, default_language: LanguageType) -> ConsoleResult<CommandInput> {
        match self {
            // preserve physical file inputs
            Self::File(path) => Ok(CommandInput::File { path: path.clone() }),

            // resolve inline source language from its explicit module name
            Self::Inline { code, name } => {
                let path = Path::new(name);
                let language = if path.extension().is_none() {
                    default_language
                } else {
                    LanguageType::from_path(path).ok_or_else(|| {
                        ConsoleError::message(format!(
                            "unsupported source module file type for '{name}'"
                        ))
                    })?
                };

                Ok(CommandInput::Inline {
                    name: name.clone(),
                    content: code.clone(),
                    file_type: language.into(),
                })
            }

            // read and resolve standard input
            Self::Stdin { name } => {
                // read standard input once
                let mut content = String::new();
                std::io::stdin()
                    .read_to_string(&mut content)
                    .map_err(|error| {
                        ConsoleError::message(format!("failed to read stdin: {error}"))
                    })?;

                // require the synthetic source name to identify a TS++ language
                let path = Path::new(name);
                let language = LanguageType::from_path(path).ok_or_else(|| {
                    ConsoleError::message(format!(
                        "unsupported source module file type for '{name}'"
                    ))
                })?;

                Ok(CommandInput::Stdin {
                    name: name.clone(),
                    content,
                    file_type: language.into(),
                })
            }
        }
    }
}
