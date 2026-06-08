use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use destack_repository::Repository;
use destack_source::{File, FileId, FileType, Uri};

use crate::error::{CliError, CliResult};

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

    /// Convert arguments to input sources.
    pub fn to_sources(&self) -> CliResult<Vec<InputSource>> {
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
            return Err(CliError::message("no input provided"));
        }

        Ok(sources)
    }

    /// Get the file type from format argument.
    pub fn file_type(&self) -> FileType {
        let format_name = self.file_type.as_deref().unwrap_or("ds");
        FileType::from_extension_or_unknown(format_name)
    }
}

/// Load input sources into transient file views.
pub fn load_sources(repository: &Repository, sources: &[InputSource]) -> CliResult<Vec<Arc<File>>> {
    let mut files = Vec::new();

    for source in sources {
        let file = load_source(repository, source)?;
        files.push(file);
    }

    Ok(files)
}

/// Load a single input source into one transient file view.
pub fn load_source(repository: &Repository, source: &InputSource) -> CliResult<Arc<File>> {
    match source {
        InputSource::File(path) => load_file(repository, path),
        InputSource::Inline { code, name } => load_string(repository, code, name),
        InputSource::Stdin { name } => load_stdin(repository, name),
    }
}

/// Load a file from disk.
fn load_file(repository: &Repository, path: &PathBuf) -> CliResult<Arc<File>> {
    let path_str = path.display().to_string();

    // determine file type from extension
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("ds");
    let file_type = FileType::from_extension_or_unknown(ext);

    // read file content
    let content = repository
        .file_system()
        .read_to_string(path)
        .map_err(|e| CliError::message(format!("\"{path_str}\": {e}")))?;

    // create file
    let file_id = repository.file_id(path);
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<file>".to_string());
    let uri = Uri::from_path(path);

    let file = File::from_text(file_id, name, uri, Some(path.clone()), file_type, content);
    Ok(Arc::new(file))
}

/// Load inline code as a virtual file.
fn load_string(_repository: &Repository, code: &str, name: &str) -> CliResult<Arc<File>> {
    let ext = name.rsplit('.').next().unwrap_or("ds");
    let file_type = FileType::from_extension_or_unknown(ext);

    let file_id = FileId::from_logical_str(name);
    let uri = Uri::from_string(name);

    let file = File::from_text(
        file_id,
        name.to_string(),
        uri,
        None,
        file_type,
        code.to_string(),
    );
    Ok(Arc::new(file))
}

/// Parse a --module argument in `name:code` format.
///
/// - Auto-appends default extension if none provided
/// - Returns error if format is invalid
///
/// Examples:
/// - `foo:export const x = 1` → ("foo.ds", "export const x = 1")
/// - `bar.ts:const x: number = 1` → ("bar.ts", "const x: number = 1")
fn parse_module_arg(arg: &str, default_extension: &str) -> CliResult<(String, String)> {
    let colon_pos = arg.find(':').ok_or_else(|| {
        CliError::message(format!(
            "invalid --module format: expected 'name:code', got '{arg}'"
        ))
    })?;

    let name_part = &arg[..colon_pos];
    let code = arg[colon_pos + 1..].to_string();

    if name_part.is_empty() {
        return Err(CliError::message(
            "invalid --module format: name cannot be empty",
        ));
    }

    // Validate name is identifier-like
    let valid_name = name_part
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.');
    if !valid_name {
        return Err(CliError::message(format!(
            "invalid --module name: '{name_part}' contains invalid characters"
        )));
    }

    // Auto-append extension if not present
    let name = if name_part.contains('.') {
        name_part.to_string()
    } else {
        format!("{name_part}.{default_extension}")
    };

    Ok((name, code))
}

/// Load from stdin as a virtual file.
fn load_stdin(repository: &Repository, name: &str) -> CliResult<Arc<File>> {
    let mut content = String::new();
    std::io::stdin()
        .read_to_string(&mut content)
        .map_err(|e| CliError::message(format!("failed to read stdin: {e}")))?;

    load_string(repository, &content, name)
}
