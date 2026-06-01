use destack_core::StringId;
use destack_dir as dir;
use destack_qir::AnnotationEntry;

use crate::core::{DirQueryContext, ModuleQueryContext};

impl ModuleQueryContext<'_> {
    /// Build annotation index entries for this module.
    pub(crate) fn build_annotation_candidates(&self) -> Vec<AnnotationEntry> {
        let dir = self.dir();
        let view = dir.view();
        let mut entries = Vec::new();

        // collect visible decorators by annotated target
        for (target_id, decorator_ids) in view.get_all_decorators() {
            let target_id = dir::LocalNodeIdAny {
                id: target_id,
                ty: view.get_node_type(target_id),
            };
            let target_id = target_id.into_global(dir.module_id());

            for decorator_id in decorator_ids {
                let decorator = view.get(decorator_id);
                let name = dir
                    .decorator_name_id(decorator)
                    .map(|name| dir.strings().get(name).to_string());
                let decorator_id = decorator_id.into_global_any(dir.module_id());

                entries.push(AnnotationEntry {
                    name,
                    module_id: dir.module_id(),
                    decorator_id,
                    target_id,
                });
            }
        }

        entries
    }
}

impl DirQueryContext<'_> {
    /// Resolve the last segment of a decorator name when it is path-like.
    pub(crate) fn decorator_name_id(self, decorator: &dir::Decorator) -> Option<StringId> {
        let mut expression_id = decorator.expression;
        loop {
            match self.tree().get(expression_id) {
                dir::Expression::Parenthesized { expression } => {
                    expression_id = *expression;
                }
                dir::Expression::Call { left, .. } => {
                    expression_id = *left;
                }
                dir::Expression::QualifiedReference { path, .. } => {
                    return path.segments.last().copied();
                }
                dir::Expression::Identifier { name } => return Some(*name),
                _ => return None,
            }
        }
    }
}
