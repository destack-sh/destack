use dyst_fir::format::{Format, FormatResult, hard_line_break};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

use crate::{
    Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Doc, DocStyle, DystFormatContext,
    DystFormatter, FormatNode, Node, NodeId, NodeTree, NodeTreeStore, NodeType, Tag,
};

impl<'ast> DystFormatContext<'ast> {
    /// Format the block infix annotations for a node.
    #[inline]
    pub fn block_infix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::BlockInfix,
            node_id,
        }
    }

    /// Format the block prefix annotations for a node.
    #[inline]
    pub fn block_prefix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::BlockPrefix,
            node_id,
        }
    }

    /// Format the block postfix annotations for a node.
    #[inline]
    pub fn block_postfix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::BlockPostfix,
            node_id,
        }
    }

    /// Format the line prefix annotations for a node.
    #[inline]
    pub fn line_prefix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::LinePrefix,
            node_id,
        }
    }

    /// Format the line postfix annotations for a node.
    #[inline]
    pub fn line_postfix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::LinePostfix,
            node_id,
        }
    }

    /// Format the line postfix boundary annotations for a node.
    #[inline]
    pub fn line_postfix_boundary_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::LinePostfixBoundary,
            node_id,
        }
    }

    /// Format the line and block prefix annotations for a node.
    #[inline]
    pub fn any_prefix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::AnyPrefix,
            node_id,
        }
    }

    /// Format the line and block postfix annotations for a node.
    #[inline]
    pub fn any_postfix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::AnyPostfix,
            node_id,
        }
    }

    /// Format the line and block infix or postfix annotations for a node.
    #[inline]
    pub fn any_infix_or_postfix_annotations<T: Node>(&self, node_id: NodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::AnyInfixOrPostfix,
            node_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnotationCapture {
    BlockInfix,
    BlockPrefix,
    BlockPostfix,
    LinePrefix,
    LinePostfix,
    LinePostfixBoundary,
    AnyPrefix,
    AnyPostfix,
    AnyInfixOrPostfix,
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
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let Some(annotations) = f.context().get_annotations(self.node_id) else {
            return Ok(());
        };
        let mut first_node_type: Option<NodeType> = None;
        for annotation_id in annotations {
            // read annotation
            let annotation = f.context().tree.get::<Annotation>(annotation_id);
            let (node_type, position) = match annotation {
                Annotation::Blank { position, .. } => (NodeType::Blank, *position),
                Annotation::Doc { position, .. } => (NodeType::Doc, *position),
                Annotation::Comment { position, .. } => (NodeType::Comment, *position),
                Annotation::Tag { position, .. } => (NodeType::Tag, *position),
            };

            // filter annotation
            let is_included = match position {
                AnnotationPosition::BlockInfix => {
                    self.position == AnnotationCapture::BlockInfix
                        || self.position == AnnotationCapture::AnyInfixOrPostfix
                }
                AnnotationPosition::BlockPrefix => {
                    self.position == AnnotationCapture::BlockPrefix
                        || self.position == AnnotationCapture::AnyPrefix
                }
                AnnotationPosition::BlockPostfix => {
                    self.position == AnnotationCapture::BlockPostfix
                        || self.position == AnnotationCapture::AnyPostfix
                        || self.position == AnnotationCapture::AnyInfixOrPostfix
                }
                AnnotationPosition::LinePrefix => {
                    self.position == AnnotationCapture::LinePrefix
                        || self.position == AnnotationCapture::AnyPrefix
                }
                AnnotationPosition::LinePostfix => {
                    self.position == AnnotationCapture::LinePostfix
                        || self.position == AnnotationCapture::AnyPostfix
                        || self.position == AnnotationCapture::AnyInfixOrPostfix
                }
                AnnotationPosition::LinePostfixBoundary => {
                    self.position == AnnotationCapture::LinePostfixBoundary
                        || self.position == AnnotationCapture::AnyPostfix
                        || self.position == AnnotationCapture::AnyInfixOrPostfix
                }
            };
            if !is_included {
                continue;
            }

            // insert space/newline for first annotation in group
            if first_node_type.is_none() {
                first_node_type = Some(node_type);

                // insert space / newline
                match position {
                    AnnotationPosition::BlockInfix
                    | AnnotationPosition::BlockPrefix
                    | AnnotationPosition::BlockPostfix => {
                        write!(f, [hard_line_break()])?;
                    }
                    AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary => {
                        write!(f, [space()])?;
                    }
                    AnnotationPosition::LinePrefix => {
                        // no spacing needed for line prefix
                    }
                }
            }

            // format annotation itself
            annotation.format_node(annotation_id, f)?;

            // insert space / newline
            match position {
                AnnotationPosition::LinePrefix | AnnotationPosition::LinePostfix => {
                    write!(f, [space()])?;
                }
                AnnotationPosition::BlockInfix
                | AnnotationPosition::BlockPrefix
                | AnnotationPosition::BlockPostfix => {
                    write!(f, [hard_line_break()])?;
                }
                AnnotationPosition::LinePostfixBoundary => {
                    write!(f, [soft_line_break()])?;
                }
            }
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
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

                node.format(f)
            }
            Annotation::Doc { node, .. } => node.format(f),
            Annotation::Comment { node, .. } => node.format(f),
            Annotation::Tag { node, .. } => node.format(f),
        }
    }
}

impl<'ast> FormatNode<'ast, Blank> for Blank {
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
    fn format_node(
        &self,
        _node_id: NodeId<Doc>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let string = f.context().session.strings.get(self.string);
        let is_multi_line = string.contains('\n');
        match self.style {
            DocStyle::Star if !is_multi_line => {
                // block doc comment with `/**` and `*/`
                //  (unless multiline, we auto-convert to line comments)
                write!(
                    f,
                    [token("/**"), space(), self.string, space(), token("*/")]
                )?;
            }
            _ => {
                // prefix every line with `///`
                for (i, line) in string.lines().enumerate() {
                    write!(f, [token("///"), space(), text(line)])?;
                    if i < string.lines().count() - 1 {
                        write!(f, [hard_line_break()])?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Comment> for Comment {
    fn format_node(
        &self,
        _node_id: NodeId<Comment>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let string = f.context().session.strings.get(self.string);
        let is_multi_line = string.contains('\n');
        match self.style {
            CommentStyle::Star if !is_multi_line => {
                // block comment with `/*` and `*/`
                //  (unless multiline, we auto-convert to line comments)
                write!(f, [token("/*"), space(), self.string, space(), token("*/")])?;
            }
            _ => {
                // prefix every line with `//`
                let string = f.context().session.strings.get(self.string);
                for (i, line) in string.lines().enumerate() {
                    write!(f, [token("//"), space(), text(line)])?;
                    if i < string.lines().count() - 1 {
                        write!(f, [hard_line_break()])?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Tag> for Tag {
    fn format_node(
        &self,
        _node_id: NodeId<Tag>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [token("#"), self.receiver])?;
        if let Some(arguments) = &self.arguments
            && !arguments.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token("("),
                    soft_block_indent(&format_with(|f| f
                        .join_with(&format_args![
                            if_group_fits_on_line(&token(",")),
                            soft_line_break_or_space()
                        ])
                        .entries(arguments)
                        .finish())),
                    token(")")
                ])]
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    /// Tags should be preserved in order.
    #[test]
    fn test_format_tags_around_struct() {
        let source = r#"struct Entity {
    /// name
    #BeginGroup(17)
    #internal #something name: String #name
    numBananas: int32 #bananas
    #EndGroup
}"#;
        assert_format!(
            source,
            source,
            |p| p.eat_struct(None),
            DystFormatOptions::default()
        );
    }

    /// Multiple comments around an expression should retain their order.
    #[test]
    fn test_format_multiple_comments_around_statement() {
        let source = "{
    // comment part 1
    // comment part 2
    let A = 1
    // comment part 3
    // comment part 4
}";
        assert_format!(
            source,
            source,
            |p| p.eat_statement(),
            DystFormatOptions::default()
        );
    }

    /// Annotations should default to infix within source if no other position is found.
    #[test]
    fn test_format_annotations_fallback_to_infix() {
        assert_format!(
            "#A struct #B Test #C { #D } #E",
            "#A struct Test {\n\t#B\n\t#C\n\t#D\n} #E\n",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    /// Multiple comments around an expression should retain their order across successive blocks.
    #[test]
    fn test_format_multiple_comments_around_statement_in_successive_blocks() {
        let source = "{
    // comment part 0
    a: {
        // comment part 1
        // comment part 2
        let A = 1
        // comment part 3
        // comment part 4
    }
    // comment part 5
    // comment part 6
    b: {
        // comment part 7
        // comment part 8
        let B = 2
        // comment part 9
        // comment part 10
    }
}";
        assert_format!(
            source,
            source,
            |p| p.eat_statement(),
            DystFormatOptions::default()
        );
    }

    /// Inline statement comments should be preserved with proper spacing.
    #[test]
    fn test_format_inline_statement_comment() {
        assert_format!(
            "/* Pre-X comment */let X=/* Pre-A comment */A/* A comment */&&B/* B comment */",
            "/* Pre-X comment */ let X = /* Pre-A comment */ A /* A comment */ && B /* B comment */",
            |p| p.eat_statement(),
            DystFormatOptions::default_with_line_width(200)
        );
    }

    /// Convert multiline block doc comments to doc line comments.
    #[test]
    fn test_format_multine_block_doc_comment_to_line_comment() {
        assert_format!(
            "{
    /** some multiline
     * doc comment
     * over multiple lines */
    let X = 1 
}",
            "{
    /// some multiline
    /// doc comment
    /// over multiple lines
    let X = 1
}",
            |p| p.eat_block(),
            DystFormatOptions::default()
        );
    }

    /// Suffix multiline annotation should be pushed to the next line *and* converted to line comments.
    #[test]
    fn test_format_multine_block_comment_push_and_convert_to_line_comment() {
        assert_format!(
            "{
    let X = 1 /* some comment
    * over multiple lines yo       */
}",
            "{
    let X = 1
    // some comment
    // over multiple lines yo
}",
            |p| p.eat_block(),
            DystFormatOptions::default()
        );
    }
}
