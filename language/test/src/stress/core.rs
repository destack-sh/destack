use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_query::RepositoryQueryIndexExt;
use destack_source::{Edit, FileId, Span};
use destack_workspace::{Ref, Repository, Revision};
use serde::Deserialize;

use crate::core::{
    SharedMemoryWorkspace, default_profile_id_for_module, module_id_for_path,
    provide_workspace_artifacts, write_workspace_text_file,
};

/// One stress recipe loaded from a checked in fixture.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct StressRecipe {
    /// The deterministic seed for generation.
    pub seed: u64,
    /// The number of generated library modules.
    pub library_modules: usize,
    /// The number of exported declaration groups per library module.
    pub declarations_per_module: usize,
    /// The number of workspace symbol queries to run.
    pub workspace_queries: usize,
    /// The number of modules that should include damaged syntax tails.
    #[serde(default)]
    pub malformed_modules: usize,
    /// Whether generated modules should include namespace imports.
    #[serde(default = "default_true")]
    pub include_namespace_imports: bool,
    /// Whether generated modules should include explicit type imports.
    #[serde(default = "default_true")]
    pub include_type_imports: bool,
}

/// One generated stress project.
#[derive(Debug, Clone)]
pub(super) struct StressProject {
    /// The generated files.
    pub files: Vec<StressFile>,
    /// The generated anchor points.
    pub anchors: Vec<StressAnchor>,
    /// The generated workspace symbol queries.
    pub workspace_queries: Vec<String>,
}

/// One generated stress file.
#[derive(Debug, Clone)]
pub(super) struct StressFile {
    /// The relative file path.
    pub path: String,
    /// The generated source text.
    pub text: String,
}

/// One generated anchor used by downstream stress drivers.
#[derive(Debug, Clone)]
pub(super) struct StressAnchor {
    /// The anchor name for debugging.
    pub name: String,
    /// The relative file path.
    pub path: String,
    /// The byte offset in the file.
    pub offset: u32,
    /// The anchor kind.
    pub kind: StressAnchorKind,
    /// The operation expectations for this anchor.
    pub expectations: StressAnchorExpectations,
}

/// One anchor kind shared across stress drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StressAnchorKind {
    /// A value symbol use site.
    ValueUse,
    /// A type symbol use site.
    TypeUse,
    /// An imported symbol name in an import clause.
    ImportName,
    /// A member completion prefix.
    MemberPrefix,
    /// A statement completion gap.
    StatementGap,
}

/// One expectation level for a single IDE operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StressOperationExpectation {
    /// Do not assert anything for this operation.
    Skip,
    /// The operation must not produce a result.
    Forbidden,
    /// The operation may or may not produce a result, but must stay stable.
    Optional,
    /// The operation must produce a non empty result.
    Required,
}

/// One explicit expectation set for a generated anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StressAnchorExpectations {
    /// The hover expectation.
    pub hover: StressOperationExpectation,
    /// The goto definition expectation.
    pub definition: StressOperationExpectation,
    /// The goto declaration expectation.
    pub declaration: StressOperationExpectation,
    /// The goto type definition expectation.
    pub type_definition: StressOperationExpectation,
    /// The references expectation.
    pub references: StressOperationExpectation,
    /// The completion expectation.
    pub completion: StressOperationExpectation,
}

impl StressAnchorExpectations {
    /// Return the expectation set for one value use.
    pub(super) fn value_use() -> Self {
        Self {
            hover: StressOperationExpectation::Required,
            definition: StressOperationExpectation::Required,
            declaration: StressOperationExpectation::Required,
            type_definition: StressOperationExpectation::Forbidden,
            references: StressOperationExpectation::Required,
            completion: StressOperationExpectation::Skip,
        }
    }

    /// Return the expectation set for one type use.
    pub(super) fn type_use() -> Self {
        Self {
            hover: StressOperationExpectation::Required,
            definition: StressOperationExpectation::Required,
            declaration: StressOperationExpectation::Required,
            type_definition: StressOperationExpectation::Required,
            references: StressOperationExpectation::Required,
            completion: StressOperationExpectation::Skip,
        }
    }

    /// Return the expectation set for one imported value name.
    pub(super) fn import_value_name() -> Self {
        Self {
            hover: StressOperationExpectation::Required,
            definition: StressOperationExpectation::Required,
            declaration: StressOperationExpectation::Required,
            type_definition: StressOperationExpectation::Forbidden,
            references: StressOperationExpectation::Required,
            completion: StressOperationExpectation::Skip,
        }
    }

    /// Return the expectation set for one imported type name.
    pub(super) fn import_type_name() -> Self {
        Self {
            hover: StressOperationExpectation::Required,
            definition: StressOperationExpectation::Required,
            declaration: StressOperationExpectation::Required,
            type_definition: StressOperationExpectation::Optional,
            references: StressOperationExpectation::Required,
            completion: StressOperationExpectation::Skip,
        }
    }

    /// Return the expectation set for one member completion anchor.
    pub(super) fn member_completion() -> Self {
        Self {
            hover: StressOperationExpectation::Skip,
            definition: StressOperationExpectation::Skip,
            declaration: StressOperationExpectation::Skip,
            type_definition: StressOperationExpectation::Skip,
            references: StressOperationExpectation::Skip,
            completion: StressOperationExpectation::Required,
        }
    }

    /// Return the expectation set for one statement completion anchor.
    pub(super) fn statement_completion() -> Self {
        Self {
            hover: StressOperationExpectation::Skip,
            definition: StressOperationExpectation::Skip,
            declaration: StressOperationExpectation::Skip,
            type_definition: StressOperationExpectation::Skip,
            references: StressOperationExpectation::Skip,
            completion: StressOperationExpectation::Required,
        }
    }
}

/// One materialized stress workspace.
#[derive(Debug)]
pub(super) struct StressWorkspace {
    /// The shared repository.
    pub repository: Arc<Repository>,
    /// The active revision for this stress workspace.
    pub revision: Revision,
    /// The generated project.
    pub project: StressProject,
    /// The materialized workspace root.
    root_path: PathBuf,
    /// The file ids keyed by relative path.
    file_ids_by_path: HashMap<String, FileId>,
    /// The relative paths keyed by file id.
    file_paths_by_id: HashMap<FileId, String>,
}

impl StressRecipe {
    /// Load one recipe from a fixture path.
    pub(super) fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

        toml::from_str(&content)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))
    }
}

impl StressProject {
    /// Build one generated project from a recipe.
    pub(super) fn from_recipe(recipe: &StressRecipe) -> Self {
        let mut generator = StressProjectGenerator::new(recipe.seed);
        generator.generate(recipe)
    }
}

impl StressWorkspace {
    /// Materialize one generated project into a compiled query-visible workspace.
    pub(super) fn materialize(label: &str, project: StressProject) -> Result<Self, String> {
        let workspace = SharedMemoryWorkspace::new("/test/stress");
        let root = workspace.allocate_root(label);
        let repository = workspace.repository();

        let (revision, file_ids_by_path, file_paths_by_id) =
            compile_and_index_stress_project(&repository, &root, &project)?;

        Ok(Self {
            repository,
            revision,
            project,
            root_path: root,
            file_ids_by_path,
            file_paths_by_id,
        })
    }

    /// Return the materialized workspace root.
    pub(super) fn workspace_root(&self) -> &Path {
        &self.root_path
    }

    /// Return the file id for one generated file path.
    pub(super) fn file_id(&self, path: &str) -> Result<FileId, String> {
        self.file_ids_by_path
            .get(path)
            .copied()
            .ok_or_else(|| format!("missing generated file id for {path}"))
    }

    /// Return the relative file path for one generated file id.
    pub(super) fn relative_path(&self, file_id: FileId) -> Result<&str, String> {
        self.file_paths_by_id
            .get(&file_id)
            .map(String::as_str)
            .ok_or_else(|| format!("missing generated file path for file id {file_id:?}"))
    }

    /// Validate one span against the materialized project.
    pub(super) fn validate_span(&self, span: Span) -> Result<(), String> {
        let file = self
            .repository
            .file(self.revision, span.file)
            .map_err(|error| format!("failed to load stress file {:?}: {error}", span.file))?
            .ok_or_else(|| format!("missing stress file {:?}", span.file))?;
        let file_text = file.text();
        let file_length = file_text.len() as u32;

        // reject inverted or out of bounds spans
        if span.start > span.end {
            return Err(format!("inverted span {span:?}"));
        }

        if span.end > file_length {
            return Err(format!(
                "out of bounds span {span:?} for file '{}' with {file_length} bytes",
                file.name,
            ));
        }

        Ok(())
    }

    /// Validate one edit against the materialized project.
    pub(super) fn validate_edit(&self, edit: &Edit) -> Result<(), String> {
        self.validate_span(edit.span)
    }
}

/// Return the shared directory for generated stress project recipes.
pub(super) fn project_recipe_dir() -> PathBuf {
    crate::core::fixtures_dir().join("stress").join("project")
}

/// Return the shared default value for boolean recipe fields.
fn default_true() -> bool {
    true
}

/// Compile and index one generated stress project.
fn compile_and_index_stress_project(
    repository: &Arc<Repository>,
    root: &Path,
    project: &StressProject,
) -> Result<(Revision, HashMap<String, FileId>, HashMap<FileId, String>), String> {
    let compiler = Arc::new(Compiler::new(repository.clone()));

    // materialize every generated source into the active repository revision first
    for file in &project.files {
        let file_path = root.join(&file.path);
        write_workspace_text_file(repository, &file_path, &file.text);
    }

    let revision = current_workspace_revision(repository)?;
    let mut module_ids = Vec::new();
    let mut module_ids_by_path = HashMap::new();

    // resolve every generated source file into one module
    for file in &project.files {
        let file_path = root.join(&file.path);
        let module_id = module_id_for_path(repository, revision, &file_path);

        module_ids.push(module_id);
        module_ids_by_path.insert(file.path.clone(), module_id);
    }

    module_ids.sort();
    module_ids.dedup();

    // enqueue the full query substrate for each module
    let mut artifact_keys = Vec::new();
    for module_id in &module_ids {
        let profile = default_profile_id_for_module(repository, revision, *module_id);
        artifact_keys.push(ArtifactKey::DirChecked {
            module: *module_id,
            profile,
        });
    }

    let revision = provide_workspace_artifacts(repository.clone(), compiler, &artifact_keys);
    repository.index_query_modules(revision, module_ids.iter().copied());
    repository.index_query_imports(revision);

    let mut file_ids_by_path = HashMap::new();
    let mut file_paths_by_id = HashMap::new();
    for (path, module_id) in module_ids_by_path {
        let module = repository
            .module(revision, module_id)
            .map_err(|error| format!("failed to load stress module {module_id:?}: {error}"))?
            .ok_or_else(|| format!("missing stress module {module_id:?}"))?;
        file_paths_by_id.insert(module.file_id, path.clone());
        file_ids_by_path.insert(path, module.file_id);
    }

    Ok((revision, file_ids_by_path, file_paths_by_id))
}

/// Return the current workspace revision for one repository.
fn current_workspace_revision(repository: &Repository) -> Result<Revision, String> {
    let reference = Ref::for_workspace_root(repository.workspace_root());

    repository
        .current(&reference)
        .map_err(|error| format!("missing current stress revision: {error}"))
}

/// One seeded project generator.
struct StressProjectGenerator {
    /// The deterministic local rng.
    rng: StressRng,
}

impl StressProjectGenerator {
    /// Create one seeded generator.
    fn new(seed: u64) -> Self {
        Self {
            rng: StressRng::new(seed),
        }
    }

    /// Generate one stress project from a recipe.
    fn generate(&mut self, recipe: &StressRecipe) -> StressProject {
        let malformed_modules = self.select_malformed_modules(recipe);
        let mut files = Vec::new();
        let mut anchors = Vec::new();
        let mut exported_names = Vec::new();

        for module_index in 0..recipe.library_modules {
            let module_path = format!("lib_{module_index}.ds");
            let mut file = GeneratedFile::new(module_path.clone());

            if module_index > 0 {
                let previous_index = module_index - 1;
                let previous_value = symbol_name("value", previous_index, 0);
                let previous_type = symbol_name("Type", previous_index, 0);
                let previous_function = symbol_name("make", previous_index, 0);

                file.push("import { ");
                file.push_anchor(
                    format!("import-value-{module_index}"),
                    StressAnchorKind::ImportName,
                    StressAnchorExpectations::import_value_name(),
                    &previous_value,
                );
                file.push(", ");
                file.push_anchor(
                    format!("import-function-{module_index}"),
                    StressAnchorKind::ImportName,
                    StressAnchorExpectations::import_value_name(),
                    &previous_function,
                );
                file.push(" } from \"./");
                file.push(&format!("lib_{previous_index}.ds"));
                file.push("\"\n");

                if recipe.include_type_imports {
                    file.push("import type { ");
                    file.push_anchor(
                        format!("import-type-{module_index}"),
                        StressAnchorKind::ImportName,
                        StressAnchorExpectations::import_type_name(),
                        &previous_type,
                    );
                    file.push(" } from \"./");
                    file.push(&format!("lib_{previous_index}.ds"));
                    file.push("\"\n");
                }

                if recipe.include_namespace_imports {
                    file.push("import * as previous_");
                    file.push(&module_index.to_string());
                    file.push(" from \"./");
                    file.push(&format!("lib_{previous_index}.ds"));
                    file.push("\"\n");
                }

                file.push("\n");
            }

            for declaration_index in 0..recipe.declarations_per_module {
                let value_name = symbol_name("value", module_index, declaration_index);
                let type_name = symbol_name("Type", module_index, declaration_index);
                let function_name = symbol_name("make", module_index, declaration_index);

                exported_names.push(type_name.clone());
                exported_names.push(function_name.clone());

                file.push("export const ");
                file.push(&value_name);
                file.push(": int32 = ");
                file.push(&((module_index + declaration_index + 1) as i32).to_string());
                file.push("\n\n");

                file.push("export type ");
                file.push(&type_name);
                file.push(" = {\n    value: int32,\n}\n\n");

                file.push("export function ");
                file.push(&function_name);
                file.push("(input: int32): ");
                file.push(&type_name);
                file.push(" {\n    return { value: input + ");
                file.push(&value_name);
                file.push(" }\n}\n\n");
            }

            if module_index > 0 {
                let previous_index = module_index - 1;
                let previous_value = symbol_name("value", previous_index, 0);
                let previous_type = symbol_name("Type", previous_index, 0);
                let previous_function = symbol_name("make", previous_index, 0);
                let local_function = symbol_name("make", module_index, 0);

                file.push("export function use_");
                file.push(&module_index.to_string());
                file.push("(input: int32): int32 {\n");
                file.push("    const previous = ");
                file.push_anchor(
                    format!("value-use-{module_index}"),
                    StressAnchorKind::ValueUse,
                    StressAnchorExpectations::value_use(),
                    &previous_function,
                );
                file.push("(input)\n");
                file.push("    const typed: ");
                file.push_anchor(
                    format!("type-use-{module_index}"),
                    StressAnchorKind::TypeUse,
                    StressAnchorExpectations::type_use(),
                    &previous_type,
                );
                file.push(" = previous\n");
                file.push("    const local = ");
                file.push(&local_function);
                file.push("(typed.value + ");
                file.push_anchor(
                    format!("imported-value-use-{module_index}"),
                    StressAnchorKind::ValueUse,
                    StressAnchorExpectations::value_use(),
                    &previous_value,
                );
                file.push(")\n");
                file.push("    const member_completion = local.");
                file.push("va");
                file.mark_anchor_here(
                    format!("member-prefix-{module_index}"),
                    StressAnchorKind::MemberPrefix,
                    StressAnchorExpectations::member_completion(),
                );
                file.push("\n");

                if recipe.include_namespace_imports {
                    file.push("    const namespace_value = previous_");
                    file.push(&module_index.to_string());
                    file.push(".");
                    file.push(&local_function_for_namespace(previous_index));
                    file.push("(input).value\n");
                    file.push("    return local.value + namespace_value\n");
                } else {
                    file.push("    return local.value\n");
                }

                file.push("}\n\n");

                exported_names.push(format!("use_{module_index}"));
            }

            file.push("export function gap_");
            file.push(&module_index.to_string());
            file.push("(): void {\n    ");
            file.mark_anchor_here(
                format!("statement-gap-{module_index}"),
                StressAnchorKind::StatementGap,
                StressAnchorExpectations::statement_completion(),
            );
            file.push("\n}\n");

            exported_names.push(format!("gap_{module_index}"));

            if malformed_modules.contains(&module_index) {
                file.push("\n");
                file.push("export function broken_");
                file.push(&module_index.to_string());
                file.push("( {\n");
                file.push("export function stable_");
                file.push(&module_index.to_string());
                file.push("(): int32 {\n    return ");
                file.push(&symbol_name("value", module_index, 0));
                file.push("\n}\n");

                exported_names.push(format!("stable_{module_index}"));
            }

            let (generated_file, file_anchors) = file.finish();
            anchors.extend(file_anchors);
            files.push(generated_file);
        }

        let main = self.generate_main_file(recipe);
        let (generated_file, file_anchors) = main.finish();
        anchors.extend(file_anchors);
        files.push(generated_file);

        let workspace_queries =
            self.build_workspace_queries(&exported_names, recipe.workspace_queries.max(1));

        StressProject {
            files,
            anchors,
            workspace_queries,
        }
    }

    /// Generate the main entry file.
    fn generate_main_file(&mut self, recipe: &StressRecipe) -> GeneratedFile {
        let mut file = GeneratedFile::new("main.ds".to_string());
        let import_count = recipe.library_modules.min(3);

        for module_index in 0..import_count {
            let value_name = symbol_name("value", module_index, 0);
            let type_name = symbol_name("Type", module_index, 0);
            let function_name = symbol_name("make", module_index, 0);

            file.push("import { ");
            file.push_anchor(
                format!("main-import-value-{module_index}"),
                StressAnchorKind::ImportName,
                StressAnchorExpectations::import_value_name(),
                &value_name,
            );
            file.push(", ");
            file.push_anchor(
                format!("main-import-function-{module_index}"),
                StressAnchorKind::ImportName,
                StressAnchorExpectations::import_value_name(),
                &function_name,
            );
            file.push(" } from \"./");
            file.push(&format!("lib_{module_index}.ds"));
            file.push("\"\n");

            if recipe.include_type_imports {
                file.push("import type { ");
                file.push_anchor(
                    format!("main-import-type-{module_index}"),
                    StressAnchorKind::ImportName,
                    StressAnchorExpectations::import_type_name(),
                    &type_name,
                );
                file.push(" } from \"./");
                file.push(&format!("lib_{module_index}.ds"));
                file.push("\"\n");
            }
        }

        file.push("\n");
        file.push("function main(value: int32): int32 {\n");

        if import_count > 0 {
            let first_function = symbol_name("make", 0, 0);
            let first_type = symbol_name("Type", 0, 0);
            let first_value = symbol_name("value", 0, 0);

            file.push("    const item: ");
            file.push_anchor(
                "main-type-use".to_string(),
                StressAnchorKind::TypeUse,
                StressAnchorExpectations::type_use(),
                &first_type,
            );
            file.push(" = ");
            file.push_anchor(
                "main-function-use".to_string(),
                StressAnchorKind::ValueUse,
                StressAnchorExpectations::value_use(),
                &first_function,
            );
            file.push("(value + ");
            file.push_anchor(
                "main-value-use".to_string(),
                StressAnchorKind::ValueUse,
                StressAnchorExpectations::value_use(),
                &first_value,
            );
            file.push(")\n");
            file.push("    const member_completion = item.");
            file.push("va");
            file.mark_anchor_here(
                "main-member-prefix".to_string(),
                StressAnchorKind::MemberPrefix,
                StressAnchorExpectations::member_completion(),
            );
            file.push("\n");
            file.push("    return item.value\n");
        } else {
            file.push("    ");
            file.mark_anchor_here(
                "main-statement-gap".to_string(),
                StressAnchorKind::StatementGap,
                StressAnchorExpectations::statement_completion(),
            );
            file.push("\n    return value\n");
        }

        file.push("}\n");

        file
    }

    /// Select the damaged module set for one recipe.
    fn select_malformed_modules(&mut self, recipe: &StressRecipe) -> HashSet<usize> {
        let mut malformed = HashSet::new();
        if recipe.library_modules == 0 {
            return malformed;
        }

        while malformed.len() < recipe.malformed_modules.min(recipe.library_modules) {
            malformed.insert(self.rng.index(recipe.library_modules));
        }

        malformed
    }

    /// Build the workspace symbol query set from exported names.
    fn build_workspace_queries(&mut self, names: &[String], count: usize) -> Vec<String> {
        let mut queries = Vec::new();
        if names.is_empty() {
            queries.push("value".to_string());
            return queries;
        }

        for _ in 0..count {
            let name = &names[self.rng.index(names.len())];
            queries.push(name.clone());
        }

        queries.sort();
        queries.dedup();
        queries
    }
}

/// One mutable generated source file.
struct GeneratedFile {
    /// The path for the file being built.
    path: String,
    /// The accumulated source text.
    text: String,
    /// The anchors recorded in this file.
    anchors: Vec<StressAnchor>,
}

impl GeneratedFile {
    /// Create one empty generated file.
    fn new(path: String) -> Self {
        Self {
            path,
            text: String::new(),
            anchors: Vec::new(),
        }
    }

    /// Append plain text.
    fn push(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// Append text and record its starting offset as one anchor.
    fn push_anchor(
        &mut self,
        name: String,
        kind: StressAnchorKind,
        expectations: StressAnchorExpectations,
        text: &str,
    ) {
        let offset = self.text.len() as u32;
        self.anchors.push(StressAnchor {
            name,
            path: self.path.clone(),
            offset,
            kind,
            expectations,
        });
        self.text.push_str(text);
    }

    /// Record one anchor at the current offset.
    fn mark_anchor_here(
        &mut self,
        name: String,
        kind: StressAnchorKind,
        expectations: StressAnchorExpectations,
    ) {
        let offset = self.text.len() as u32;
        self.anchors.push(StressAnchor {
            name,
            path: self.path.clone(),
            offset,
            kind,
            expectations,
        });
    }

    /// Finish the generated file.
    fn finish(self) -> (StressFile, Vec<StressAnchor>) {
        (
            StressFile {
                path: self.path,
                text: self.text,
            },
            self.anchors,
        )
    }
}

/// One tiny deterministic rng.
struct StressRng {
    /// The current generator state.
    state: u64,
}

impl StressRng {
    /// Create one deterministic rng from a seed.
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    /// Return the next pseudorandom word.
    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    /// Return one index below the given upper bound.
    fn index(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            return 0;
        }

        (self.next_u64() as usize) % upper
    }
}

/// Build one generated symbol name.
fn symbol_name(prefix: &str, module_index: usize, declaration_index: usize) -> String {
    format!("{prefix}_{module_index}_{declaration_index}")
}

/// Build one namespace call symbol for a prior module.
fn local_function_for_namespace(module_index: usize) -> String {
    symbol_name("make", module_index, 0)
}
