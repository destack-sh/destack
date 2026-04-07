use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use destack_lsp_types as lsp;

use super::core::{StressAnchor, StressProject, StressRecipe, StressWorkspace, project_recipe_dir};
use super::ide::{StressIdeBattery, StressIdeDriver, run_ide_invariants, run_ide_parity};
use super::query::QueryStressDriver;
use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite, discover_file_cases};
use crate::lsp::{
    LspDriver, LspExpectations, LspFixture, LspSourceFile, NormalizedCompletionItem,
    NormalizedLocation, NormalizedQuickInfo, NormalizedWorkspaceSymbol,
    normalize_completion_response, normalize_definition_response, normalize_quick_info,
    normalize_references_response, normalize_workspace_symbols,
};

/// Stress test suite for LSP parity over generated projects.
#[derive(Debug, Clone, Copy, Default)]
pub struct LspStressSuite;

/// One real LSP driver over one generated stress project.
struct LspStressDriver<'a> {
    /// The generated direct query workspace used for anchor positions.
    workspace: &'a StressWorkspace,
    /// The real in process LSP driver.
    driver: LspDriver,
}

impl Suite for LspStressSuite {
    fn name(&self) -> &'static str {
        "stress-lsp"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let stress_dir = project_recipe_dir();
        if !stress_dir.exists() {
            return vec![];
        }

        discover_file_cases(&stress_dir, &["toml"], "destack_test::stress::lsp").unwrap_or_default()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        run_lsp_stress(case)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(60))
    }
}

impl<'a> LspStressDriver<'a> {
    /// Create one LSP stress driver from one generated project.
    fn new(workspace: &'a StressWorkspace, recipe_path: &Path) -> Result<Self, String> {
        let fixture = fixture_for_project(recipe_path, &workspace.project);
        let mut driver = LspDriver::from_fixture("stress-lsp", &fixture)?;
        driver.open_fixture_files(&fixture)?;

        Ok(Self { workspace, driver })
    }

    /// Return the generated LSP position for one anchor.
    fn position(&self, anchor: &StressAnchor) -> Result<lsp::Position, String> {
        let file_id = self.workspace.file_id(&anchor.path)?;
        let file = self
            .workspace
            .repository
            .file(self.workspace.revision, file_id)
            .map_err(|error| format!("failed to read file snapshot: {error}"))?
            .ok_or_else(|| format!("missing file snapshot for {}", anchor.path))?;

        // generated stress sources are ascii only, so byte columns and utf16 columns coincide
        let (line, character) = file
            .get_position(anchor.offset)
            .ok_or_else(|| format!("failed to resolve lsp position for {}", anchor.name))?;

        Ok(lsp::Position { line, character })
    }
}

impl StressIdeDriver for LspStressDriver<'_> {
    fn label(&self) -> &'static str {
        "lsp"
    }

    fn hover(&mut self, anchor: &StressAnchor) -> Result<Option<NormalizedQuickInfo>, String> {
        let position = self.position(anchor)?;
        let hover = self.driver.hover(&anchor.path, position)?;
        let Some(hover) = hover else {
            return Ok(None);
        };

        normalize_quick_info(self.driver.workspace_root(), &anchor.path, &hover).map(Some)
    }

    fn goto_definition(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedLocation>, String> {
        let position = self.position(anchor)?;
        let definition = self.driver.goto_definition(&anchor.path, position)?;
        let Some(definition) = definition else {
            return Ok(Vec::new());
        };

        normalize_definition_response(self.driver.workspace_root(), &definition)
    }

    fn goto_declaration(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedLocation>, String> {
        let position = self.position(anchor)?;
        let declaration = self.driver.goto_declaration(&anchor.path, position)?;
        let Some(declaration) = declaration else {
            return Ok(Vec::new());
        };

        normalize_definition_response(self.driver.workspace_root(), &declaration)
    }

    fn goto_type_definition(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedLocation>, String> {
        let position = self.position(anchor)?;
        let type_definition = self.driver.goto_type_definition(&anchor.path, position)?;
        let Some(type_definition) = type_definition else {
            return Ok(Vec::new());
        };

        normalize_definition_response(self.driver.workspace_root(), &type_definition)
    }

    fn references(&mut self, anchor: &StressAnchor) -> Result<Vec<NormalizedLocation>, String> {
        let position = self.position(anchor)?;
        let references = self.driver.find_references(&anchor.path, position, true)?;
        let Some(references) = references else {
            return Ok(Vec::new());
        };

        normalize_references_response(self.driver.workspace_root(), &references)
    }

    fn completion(
        &mut self,
        anchor: &StressAnchor,
    ) -> Result<Vec<NormalizedCompletionItem>, String> {
        let position = self.position(anchor)?;
        let completion = self
            .driver
            .completion(&anchor.path, position)?
            .ok_or_else(|| format!("missing lsp completion at {}", anchor.name))?;

        normalize_completion_response(&completion)
    }

    fn workspace_symbols(&mut self, query: &str) -> Result<Vec<NormalizedWorkspaceSymbol>, String> {
        let symbols = self
            .driver
            .workspace_symbols(query)?
            .ok_or_else(|| format!("missing lsp workspace symbols for query '{query}'"))?;

        normalize_workspace_symbols(self.driver.workspace_root(), &symbols)
    }
}

/// Run one generated LSP stress case.
fn run_lsp_stress(case: &Case) -> CaseResult {
    let recipe = match StressRecipe::load(&case.path) {
        Ok(recipe) => recipe,
        Err(message) => return CaseResult::Failed { message },
    };
    let project = StressProject::from_recipe(&recipe);
    let workspace = match StressWorkspace::materialize(&case.name, project) {
        Ok(workspace) => workspace,
        Err(message) => return CaseResult::Failed { message },
    };
    let battery = StressIdeBattery::full(&workspace.project);
    let mut query_driver = QueryStressDriver::new(&workspace);
    let mut lsp_driver = match LspStressDriver::new(&workspace, &case.path) {
        Ok(driver) => driver,
        Err(message) => return CaseResult::Failed { message },
    };

    if let Err(message) = run_ide_invariants(&mut lsp_driver, &battery) {
        return CaseResult::Failed {
            message: format!("static lsp invariants failed: {message}"),
        };
    }

    if let Err(message) = run_ide_parity(&mut query_driver, &mut lsp_driver, &battery) {
        return CaseResult::Failed {
            message: format!("static query/lsp parity failed: {message}"),
        };
    }

    eprintln!(
        "  {} files, {} anchors, {} workspace queries",
        workspace.project.files.len(),
        workspace.project.anchors.len(),
        workspace.project.workspace_queries.len(),
    );

    CaseResult::Passed
}

/// Build one plain LSP fixture from a generated stress project.
fn fixture_for_project(recipe_path: &Path, project: &StressProject) -> LspFixture {
    let files = project
        .files
        .iter()
        .map(|file| LspSourceFile {
            path: file.path.clone(),
            text: file.text.clone(),
        })
        .collect();

    LspFixture {
        path: recipe_path.to_path_buf(),
        files,
        step_snapshots: BTreeMap::new(),
        markers: BTreeMap::new(),
        ranges: Vec::new(),
        step_cases: BTreeMap::new(),
        expectations: LspExpectations::default(),
    }
}
