use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use crate::Compiler;

#[allow(dead_code)]
impl Compiler {
    /// Unbind a DIR annotation position to an AST annotation position.
    /// Note: DIR has fewer positions than AST, so we map conservatively.
    #[inline]
    pub(super) fn unbind_annotation_position(
        &self,
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
    ) -> ast::LocalNodeId<ast::Annotation> {
        let annotation = tree.get(annotation_id);
        let span = self.unbind_span(module, annotation_id.into());
        let ast_annotation = match annotation {
            dir::Annotation::Doc { position, string } => {
                let position = self.unbind_annotation_position(*position);
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
                let position = self.unbind_annotation_position(*position);
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
                let position = self.unbind_annotation_position(*position);
                // Unbind the left expression to get a path
                let left_expr = tree.get(*left);
                let path = match left_expr {
                    dir::Expression::UnresolvedPath { path, .. }
                    | dir::Expression::LocalReference { path, .. }
                    | dir::Expression::ModuleReference { path, .. }
                    | dir::Expression::GlobalReference { path, .. } => {
                        self.unbind_path(path, ast_strings)
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
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings)
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
        ast_tree.insert(ast_annotation, span)
    }
}
