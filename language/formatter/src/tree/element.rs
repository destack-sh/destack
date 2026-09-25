use crate::annotation::FormatTrailingComments;
use crate::context::with_following_span_start;
use crate::expression::format_generic_argument_list;
use crate::tree::write_tree_attribute;
use crate::{TsppFormatContext, TsppFormatter};
use tspp_dir::{
    Expression, GenericArgument, LocalNodeId, TreeAttribute, TreeAttributeValue, TreeChild,
};
use tspp_fir::format::{Format, FormatResult};
use tspp_fir::prelude::{
    expand_parent, format_with, group, hard_line_break, soft_line_break, soft_line_break_or_space,
    soft_line_indent_or_space, space, token,
};
use tspp_fir::write;
use tspp_source::{NodeSpanRegion, NodeSpanType};

/// Formatter for one tree opening element.
pub(crate) struct FormatTreeOpeningElement<'tree> {
    /// The tree expression id.
    pub expression_id: LocalNodeId<Expression>,
    /// The optional tag name expression.
    pub left: &'tree Option<LocalNodeId<Expression>>,
    /// The generic arguments on the tag name.
    pub generic_arguments: &'tree [LocalNodeId<GenericArgument>],
    /// The opening tag attributes.
    pub attributes: &'tree Option<Vec<LocalNodeId<TreeAttribute>>>,
    /// The tree children.
    pub children: &'tree Option<Vec<LocalNodeId<TreeChild>>>,
    /// Whether attributes force the opening element to break.
    pub force_break_attributes: bool,
}

impl<'tree> FormatTreeOpeningElement<'tree> {
    /// Create a formatter for one tree opening element.
    pub(crate) fn new(
        expression_id: LocalNodeId<Expression>,
        left: &'tree Option<LocalNodeId<Expression>>,
        generic_arguments: &'tree [LocalNodeId<GenericArgument>],
        attributes: &'tree Option<Vec<LocalNodeId<TreeAttribute>>>,
        children: &'tree Option<Vec<LocalNodeId<TreeChild>>>,
        force_break_attributes: bool,
    ) -> Self {
        Self {
            expression_id,
            left,
            generic_arguments,
            attributes,
            children,
            force_break_attributes,
        }
    }

    /// Return whether this opening element is self-closing.
    fn is_self_closing(&self) -> bool {
        self.children.is_none()
    }

    /// Compute the opening element layout.
    fn compute_layout(&self, context: &TsppFormatContext<'_>) -> TreeOpeningElementLayout {
        let attributes = self.attributes.as_deref().unwrap_or(&[]);
        let comments = context.comments();
        let opening_span = context
            .tree
            .get_side_span(
                self.expression_id,
                NodeSpanType::Region(NodeSpanRegion::Opening),
            )
            .unwrap_or_else(|| context.span(self.expression_id));

        let last_attribute_has_comment = attributes.last().is_some_and(|attribute_id| {
            comments.has_comment_in_range(context.span(*attribute_id).end, opening_span.end)
        });

        let type_arguments_or_name_end = self
            .generic_arguments
            .last()
            .map(|argument_id| context.span(*argument_id).end)
            .or_else(|| self.left.map(|left_id| context.span(left_id).end))
            .unwrap_or(opening_span.start);
        let first_attribute_start_or_element_end =
            attributes.first().map_or(opening_span.end, |attribute_id| {
                context.span(*attribute_id).start
            });
        let name_has_comment = comments.has_comment_in_range(
            type_arguments_or_name_end,
            first_attribute_start_or_element_end,
        );

        if self.is_self_closing() && attributes.is_empty() && !name_has_comment {
            TreeOpeningElementLayout::Inline
        } else if attributes.len() == 1
            && !name_has_comment
            && !last_attribute_has_comment
            && is_single_line_string_literal_attribute(context, attributes[0])
        {
            TreeOpeningElementLayout::SingleStringAttribute
        } else {
            TreeOpeningElementLayout::IndentAttributes {
                name_has_comment,
                last_attribute_has_comment,
            }
        }
    }
}

impl<'ast> Format<'ast, TsppFormatContext<'ast>> for FormatTreeOpeningElement<'_> {
    fn format(&self, f: &mut TsppFormatter<'ast, '_>) -> FormatResult<()> {
        let layout = self.compute_layout(f.context());
        let format_open = format_with(|f: &mut TsppFormatter<'ast, '_>| {
            write!(f, [token("<")])?;
            if let Some(left) = self.left {
                let attributes = self.attributes.as_deref().unwrap_or(&[]);
                let left_following_span_start = self
                    .generic_arguments
                    .first()
                    .map(|argument_id| f.context().span(*argument_id).start)
                    .or_else(|| {
                        attributes
                            .first()
                            .map(|argument_id| f.context().span(*argument_id).start)
                    })
                    .or(f.context().following_span_start());
                with_following_span_start(f, left_following_span_start, |f| write!(f, [left]))?;
            }
            if !self.generic_arguments.is_empty() {
                format_generic_argument_list(f, self.generic_arguments)?;
            }

            Ok(())
        });
        let format_close = format_with(|f| {
            if self.is_self_closing() {
                write!(f, [token("/")])?;
            }

            write!(f, [token(">")])
        });

        match layout {
            TreeOpeningElementLayout::Inline => {
                write!(f, [format_open, space(), format_close])
            }
            TreeOpeningElementLayout::SingleStringAttribute => {
                let attribute_spacing = self.is_self_closing().then_some(space());
                let attributes = self.attributes.as_deref().unwrap_or(&[]);
                write!(
                    f,
                    [
                        format_open,
                        space(),
                        format_with(|f| write_tree_attribute(f, attributes[0], None)),
                        attribute_spacing,
                        format_close
                    ]
                )
            }
            TreeOpeningElementLayout::IndentAttributes {
                name_has_comment,
                last_attribute_has_comment,
            } => {
                let opening_span = f
                    .context()
                    .tree
                    .get_side_span(
                        self.expression_id,
                        NodeSpanType::Region(NodeSpanRegion::Opening),
                    )
                    .unwrap_or_else(|| f.context().span(self.expression_id));
                let attributes = self.attributes.as_deref().unwrap_or(&[]);
                let format_inner = format_with(|f| {
                    write!(f, [format_open])?;

                    if !attributes.is_empty() {
                        format_tree_attributes(f, attributes, self.force_break_attributes)?;
                    }

                    let comments = f.context().comments().comments_before(opening_span.end);
                    write!(f, [FormatTrailingComments::Comments(comments)])?;

                    let force_bracket_same_line = f.context().options.bracket_same_line;
                    let wants_bracket_same_line = attributes.is_empty() && !name_has_comment;
                    if self.is_self_closing() {
                        write!(f, [soft_line_break_or_space(), format_close])
                    } else if last_attribute_has_comment {
                        write!(f, [soft_line_break(), format_close])
                    } else if (force_bracket_same_line && !attributes.is_empty())
                        || wants_bracket_same_line
                    {
                        write!(f, [format_close])
                    } else {
                        write!(f, [soft_line_break(), format_close])
                    }
                });

                let has_multiline_string_attribute =
                    attributes.iter().copied().any(|attribute_id| {
                        is_multiline_string_literal_attribute(f.context(), attribute_id)
                    });
                write!(
                    f,
                    [group(&format_inner).should_expand(has_multiline_string_attribute)]
                )
            }
        }
    }
}

/// Opening element layout.
#[derive(Copy, Clone, Debug)]
enum TreeOpeningElementLayout {
    /// Do not create a group around the element to avoid it breaking.
    Inline,
    /// Opening element with one string literal attribute and no comments.
    SingleStringAttribute,
    /// Default layout that indents attributes.
    IndentAttributes {
        /// Whether the tag name has a following comment.
        name_has_comment: bool,
        /// Whether the last attribute has a following comment.
        last_attribute_has_comment: bool,
    },
}

/// Return whether one attribute has a multiline string literal value.
fn is_multiline_string_literal_attribute(
    context: &TsppFormatContext<'_>,
    attribute_id: LocalNodeId<TreeAttribute>,
) -> bool {
    as_string_literal_attribute_value(context, attribute_id)
        .is_some_and(|string_id| context.strings.get(string_id).contains('\n'))
}

/// Return whether one attribute has a single-line string literal value.
fn is_single_line_string_literal_attribute(
    context: &TsppFormatContext<'_>,
    attribute_id: LocalNodeId<TreeAttribute>,
) -> bool {
    as_string_literal_attribute_value(context, attribute_id)
        .is_some_and(|string_id| !context.strings.get(string_id).contains('\n'))
}

/// Return a string literal attribute value.
fn as_string_literal_attribute_value(
    context: &TsppFormatContext<'_>,
    attribute_id: LocalNodeId<TreeAttribute>,
) -> Option<tspp_core::StringId> {
    let TreeAttribute::Named {
        value: Some(TreeAttributeValue::String(string_id)),
        ..
    } = context.tree.get(attribute_id)
    else {
        return None;
    };

    Some(*string_id)
}

/// Format tree attributes in one opening tag.
fn format_tree_attributes<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    attributes: &[LocalNodeId<TreeAttribute>],
    force_break_attributes: bool,
) -> FormatResult<()> {
    let single_attribute_per_line = f.context().options.single_attribute_per_line;
    let attr_separator: &dyn Format<'ast, TsppFormatContext<'ast>> =
        if force_break_attributes || (single_attribute_per_line && attributes.len() > 1) {
            &hard_line_break()
        } else {
            &soft_line_break_or_space()
        };
    let format_attrs = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        let following_span_starts = attributes
            .iter()
            .enumerate()
            .map(|(index, _)| {
                attributes
                    .get(index + 1)
                    .map(|attribute| f.context().span(*attribute).start)
            })
            .collect::<Vec<_>>();

        f.join_with(attr_separator)
            .entries(attributes.iter().enumerate().map(|(index, attribute)| {
                let attribute_id = *attribute;
                let following_span_start = following_span_starts[index];

                format_with(move |f| write_tree_attribute(f, attribute_id, following_span_start))
            }))
            .finish()
    });

    if force_break_attributes {
        write!(f, [expand_parent()])?;
    }

    write!(f, [soft_line_indent_or_space(&format_attrs)])?;

    Ok(())
}
