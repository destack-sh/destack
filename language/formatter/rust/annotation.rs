use dyst_fir::format::{Format, FormatResult, hard_line_break};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

use crate::{
    Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Decorator, Doc, DocStyle,
    DystFormatContext, DystFormatter, FormatNode, Node, NodeId, NodeTree, NodeTreeImpl, NodeType,
    Tag,
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
    NodeTree: NodeTreeImpl<T>,
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
                Annotation::Decorator { position, .. } => (NodeType::Decorator, *position),
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
                let container = f
                    .context()
                    .get_ancestors(node_id)
                    .into_iter()
                    .find(|(_, node_type)| *node_type == NodeType::Definition);
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
            Annotation::Decorator { node, .. } => node.format(f),
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
        let string = f.context().strings.get(self.string);
        let is_multi_line = string.contains('\n');
        match self.style {
            DocStyle::Star => {
                if is_multi_line {
                    let total_lines = string.lines().count();
                    for (i, line) in string.lines().enumerate() {
                        if i == 0 {
                            write!(f, [token("/**")])?;
                        } else {
                            write!(f, [token(" *")])?;
                        }
                        if !line.is_empty() {
                            write!(f, [space(), text(line)])?;
                        } else if i == 0 {
                            write!(f, [space()])?;
                        }
                        if i != total_lines - 1 {
                            write!(f, [hard_line_break()])?;
                        }
                    }
                    if string.ends_with('\n') {
                        write!(f, [hard_line_break()])?;
                    }
                    write!(f, [token(" */")])?;
                } else {
                    write!(
                        f,
                        [token("/**"), space(), self.string, space(), token("*/")]
                    )?;
                }
            }
            DocStyle::Slash => {
                let mut lines = string.lines().peekable();
                while let Some(line) = lines.next() {
                    write!(f, [token("///"), space(), text(line)])?;
                    if lines.peek().is_some() {
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
        let string = f.context().strings.get(self.string);
        let is_multi_line = string.contains('\n');
        match self.style {
            CommentStyle::Star => {
                if is_multi_line {
                    for (i, line) in string.lines().enumerate() {
                        if i == 0 {
                            write!(f, [token("/*")])?;
                        } else {
                            write!(f, [token(" *")])?;
                        }
                        if !line.is_empty() {
                            write!(f, [space(), text(line)])?;
                        }
                        if i != string.lines().count() - 1 {
                            write!(f, [hard_line_break()])?;
                        }
                    }
                    if string.ends_with('\n') {
                        write!(f, [hard_line_break()])?;
                    }
                    write!(f, [token(" */")])?;
                } else {
                    write!(f, [token("/*"), space(), self.string, space(), token("*/")])?;
                }
            }
            CommentStyle::Slash => {
                let mut lines = string.lines().peekable();
                while let Some(line) = lines.next() {
                    write!(f, [token("//"), space(), text(line)])?;
                    if lines.peek().is_some() {
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
                        .join_with(&format_args![&token(","), soft_line_break_or_space()])
                        .entries(arguments)
                        .finish())),
                    token(")")
                ])]
            )?;
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Decorator> for Decorator {
    fn format_node(
        &self,
        _node_id: NodeId<Decorator>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [token("@"), self.receiver])?;
        if let Some(arguments) = &self.arguments
            && !arguments.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token("("),
                    soft_block_indent(&format_with(|f| f
                        .join_with(&format_args![&token(","), soft_line_break_or_space()])
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
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};
    use dyst_ast::DefinitionMeta;

    /// Block comments should retain all their newlines (including leading and trailing newlines).
    #[test]
    fn test_format_block_comment_retain_newlines() {
        let source = r#"{
    /*
     * Comment 1
     */
    let x

    /*
     * Comment 2.1
     * Comment 2.2
     * Comment 2.3
     */
    let y
}"#;
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DystFormatOptions::default()
        );
    }

    /// Tags should be preserved in order.
    #[test]
    fn test_format_tags_around_definition() {
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
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default()
        );
    }

    /// Decorators should be preserved in order with other annotations.
    #[test]
    fn test_format_decorators_on_struct() {
        let source = r#"{
    // comment before entity
    @entity
    // comment after entity
    // comment before foo
    @foo(1, 2, 3)
    // comment after foo
    struct Entity { }
}"#;
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DystFormatOptions::default()
        );
    }

    /// Multiple comments around an expression should retain their order.
    #[test]
    fn test_format_multiple_comments_around_expression() {
        let source = "{
    // comment part 1
    // comment part 2
    const A = 1
    // comment part 3
    // comment part 4
}";
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    /// Annotations should default to infix within source if no other position is found.
    #[test]
    fn test_format_annotations_fallback_to_infix() {
        assert_format!(
            "#A struct #B Test #C { #D } #E",
            "#A struct Test {\n\t#B\n\t#C\n\t#D\n} #E\n",
            |p| p.eat_struct(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    /// Multiple comments around an expression should retain their order across successive blocks.
    #[test]
    fn test_format_multiple_comments_around_expression_in_successive_blocks() {
        let source = "root: {
    // comment part 0
    a: {
        // comment part 1
        // comment part 2
        const A = 1
        // comment part 3
        // comment part 4
    }
    // comment part 5
    // comment part 6
    b: {
        // comment part 7
        // comment part 8
        const B = 2
        // comment part 9
        // comment part 10
    }
    // comment part 11
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DystFormatOptions::default()
        );
    }

    /// Inline expression comments should be preserved with proper spacing.
    #[test]
    fn test_format_inline_expression_comment() {
        assert_format!(
            "/* Pre-X comment */const X=/* Pre-A comment */A/* A comment */&&B/* B comment */",
            "/* Pre-X comment */ const X = /* Pre-A comment */ A /* A comment */ && B /* B comment */",
            |p| p.eat_expression(),
            DystFormatOptions::default_with_line_width(200)
        );
    }

    /// Keep multiline block doc comments as block comments.
    #[test]
    fn test_format_multine_block_doc_comment_stays_block() {
        assert_format!(
            "{
    /** some multiline
     * doc comment
     * over multiple lines */
    const X = 1 
}",
            "{
    /** some multiline
     * doc comment
     * over multiple lines */
    const X = 1
}",
            |p| p.eat_block(),
            DystFormatOptions::default()
        );
    }

    /// Keep multiline postfix comments as block comments.
    #[test]
    fn test_format_multine_block_comment_stays_block() {
        assert_format!(
            "{
    const X = 1 /* some comment
    * over multiple lines yo       */
}",
            "{
    const X = 1
    /* some comment
     * over multiple lines yo */
}",
            |p| p.eat_block(),
            DystFormatOptions::default()
        );
    }

    /// Excessive whitespace in line comments should be preserved.
    #[test]
    fn test_format_excessive_whitespace_in_line_comment() {
        let source = r"{
    // /// An Identity is globally unique identifier for an Entity.
    // struct Identity {
    //     /// The universally unique identifier of this Entity.
    //     id: Uuid
    // }
    const X = 1
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DystFormatOptions::default()
        );
    }
}
