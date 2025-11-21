use dyst_ast::StringPool;
use dyst_dir::ModuleRegistry;
use dyst_javascript_ast as ast;
use dyst_source::{FileId, Uri};

use crate::{TranspileOptions, Transpiler, TranspilerMode, TranspilerUnit, TranspilerUnitId};

impl<'a> Transpiler<'a> {
    /// Map the modules to the units.
    pub(crate) fn make_units(
        options: &TranspileOptions,
        modules: &ModuleRegistry,
    ) -> Vec<TranspilerUnit> {
        let mut units: Vec<TranspilerUnit> = Vec::new();
        match options.mode {
            // map every module to an artifact
            TranspilerMode::Retained => {
                for (idx, module) in modules.iter().enumerate() {
                    let module = module.read();
                    let unit_id = TranspilerUnitId::new(idx as u32);
                    let uri = module.uri.without_extension();
                    let unit = TranspilerUnit {
                        options: options.clone(),
                        id: unit_id,
                        uri,
                        ast: ast::NodeTree::new(),
                        roots: Vec::new(),
                        strings: StringPool::new(),
                        sources: vec![module.id],
                        pending_diagnostics: Vec::new(),
                        artifacts: Vec::new(),
                    };
                    units.push(unit);
                }
            }
            // map all modules to a single artifact
            TranspilerMode::Combined => {
                let unit = TranspilerUnit {
                    options: options.clone(),
                    id: TranspilerUnitId::new(0),
                    uri: Uri::from_string("combined"),
                    ast: ast::NodeTree::new(),
                    roots: Vec::new(),
                    strings: StringPool::new(),
                    sources: modules.iter().map(|module| module.read().id).collect(),
                    pending_diagnostics: Vec::new(),
                    artifacts: Vec::new(),
                };
                units.push(unit);
            }
        }
        units
    }

    /// Transpile the compiler's DIR into JS/TS artifacts.
    pub fn transpile(&self) {
        // transpile each module into AST
        let mut units = Transpiler::make_units(&self.options, &self.session.modules);
        let tree = self.session.tree.read();
        for unit in units.iter_mut() {
            for source_module_id in unit.sources.clone() {
                let source_module = self
                    .session
                    .modules
                    .get(source_module_id)
                    .unwrap_or_else(|| panic!("source module not found: {source_module_id:?}"));
                let source_module = source_module.read();
                self.transpile_module(&source_module, &tree, unit);
            }
        }

        // print content
        for (file_idx, mut unit) in units.into_iter().enumerate() {
            let file_id = FileId::new(file_idx as u32);
            for language in self.options.target.language_targets() {
                let formatting = self.options.formatting.with_language(language);
                match self.generate_artifact(&unit, file_id, formatting, language) {
                    Ok(artifact) => {
                        unit.artifacts.push(artifact.file.uri.clone());
                        self.artifacts
                            .write()
                            .insert(artifact.file.uri.clone(), artifact);
                    }
                    Err(error) => unit.error(error),
                }
            }
            // add all the diagnostics to the session
            for diagnostic in unit.pending_diagnostics.drain(..) {
                self.diagnostic(diagnostic);
            }
            self.units.write().insert(unit.uri.clone(), unit);
        }

        // flush remaining diagnostics
        self.flush_diagnostics();
    }
}
