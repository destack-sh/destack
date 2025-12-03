use destack_dir::{self as dir, Module, NodeTree};
use destack_javascript_ast::{Annotation, AnnotationPosition, LocalNodeId};

use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};

impl Transpiler {
    /// Transpile a DIR annotation position into a JS annotation position.
    pub fn transpile_annotation_position(
        &self,
        position: dir::AnnotationPosition,
    ) -> AnnotationPosition {
        match position {
            dir::AnnotationPosition::Prefix => AnnotationPosition::Prefix,
            dir::AnnotationPosition::Infix => AnnotationPosition::Infix,
            dir::AnnotationPosition::Postfix => AnnotationPosition::Postfix,
        }
    }

    /// Transpile an annotation from DIR into JS AST.
    pub fn transpile_annotation(
        &self,
        module: &Module,
        tree: &NodeTree,
        _scope_id: dir::LocalNodeIdAny,
        annotation_id: dir::LocalNodeId<dir::Annotation>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<Annotation>> {
        let annotation = tree.get(annotation_id);
        let annotation = match annotation {
            dir::Annotation::Doc { position, string } => {
                let position = self.transpile_annotation_position(*position);
                let string = unit.strings.intern_from(&self.program.strings, *string);
                Annotation::Doc { position, string }
            }
            dir::Annotation::Comment { position, string } => {
                let position = self.transpile_annotation_position(*position);
                let string = unit.strings.intern_from(&self.program.strings, *string);
                Annotation::Comment { position, string }
            }

            dir::Annotation::Decorator {
                position: _,
                left: _,
                arguments: _,
            } => {
                // NOTE #Incomplete: properly transpile decorators to JS decorator syntax
                return Err(TranspileError::UnsupportedNode {
                    node: annotation_id.into_global_any(module.id),
                    message: None,
                });
            }
        };
        let annotation_id = unit
            .ast
            .insert_from_source(annotation, module.id, annotation_id);
        Ok(annotation_id)
    }
}
