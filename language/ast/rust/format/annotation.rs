use dyst_language_fir::format::{Format, FormatResult, hard_line_break};
use dyst_language_fir::prelude::*;
use dyst_language_fir::write;

use crate::{
    Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Doc, DocStyle, DystFormatContext,
    DystFormatter, FormatNode, Node, NodeId, NodeTree, NodeTreeStore,
};

impl<'ast> DystFormatContext<'ast> {
    /// Format the preceding annotations for a node.
    #[inline]
    pub fn prefix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationPosition::BlockPrefix,
            node_id,
        }
    }

    /// Format the following annotations for a node.
    #[inline]
    pub fn postfix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationPosition::BlockPostfix,
            node_id,
        }
    }

    /// Format the infix annotations for a node.
    #[inline]
    pub fn infix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationPosition::BlockInfix,
            node_id,
        }
    }
}

/// Annotations for a node.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotations<T: Node> {
    /// The position of the annotations.
    position: AnnotationPosition,
    /// The node ID.
    node_id: NodeId<T>,
}

impl<'ast, T> Format<DystFormatContext<'ast>> for Annotations<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeStore<T>,
{
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let Some(annotations) = f.context().get_annotations(self.node_id) else {
            return Ok(());
        };
        for annotation_id in annotations {
            let annotation = f.context().tree.get::<Annotation>(annotation_id);
            let position = match annotation {
                Annotation::Blank { position, .. } => *position,
                Annotation::Doc { position, .. } => *position,
                Annotation::Comment { position, .. } => *position,
            };
            if position == self.position {
                annotation.format_node(annotation_id, f)?;
            }
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
    #[inline]
    fn format_node(
        &self,
        node_id: NodeId<Annotation>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Annotation::Blank { node, .. } => {
                // skip blanks at the end of the source
                let container = f.context().get_container(node_id);
                if let Some((container_id, _)) = container {
                    let container_span = f.context().get_span_by_id(container_id);
                    if container_span.end >= f.context().source.len - 1 {
                        return Ok(());
                    }
                }

                write!(f, [node])
            }
            Annotation::Doc { node, .. } => {
                write!(f, [node, hard_line_break()])
            }
            Annotation::Comment { node, .. } => {
                write!(f, [node, hard_line_break()])
            }
        }
    }
}

impl<'ast> FormatNode<'ast, Blank> for Blank {
    #[inline]
    fn format_node(
        &self,
        _node_id: NodeId<Blank>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // reduce any number of blank lines to a single one
        write!(f, [empty_line()])?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Doc> for Doc {
    #[inline]
    fn format_node(
        &self,
        _node_id: NodeId<Doc>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self.style {
            DocStyle::Line => {
                // line doc comment with `///`
                let string = f.context().session.strings.get(self.string);
                if string.contains('\n') {
                    // prefix every line with `///`
                    let string = string
                        .lines()
                        .map(|line| format!("/// {line}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    write!(f, [token("///"), space(), text(string.as_ref())])?;
                } else {
                    write!(f, [token("///"), space(), text(string)])?;
                }
            }
            DocStyle::Block => {
                // block doc comment with `/**` and `*/`
                write!(
                    f,
                    [token("/**"), space(), self.string, space(), token("*/")]
                )?;
            }
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Comment> for Comment {
    #[inline]
    fn format_node(
        &self,
        _node_id: NodeId<Comment>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self.style {
            CommentStyle::Line => {
                // line comment with `//`
                let string = f.context().session.strings.get(self.string);
                if string.contains('\n') {
                    // prefix every line with `//`
                    let string = string
                        .lines()
                        .map(|line| format!("// {line}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    write!(f, [text(string.as_ref())])?;
                } else {
                    write!(f, [token("//"), space(), text(string),])?;
                }
            }
            CommentStyle::Block => {
                // block comment with `/*` and `*/`
                write!(f, [token("/*"), space(), self.string, space(), token("*/")])?;
            }
        }
        Ok(())
    }
}
