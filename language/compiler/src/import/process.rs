use std::path::PathBuf;
use std::sync::Arc;

use crate::{BindTask, Compiler, ImportError, ImportResult, Task, TaskOutput};

use dyst_dir::{DependencySource, Module, ModuleId, Program};
use dyst_parser::Parser;
use dyst_resolver::Resolver;
use dyst_source::{File, FileId, FileType, StringId, Uri};

/// Task to import a file into the compiler.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ImportTask {
    /// Import module from a (preloaded) file. (Mostly for internal use.)
    ImportModuleFromFile { file: FileId },
    /// Import module from a Uri.
    ImportModuleFromUri { uri: Uri, ty: Option<FileType> },
    /// Import module from a path.
    ImportModuleFromPath { path: PathBuf, ty: Option<FileType> },
    /// Import module from a specifier.
    ImportModuleFromSpecifier {
        target: StringId,
        ty: Option<FileType>,
        module: ModuleId,
        source: DependencySource,
    },
}

impl ImportTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            ImportTask::ImportModuleFromFile { .. } => 1,
            ImportTask::ImportModuleFromUri { .. } => 2,
            ImportTask::ImportModuleFromPath { .. } => 3,
            ImportTask::ImportModuleFromSpecifier { .. } => 4,
        }
    }

    /// Get a message for the task.
    pub fn message(&self, program: &Program) -> String {
        match self {
            ImportTask::ImportModuleFromFile { file: file_id } => {
                format!("import file:{file_id:?}")
            }
            ImportTask::ImportModuleFromUri { uri, .. } => {
                format!("import '{uri}'")
            }
            ImportTask::ImportModuleFromPath { path, .. } => {
                format!("import '{path:?}'")
            }
            ImportTask::ImportModuleFromSpecifier { target, .. } => {
                let target_str = program.strings.get(*target).to_string();
                format!("import '{target_str}'")
            }
        }
    }
}

impl From<ImportTask> for Task {
    fn from(task: ImportTask) -> Self {
        Task::Import(task)
    }
}

/// Output of an import task.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportOutput {
    pub module: ModuleId,
}

impl From<ImportOutput> for TaskOutput {
    fn from(output: ImportOutput) -> Self {
        TaskOutput::Import(output)
    }
}

impl Compiler {
    /// Process an import task for a module.
    pub fn process_import(&self, task: ImportTask) -> ImportResult<ImportOutput> {
        let resolver_options = self
            .options
            .import
            .resolve
            .clone()
            .with_conditions(vec!["types".to_string(), "import".to_string()]);
        let resolver: Resolver = Resolver::new(self.program.clone(), resolver_options);

        // read file
        let file: Arc<File> = match task {
            ImportTask::ImportModuleFromFile { file: file_id } => self.program.files.get(file_id),
            ImportTask::ImportModuleFromPath { path, ty } => {
                self.import_file_from_path(path, ty, &resolver)?
            }
            ImportTask::ImportModuleFromUri { uri, ty } => {
                self.import_file_from_uri(uri, ty, &resolver)?
            }
            ImportTask::ImportModuleFromSpecifier {
                target,
                module: module_id,
                ty,
                source,
            } => self.import_file_from_specifier(module_id, target, ty, source, &resolver)?,
        };

        // find package
        let package_id = {
            if let Some(path) = file.uri.to_path_buf() {
                resolver.find_package(&path)
            } else {
                None
            }
        };

        // parse AST from file
        let mut parser = Parser::lex_file(file.clone(), self.program.language);
        let expressions = parser.parse();
        self.program.diagnostics.merge_from(&parser.diagnostics);

        // insert module
        let module_id = self.program.modules.next_id();
        let module = Module::new(
            module_id,
            file.id,
            file.uri.clone(),
            file.path.clone(),
            package_id,
            parser.tree,
            expressions,
            parser.strings,
        );
        self.program.modules.insert(module);

        // next task: bind module
        self.enqueue(BindTask::BindModule { module: module_id });

        Ok(ImportOutput { module: module_id })
    }

    /// Import a file from a URI.
    pub(super) fn import_file_from_uri(
        &self,
        uri: Uri,
        ty: Option<FileType>,
        resolver: &Resolver,
    ) -> ImportResult<Arc<File>> {
        let path = uri.to_path().ok_or_else(|| ImportError::ModuleNotFound {
            node: self.program.root_node_id,
            target: self.program.strings.intern(&uri),
            error: None,
        })?;
        self.import_file_from_path(path.to_path_buf(), ty, resolver)
    }

    /// Import a file from a path.
    pub(super) fn import_file_from_path(
        &self,
        path: PathBuf,
        ty: Option<FileType>,
        _resolver: &Resolver,
    ) -> ImportResult<Arc<File>> {
        // metadata
        let extension = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let ty = ty.unwrap_or_else(|| FileType::from_extension_or_unknown(extension.as_ref()));
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        // read file
        let content =
            self.program
                .fs
                .read_to_string(&path)
                .map_err(|_| ImportError::ModuleNotFound {
                    node: self.program.root_node_id,
                    target: self.program.strings.intern(&path.to_string_lossy()),
                    error: None,
                })?;

        // make file
        let file_id = self.program.files.next_id();
        let uri = Uri::from_path(&path);
        let file = File::from_text(file_id, name, uri, Some(path), ty, content);
        self.program.files.insert(file);
        let file = self.program.files.get(file_id);
        Ok(file)
    }

    /// Import a file from a specifier.
    pub(super) fn import_file_from_specifier(
        &self,
        module_id: ModuleId,
        target: StringId,
        ty: Option<FileType>,
        _source: DependencySource,
        resolver: &Resolver,
    ) -> ImportResult<Arc<File>> {
        // prepare context
        let module = self.program.modules.get(module_id);
        let module_file = self.program.files.get(module.read().file_id);
        let module_directory = module_file
            .uri
            .to_path_buf()
            .unwrap_or_else(|| self.program.cwd.clone());
        let specifier = self.program.strings.get(target).to_string();

        // resolve
        let resolution = resolver
            .resolve(&module_directory, &specifier)
            .map_err(|error| ImportError::ModuleNotFound {
                node: self.program.root_node_id,
                target,
                error: Some(error),
            })?;

        // import file
        self.import_file_from_path(resolution.path.clone(), ty, resolver)
    }
}
