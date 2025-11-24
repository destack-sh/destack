use crate::{BindTask, CompileTask, Compiler, ImportError, ImportResult};

use dyst_dir::{Module, PackageId, Program};
use dyst_parser::Parser;
use dyst_source::{DiagnosticCollector, File, FileId, FileType, StringId, Uri};

/// Task to import a file into the compiler.
#[derive(Debug, Clone)]
pub enum ImportTask {
    /// Import module from a (preloaded) file. (Mostly for internal use.)
    ImportModuleFromFile { file: FileId },
    /// Import module from a Uri.
    ImportModuleFromUri { uri: Uri, ty: Option<FileType> },
    /// Import module from a specifier.
    ImportModuleFromSpecifier {
        directory: Option<StringId>,
        target: StringId,
    },
}

impl ImportTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            ImportTask::ImportModuleFromFile { .. } => 1,
            ImportTask::ImportModuleFromUri { .. } => 2,
            ImportTask::ImportModuleFromSpecifier { .. } => 3,
        }
    }

    /// Get a message for the task.
    pub fn message<'a>(&self, program: &'a Program<'a>) -> String {
        match self {
            ImportTask::ImportModuleFromFile { file: file_id } => {
                format!("import file:{file_id:?}")
            }
            ImportTask::ImportModuleFromUri { uri, .. } => {
                format!("import '{uri}'")
            }
            ImportTask::ImportModuleFromSpecifier { directory, target } => {
                let target_str = program.strings.get(*target).to_string();
                if let Some(directory) = directory {
                    let directory_str = program.strings.get(*directory).to_string();
                    format!("import '{target_str}' from '{directory_str}'")
                } else {
                    format!("import '{target_str}'")
                }
            }
        }
    }
}

impl From<ImportTask> for CompileTask {
    fn from(task: ImportTask) -> Self {
        CompileTask::Import(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process an import task for a module.
    pub fn process_import(&self, task: ImportTask) -> ImportResult<()> {
        let file: &File = match task {
            ImportTask::ImportModuleFromFile { file: file_id } => {
                match self.program.files.get(file_id) {
                    Some(file) => file,
                    None => return Err(ImportError::FileIdNotFound { file_id }),
                }
            }
            ImportTask::ImportModuleFromUri { uri, ty } => {
                let path = uri
                    .to_path()
                    .ok_or_else(|| ImportError::InvalidUri { uri: uri.clone() })?;
                let content = self
                    .program
                    .fs
                    .read_to_string(path)
                    .map_err(|_| ImportError::FileUriNotFound { uri: uri.clone() })?;
                let ty = ty.unwrap_or_else(|| {
                    path.extension()
                        .map(|ext| {
                            FileType::from_extension_or_unknown(ext.to_string_lossy().as_ref())
                        })
                        .unwrap_or(FileType::Unknown)
                });
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let file_id = self.program.files.next_id();
                let file = File::from_text(file_id, name, uri, ty, content);
                self.program.files.insert(file);
                self.program.files.get(file_id).unwrap()
            }
            ImportTask::ImportModuleFromSpecifier { directory, target } => {
                return Err(ImportError::ModuleNotFound {
                    target,
                    directory,
                    error: None,
                });
            }
        };
        let package_id: Option<PackageId> = None; // nocheckin: resolve package/.. for module

        // parse AST from file
        let mut diagnostics = DiagnosticCollector::new();
        let mut parser = Parser::lex_file(file, self.program.language, &mut diagnostics);
        let expressions = parser.parse();
        self.program.diagnostics.merge_from(parser.diagnostics);

        // insert module
        let module_id = self.program.modules.next_id();
        let module = Module::new(
            module_id,
            file.id,
            file.uri.clone(),
            package_id,
            parser.tree,
            expressions,
            parser.strings,
        );
        self.program.modules.insert(module);

        // next task: bind module
        self.enqueue(BindTask::BindModule { module: module_id }.into());

        Ok(())
    }
}
