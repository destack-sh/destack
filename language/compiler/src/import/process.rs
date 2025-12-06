use std::path::PathBuf;
use std::sync::Arc;

use crate::{Compiler, ImportError, ImportResult, ResolveTask, Task, TaskDebug, TaskOutput};

use destack_dir::DependencySource;
use destack_parser::Parser;
use destack_resolver::Resolver;
use destack_source::{File, FileId, FileType, StringId, Uri};

use destack_source::ModuleId;
use destack_workspace::{Module, ModuleAst, Program};

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
        module: Option<ModuleId>,
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
}

impl TaskDebug for ImportTask {
    fn name(&self) -> &'static str {
        match self {
            ImportTask::ImportModuleFromFile { .. } => "file",
            ImportTask::ImportModuleFromUri { .. } => "uri",
            ImportTask::ImportModuleFromPath { .. } => "path",
            ImportTask::ImportModuleFromSpecifier { .. } => "specifier",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            ImportTask::ImportModuleFromFile { file } => {
                let uri = &program.files.get(*file).uri.to_string();
                format!(r#"file="{uri}""#)
            }
            ImportTask::ImportModuleFromUri { uri, .. } => format!(r#"uri="{uri}""#),
            ImportTask::ImportModuleFromPath { path, .. } => {
                format!(r#"path="{}""#, path.display())
            }
            ImportTask::ImportModuleFromSpecifier { target, module, .. } => {
                let specifier = program.strings.get(*target).to_string();
                if let Some(module) = module {
                    let module = program.modules.get(*module);
                    let module_uri = module.read().uri.clone().to_string();
                    format!(r#"specifier="{specifier}" module="{module_uri}""#)
                } else {
                    format!(r#"specifier="{specifier}""#)
                }
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
            .with_extensions(vec![
                ".ds".into(),
                ".tsx".into(),
                ".ts".into(),
                ".jsx".into(),
                ".js".into(),
                ".mjs".into(),
                ".cjs".into(),
                ".json".into(),
                ".node".into(),
            ])
            .with_conditions(vec!["types".to_string(), "import".to_string()]);
        let resolver: Resolver = Resolver::new(self.program.clone(), resolver_options);

        // find and read file
        let file: Arc<File> = match &task {
            ImportTask::ImportModuleFromFile { file: file_id } => self.program.files.get(*file_id),
            ImportTask::ImportModuleFromPath { path, ty } => {
                self.import_file_from_path(path.clone(), *ty, &resolver)?
            }
            ImportTask::ImportModuleFromUri { uri, ty } => {
                self.import_file_from_uri(uri.clone(), *ty, &resolver)?
            }
            ImportTask::ImportModuleFromSpecifier {
                target,
                module: module_id,
                ty,
                source,
            } => self.import_file_from_specifier(*module_id, *target, *ty, *source, &resolver)?,
        };

        // lock module creation to prevent race conditions when imports resolve to the same file
        let import_lock = self.get_import_lock(&file.uri);
        let mut import_guard = import_lock.lock();

        // check if another task completed while we were waiting for the lock
        if let Some(module_id) = *import_guard {
            drop(import_guard);
            self.resolve_import_specifier(&task, module_id);
            return Ok(ImportOutput { module: module_id });
        }
        // also check registry (belt and suspenders)
        else if let Some(module_id) = self.program.modules.get_id_by_uri(&file.uri) {
            *import_guard = Some(module_id);
            drop(import_guard);
            self.resolve_import_specifier(&task, module_id);
            return Ok(ImportOutput { module: module_id });
        }

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

        // create and insert module (must insert before bind so diagnostics can reference it)
        let module_id = self.program.modules.next_id();
        let module_ast = ModuleAst::from_tree(module_id, parser.tree, expressions, parser.strings);
        let module = Module::from_ast(
            module_id,
            file.id,
            file.uri.clone(),
            file.path.clone(),
            package_id,
            module_ast,
        );
        self.program.modules.insert(module);
        tracing::trace!(?module_id, ?file.uri, "import.module.resolve");

        // immediately bind module
        // (bind is conceptually a stage, but we bind immediately during import to avoid race conditions)
        {
            let module_arc = self.program.modules.get(module_id);
            let mut module_guard = module_arc.write();
            self.bind_module(&mut module_guard);
        }

        // mark import as complete so waiting tasks can proceed
        *import_guard = Some(module_id);
        drop(import_guard);

        // resolve import specifier in symbol table
        self.resolve_import_specifier(&task, module_id);

        // next task: resolve module
        self.enqueue(ResolveTask::ResolveModule { module: module_id });

        Ok(ImportOutput { module: module_id })
    }

    /// Resolve import specifier in symbol table (if this is a specifier import).
    fn resolve_import_specifier(&self, task: &ImportTask, module_id: ModuleId) {
        if let ImportTask::ImportModuleFromSpecifier {
            target,
            module: source_module_id,
            ..
        } = task
        {
            // resolve relative import
            if let &Some(source_module_id) = source_module_id {
                let source_module = self.program.modules.get(source_module_id);
                let source_module = source_module.read();
                let mut symbols = source_module.dir.symbols.write();
                symbols.resolve_import(Some(source_module_id), *target, module_id);
            }
            // resolve global import
            else {
                let global_module = self.program.modules.get(self.program.root_module_id);
                let global_module = global_module.read();
                let mut symbols = global_module.dir.symbols.write();
                symbols.resolve_import(None, *target, module_id);
            }
        }
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
                    target: self.program.strings.intern(path.to_string_lossy()),
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
        module_id: Option<ModuleId>,
        target: StringId,
        ty: Option<FileType>,
        _source: DependencySource,
        resolver: &Resolver,
    ) -> ImportResult<Arc<File>> {
        // prepare context
        let directory = {
            if let Some(module_id) = module_id {
                let module = self.program.modules.get(module_id);
                let module_file = self.program.files.get(module.read().file_id);
                module_file
                    .uri
                    .to_path_buf()
                    .and_then(|path| path.parent().map(|p| p.to_path_buf()))
                    .unwrap_or_else(|| self.program.cwd.clone())
            } else {
                self.program.cwd.clone()
            }
        };

        // resolve
        let specifier = self.program.strings.get(target).to_string();
        let resolution = resolver.resolve(&directory, &specifier).map_err(|error| {
            ImportError::ModuleNotFound {
                node: self.program.root_node_id,
                target,
                error: Some(error),
            }
        })?;

        // import file
        self.import_file_from_path(resolution.path.clone(), ty, resolver)
    }
}
