use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR annotation position to an AST annotation position.
    #[inline]
    pub(super) fn unbind_annotation_position(
        &self,
        _context: &mut UnbindContext,
        position: dir::AnnotationPosition,
    ) -> ast::AnnotationPosition {
        match position {
            dir::AnnotationPosition::Prefix => ast::AnnotationPosition::LinePrefix,
            dir::AnnotationPosition::Infix => ast::AnnotationPosition::BlockInfix,
            dir::AnnotationPosition::Postfix => ast::AnnotationPosition::LinePostfix,
        }
    }

    /// Unbind a DIR annotation to an AST annotation.
    pub(super) fn unbind_annotation(
        &self,
        module: &Module,
        annotation_id: dir::LocalNodeId<dir::Annotation>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Annotation> {
        let annotation = tree.get(annotation_id);
        let span = self.unbind_span(module, annotation_id.into());
        let ast_annotation = match annotation {
            dir::Annotation::Doc { position, string } => {
                let position = self.unbind_annotation_position(context, *position);
                let string = ast_strings.intern_from(&self.program.strings, *string);
                let doc = ast::Doc {
                    string,
                    style: ast::DocStyle::Slash,
                };
                let doc_id = ast_tree.insert(doc, span);
                ast::Annotation::Doc {
                    node: doc_id,
                    position,
                }
            }
            dir::Annotation::Comment { position, string } => {
                let position = self.unbind_annotation_position(context, *position);
                let string = ast_strings.intern_from(&self.program.strings, *string);
                let comment = ast::Comment {
                    string,
                    style: ast::CommentStyle::Slash,
                };
                let comment_id = ast_tree.insert(comment, span);
                ast::Annotation::Comment {
                    node: comment_id,
                    position,
                }
            }
            dir::Annotation::Decorator {
                position,
                left,
                arguments,
            } => {
                let position = self.unbind_annotation_position(context, *position);
                // Unbind the left expression to get a path
                let left_expr = tree.get(*left);
                let path = match left_expr {
                    dir::Expression::UnresolvedPath { path, .. }
                    | dir::Expression::LocalReference { path, .. }
                    | dir::Expression::ModuleReference { path, .. }
                    | dir::Expression::GlobalReference { path, .. } => {
                        self.unbind_path(path, ast_strings, context)
                    }
                    _ => {
                        // Fallback: a single-segment path with an error placeholder
                        ast::Path {
                            segments: smallvec::smallvec![ast_strings.intern("__error__")],
                        }
                    }
                };
                let arguments = arguments.as_ref().map(|args| {
                    args.iter()
                        .map(|arg| {
                            self.unbind_argument(
                                module,
                                *arg,
                                tree,
                                symbols,
                                ast_tree,
                                ast_strings,
                                context,
                            )
                        })
                        .collect()
                });
                let decorator = ast::Decorator {
                    left: path,
                    arguments,
                };
                let decorator_id = ast_tree.insert(decorator, span);
                ast::Annotation::Decorator {
                    node: decorator_id,
                    position,
                }
            }
        };
        let ast_annotation_id = ast_tree.insert(ast_annotation, span);
        context.map(annotation_id.into_any(), ast_annotation_id.into_any());
        ast_annotation_id
    }

    /// Attach all annotations for a DIR module to the given AST tree.
    pub(super) fn attach_unbind_annotations(
        &self,
        module: &Module,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) {
        let nodes: Vec<(dir::LocalNodeIdAny, ast::LocalNodeIdAny)> = context
            .node_map
            .iter()
            .map(|(dir_id, ast_id)| (*dir_id, *ast_id))
            .collect();
        for (dir_id, ast_id) in nodes {
            let annotations = tree.get_annotations(dir_id.id);
            for annotation_id in annotations {
                let ast_annotation_id = self.unbind_annotation(
                    module,
                    annotation_id,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast_tree.append_annotation(ast_id.id, ast_annotation_id);
            }
        }
    }
}
