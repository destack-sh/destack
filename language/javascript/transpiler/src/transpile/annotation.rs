use destack_dir::{self as dir, Module, NodeTree};
use destack_javascript_ast::{Annotation, AnnotationPosition, LocalNodeId};

use crate::{TranspileResult, Transpiler, TranspilerUnit};

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
        scope_id: dir::LocalNodeIdAny,
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

            // transpile tag/decorator annotations to plain comments
            // (since there is no real equivalent in JS for remaining unresolved tags/decorators)
            dir::Annotation::Tag {
                position,
                left,
                target_symbol: _,
                arguments: _,
            }
            | dir::Annotation::UnresolvedTag {
                position,
                left,
                arguments: _,
            } => {
                let position = self.transpile_annotation_position(*position);
                let receiver = self.transpile_path(module, scope_id, left, unit)?;
                let receiver_str = format!("#{}", self.render_path(&receiver, unit));
                let receiver_str = unit.strings.intern(receiver_str);
                Annotation::Comment {
                    position,
                    string: receiver_str,
                }
            }
            dir::Annotation::Decorator {
                position,
                left,
                target_symbol: _,
                arguments: _,
            }
            | dir::Annotation::UnresolvedDecorator {
                position,
                left,
                arguments: _,
            } => {
                let position = self.transpile_annotation_position(*position);
                let receiver = self.transpile_path(module, scope_id, left, unit)?;
                let receiver_str = format!("@{}", self.render_path(&receiver, unit));
                let receiver_str = unit.strings.intern(receiver_str);
                Annotation::Comment {
                    position,
                    string: receiver_str,
                }
            }
        };
        let annotation_id = unit
            .ast
            .insert_from_source(annotation, module.id, annotation_id);
        Ok(annotation_id)
    }
}
