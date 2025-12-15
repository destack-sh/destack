use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use destack_source::{File, FileType, Uri};
use destack_workspace::Program;

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

    /// Source format (ds|ts|tsx|js|jsx, default: ds).
    #[arg(long = "type", alias = "format", value_name = "FORMAT")]
    pub format: Option<String>,
}

impl SingleInputArgs {
    /// Convert to InputSource, returning an error message if no input provided.
    pub fn to_source(&self) -> Result<InputSource, &'static str> {
        if let Some(ref path) = self.file {
            Ok(InputSource::File(path.clone()))
        } else if let Some(ref code) = self.eval {
            let extension = self.format.as_deref().unwrap_or("ds");
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
        let format_name = self.format.as_deref().unwrap_or("ds");
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

    /// Named module as name=code (can be specified multiple times).
    /// Example: --module 'foo=export const x = 1' creates foo.ds
    /// Include extension to override: --module 'bar.ts=const x: number = 1'
    #[arg(short = 'm', long = "module")]
    pub module: Vec<String>,

    /// Read from stdin.
    #[arg(long)]
    pub stdin: bool,

    /// Source format for --eval/--stdin (ds|ts|tsx|js|jsx, default: ds).
    #[arg(long = "type", alias = "format", value_name = "FORMAT")]
    pub format: Option<String>,
}

impl InputArgs {
    /// Convert arguments to input sources.
    pub fn to_sources(&self) -> Result<Vec<InputSource>, String> {
        let mut sources = Vec::new();
        let default_extension = self.format.as_deref().unwrap_or("ds");

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
            return Err("no input provided".to_string());
        }

        Ok(sources)
    }

    /// Get the file type from format argument.
    pub fn file_type(&self) -> FileType {
        let format_name = self.format.as_deref().unwrap_or("ds");
        FileType::from_extension_or_unknown(format_name)
    }
}

/// Load input sources into the program's file registry.
pub fn load_sources(program: &Program, sources: &[InputSource]) -> Result<Vec<Arc<File>>, String> {
    let mut files = Vec::new();

    for source in sources {
        let file = load_source(program, source)?;
        files.push(file);
    }

    Ok(files)
}

/// Load a single input source into the program's file registry.
pub fn load_source(program: &Program, source: &InputSource) -> Result<Arc<File>, String> {
    match source {
        InputSource::File(path) => load_file(program, path),
        InputSource::Inline { code, name } => load_string(program, code, name),
        InputSource::Stdin { name } => load_stdin(program, name),
    }
}

/// Load a file from disk.
fn load_file(program: &Program, path: &PathBuf) -> Result<Arc<File>, String> {
    let path_str = path.display().to_string();

    // determine file type from extension
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("ds");
    let file_type = FileType::from_extension_or_unknown(ext);

    // read file content
    let content = program
        .fs
        .read_to_string(path)
        .map_err(|e| format!("\"{path_str}\": {e}"))?;

    // create file
    let file_id = program.files.next_id();
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<file>".to_string());
    let uri = Uri::from_path(path);

    let file = File::from_text(file_id, name, uri, Some(path.clone()), file_type, content);
    program.files.insert(file);
    Ok(program.files.get(file_id))
}

/// Load inline code as a virtual file.
fn load_string(program: &Program, code: &str, name: &str) -> Result<Arc<File>, String> {
    let ext = name.rsplit('.').next().unwrap_or("ds");
    let file_type = FileType::from_extension_or_unknown(ext);

    let file_id = program.files.next_id();
    let uri = Uri::from_string(name);

    let file = File::from_text(
        file_id,
        name.to_string(),
        uri,
        None,
        file_type,
        code.to_string(),
    );
    program.files.insert(file);
    Ok(program.files.get(file_id))
}

/// Parse a --module argument in `name=code` format.
///
/// - Auto-appends default extension if none provided
/// - Returns error if format is invalid
///
/// Examples:
/// - `foo=export const x = 1` → ("foo.ds", "export const x = 1")
/// - `bar.ts=const x: number = 1` → ("bar.ts", "const x: number = 1")
fn parse_module_arg(arg: &str, default_extension: &str) -> Result<(String, String), String> {
    let eq_pos = arg
        .find('=')
        .ok_or_else(|| format!("invalid --module format: expected 'name=code', got '{arg}'"))?;

    let name_part = &arg[..eq_pos];
    let code = arg[eq_pos + 1..].to_string();

    if name_part.is_empty() {
        return Err("invalid --module format: name cannot be empty".to_string());
    }

    // Validate name is identifier-like
    let valid_name = name_part
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.');
    if !valid_name {
        return Err(format!(
            "invalid --module name: '{name_part}' contains invalid characters"
        ));
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
fn load_stdin(program: &Program, name: &str) -> Result<Arc<File>, String> {
    let mut content = String::new();
    std::io::stdin()
        .read_to_string(&mut content)
        .map_err(|e| format!("failed to read stdin: {e}"))?;

    load_string(program, &content, name)
}
