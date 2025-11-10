use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{Definition, EnumField, Keyword, NodeId};

use dyst_fir::prelude::*;
use dyst_fir::write;

use crate::format::argument::list_like;
use crate::{FormatNode, JavaScriptFormatter};

/// Format a block of definitions.
pub(crate) fn format_block_of_definitions<'ast>(
    f: &mut JavaScriptFormatter<'ast, '_>,
    definitions: &Vec<NodeId<Definition>>,
) -> FormatResult<()> {
    f.join_with(hard_line_break()).entries(definitions).finish()
}

impl<'ast> FormatNode<'ast, Definition> for Definition {
    fn format_node(
        &self,
        node_id: NodeId<Definition>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Definition::Namespace { meta, definitions } => {
                assert!(
                    f.context().include_types(),
                    "namespace in non-type context: {node_id:?}"
                );
                write!(f, [Keyword::Namespace, space(), meta.name, space()])?;
                write!(
                    f,
                    [
                        token("{"),
                        hard_line_break(),
                        block_indent(&format_with(|f| format_block_of_definitions(
                            f,
                            definitions
                        ))),
                        hard_line_break(),
                        token("}"),
                    ]
                )?;
                Ok(())
            }
            Definition::Class {
                meta,
                static_parameters,
                fields,
                definitions,
            } => {
                write!(f, [Keyword::Class, space(), meta.name, space()])?;
                if f.context().include_types()
                    && let Some(static_parameters) = static_parameters
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }
                write!(f, [token("{"), hard_line_break()])?;
                write!(
                    f,
                    [block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(fields)
                        .finish()))]
                )?;
                write!(
                    f,
                    [block_indent(&format_with(|f| format_block_of_definitions(
                        f,
                        definitions
                    )))]
                )?;
                write!(f, [hard_line_break(), token("}"),])?;
                Ok(())
            }
            Definition::Interface {
                meta,
                static_parameters,
                fields,
                definitions,
            } => {
                assert!(
                    f.context().include_types(),
                    "interface in non-type context: {node_id:?}"
                );
                write!(f, [Keyword::Interface, space(), meta.name, space()])?;
                if let Some(static_parameters) = static_parameters {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }
                write!(f, [token("{")])?;
                write!(
                    f,
                    [block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(fields)
                        .finish()))]
                )?;
                write!(
                    f,
                    [block_indent(&format_with(|f| format_block_of_definitions(
                        f,
                        definitions
                    )))]
                )?;
                write!(f, [hard_line_break(), token("}"),])?;
                Ok(())
            }
            Definition::Enum { meta, fields } => {
                assert!(
                    f.context().include_types(),
                    "enum in non-type context: {node_id:?}"
                );
                write!(f, [Keyword::Enum, space(), meta.name, space()])?;
                write!(f, [token("{")])?;
                write!(
                    f,
                    [block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(fields)
                        .finish()))]
                )?;
                write!(f, [hard_line_break(), token("}"),])?;
                Ok(())
            }
            _ => unimplemented!("format_node: {self:?}"),
        }
    }
}

impl<'ast> FormatNode<'ast, EnumField> for EnumField {
    fn format_node(
        &self,
        _node_id: NodeId<EnumField>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [self.name])?;
        if let Some(value) = self.value {
            write!(f, [space(), token("="), space(), value])?;
        }
        Ok(())
    }
}
