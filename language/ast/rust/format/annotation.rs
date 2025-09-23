use dyst_language_fir::format::{Format, FormatResult, hard_line_break};
use dyst_language_fir::prelude::*;
use dyst_language_fir::write;

use crate::{
    Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Doc, DocStyle, DystFormatContext,
    DystFormatter, FormatNode, Node, NodeId, NodeTree, NodeTreeStore, NodeType,
};

impl<'ast> DystFormatContext<'ast> {
    /// Format the infix annotations for a node.
    #[inline]
    pub fn infix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::BlockInfix,
            node_id,
        }
    }

    /// Format the prefix annotations for a node.
    #[inline]
    pub fn prefix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::BlockPrefix,
            node_id,
        }
    }

    /// Format the postfix annotations for a node.
    #[inline]
    pub fn postfix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::BlockPostfix,
            node_id,
        }
    }

    /// Format the line suffix annotations for a node.
    #[inline]
    pub fn suffix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::LineSuffix,
            node_id,
        }
    }

    /// Format the postfix and line suffix annotations for a node.
    #[inline]
    pub fn suffix_and_postfix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::BlockPostfixAndLineSuffix,
            node_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnotationCapture {
    /// Annotations inside the node (without next node to attach to, like in an empty block.)
    BlockInfix,
    /// Annotation preceding the node on previous lines (most common).
    BlockPrefix,
    /// Annotation after the node on a new line (only if prefix and infix are not possible).
    BlockPostfix,
    /// Annotation after the node on the same line (like infix comments).
    LineSuffix,
    /// Annotation after the node on a new line (only if prefix and infix are not possible).
    BlockPostfixAndLineSuffix,
}

/// Annotations for a node.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotations<T: Node> {
    /// The position of the annotations.
    position: AnnotationCapture,
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
        let mut first_node_type: Option<NodeType> = None;
        for annotation_id in annotations {
            let annotation = f.context().tree.get::<Annotation>(annotation_id);
            let (node_type, position) = match annotation {
                Annotation::Blank { position, .. } => (NodeType::Blank, *position),
                Annotation::Doc { position, .. } => (NodeType::Doc, *position),
                Annotation::Comment { position, .. } => (NodeType::Comment, *position),
            };
            let is_included = match position {
                AnnotationPosition::BlockInfix => self.position == AnnotationCapture::BlockInfix,
                AnnotationPosition::BlockPrefix => self.position == AnnotationCapture::BlockPrefix,
                AnnotationPosition::BlockPostfix => {
                    self.position == AnnotationCapture::BlockPostfix
                        || self.position == AnnotationCapture::BlockPostfixAndLineSuffix
                }
                AnnotationPosition::LineSuffix => {
                    self.position == AnnotationCapture::LineSuffix
                        || self.position == AnnotationCapture::BlockPostfixAndLineSuffix
                }
            };
            if is_included {
                // insert space/newline for first annotation in group
                if first_node_type.is_none() {
                    first_node_type = Some(node_type);
                    if position == AnnotationPosition::LineSuffix {
                        write!(f, [space()])?;
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }

                // format annotation itself
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
