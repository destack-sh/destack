use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Annotation, AnnotationPosition, NodeId};

use crate::{TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
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
        module: &'a Module,
        scope_id: dir::NodeIdAny,
        annotation_id: dir::NodeId<dir::Annotation>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Annotation>> {
        let annotation = self.session.tree.get(annotation_id);
        let annotation = match annotation.as_ref() {
            dir::Annotation::Doc { position, string } => {
                let position = self.transpile_annotation_position(*position);
                let string = unit.strings.intern_from(&self.session.strings, *string);
                Annotation::Doc { position, string }
            }
            dir::Annotation::Comment { position, string } => {
                let position = self.transpile_annotation_position(*position);
                let string = unit.strings.intern_from(&self.session.strings, *string);
                Annotation::Comment { position, string }
            }

            // transpile tag/decorator annotations to plain comments
            // (since there is no real equivalent in JS for remaining unresolved tags/decorators)
            dir::Annotation::Tag {
                position,
                symbol,
                arguments: _,
            } => {
                let position = self.transpile_annotation_position(*position);
                let receiver = self.transpile_path(module, scope_id, receiver, unit)?;
                let receiver_str = format!("#{}", self.render_path(&receiver, unit));
                let receiver_str = unit.strings.intern(receiver_str);
                Annotation::Comment {
                    position,
                    string: receiver_str,
                }
            }
            dir::Annotation::Decorator {
                position,
                receiver,
                arguments: _,
            } => {
                let position = self.transpile_annotation_position(*position);
                let receiver = self.transpile_path(module, scope_id, receiver, unit)?;
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
