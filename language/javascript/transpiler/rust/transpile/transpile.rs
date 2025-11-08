use std::collections::HashMap;

use dyst_dir::ModuleGraph;
use dyst_javascript_ast as ast;
use dyst_source::{FileContent, FileId, FileType};

use crate::{
    LanguageFormatOptions, Transpiler, TranspilerArtifact, TranspilerMode, TranspilerOptions,
};

impl<'a> Transpiler<'a> {
    /// Map the modules to the artifacts for the transpiled files.
    pub(crate) fn map_artifacts(
        options: TranspilerOptions,
        modules: &ModuleGraph,
    ) -> HashMap<FileId, TranspilerArtifact> {
        let mut artifacts: HashMap<FileId, TranspilerArtifact> = HashMap::new();
        // map every module to an artifact
        if options.mode == TranspilerMode::Retained {
            for module in modules.iter() {
                let artifact = TranspilerArtifact {
                    id: module.file.id,
                    ty: module.file.ty,
                    ast: ast::NodeTree::new(module.file.id),
                    sources: vec![module.file.id],
                    content: FileContent::Unloaded,
                };
                artifacts.insert(module.file.id, artifact);
            }
        }
        // other
        else {
            todo!("create_artifacts({:?})", options.mode)
        }

        artifacts
    }

    /// Transpile the compiler's DIR into JS/TS/.. artifacts.
    pub fn transpile(&mut self) {
        // transpile into AST
        // ...

        // format content
        let context = LanguageFormatOptions::default();
        for artifact in self.artifacts.values() {}
    }
}
