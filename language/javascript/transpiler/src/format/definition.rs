use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{
    Asynchrony, DeclarationKind, Definition, EnumField, ExportType, FunctionCardinality, Keyword,
    NodeId, Type, Visibility,
};

use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

use crate::format::argument::list_like;
use crate::{FormatNode, JavaScriptFormatContext, JavaScriptFormatter};

/// Format a super type clause.
pub(crate) fn format_super_type_clause<'ast>(
    f: &mut JavaScriptFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[NodeId<Type>],
) -> FormatResult<()> {
    assert!(!types.is_empty());

    write!(
        f,
        [
            space(),
            keyword,
            space(),
            group(&format_args![
                if_group_breaks(&token("(")),
                soft_block_indent(&format_with(|f| {
                    f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                        .entries(types)
                        .finish()?;
                    write!(f, [if_group_breaks(&token(","))])?;
                    Ok(())
                })),
                if_group_breaks(&token(")")),
            ]),
        ]
    )
}

/// Format a block of definitions.
pub(crate) fn format_block_of_definitions<'ast>(
    f: &mut JavaScriptFormatter<'ast, '_>,
    definitions: &Vec<NodeId<Definition>>,
) -> FormatResult<()> {
    f.join_with(hard_line_break()).entries(definitions).finish()
}

impl<'ast> Format<JavaScriptFormatContext<'ast>> for Visibility {
    fn format(&self, f: &mut JavaScriptFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Visibility::Public => write!(f, [Keyword::Public]),
            Visibility::Protected => write!(f, [Keyword::Protected]),
            Visibility::Private => write!(f, [Keyword::Private]),
        }
    }
}

impl<'ast> Format<JavaScriptFormatContext<'ast>> for ExportType {
    fn format(&self, f: &mut JavaScriptFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            ExportType::Item => write!(f, [Keyword::Export]),
            ExportType::Default => write!(f, [Keyword::Export, space(), Keyword::Default]),
            ExportType::Namespace => write!(f, [Keyword::Export]),
        }
    }
}

impl<'ast> FormatNode<'ast, Definition> for Definition {
    fn format_node(
        &self,
        _node_id: NodeId<Definition>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Definition::Namespace {
                descriptor,
                definitions,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Namespace])?;

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // body
                write!(f, [space()])?;
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
            }
            Definition::Class {
                descriptor,
                generics,
                heritage,
                properties,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Class])?;

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // static parameters
                if f.context().include_types()
                    && let Some(static_parameters) = generics.static_parameters.as_ref()
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // extends types
                if let Some(extends_types) = heritage.extends_types.as_ref()
                    && !extends_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Extends, extends_types)?;
                }

                // implements types
                if f.context().include_types()
                    && let Some(implements_types) = heritage.implements_types.as_ref()
                    && !implements_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Implements, implements_types)?;
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;
                write!(
                    f,
                    [block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(properties)
                        .finish()))]
                )?;
                write!(f, [hard_line_break(), token("}"),])?;
            }
            Definition::Interface {
                descriptor,
                generics,
                heritage,
                properties,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Interface])?;

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // static parameters
                if let Some(static_parameters) = generics.static_parameters.as_ref()
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // extends types
                if let Some(extends_types) = heritage.extends_types.as_ref()
                    && !extends_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Extends, extends_types)?;
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;
                write!(
                    f,
                    [block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(properties)
                        .finish()))]
                )?;
                write!(f, [hard_line_break(), token("}"),])?;
            }
            Definition::Enum { descriptor, fields } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Enum])?;

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;
                write!(
                    f,
                    [block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(fields)
                        .finish()))]
                )?;
                write!(f, [hard_line_break(), token("}"),])?;
            }
            Definition::Function {
                descriptor,
                signature,
                body,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // asynchrony
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // cardinality
                if signature.cardinality == FunctionCardinality::Generator {
                    write!(f, [token("*")])?;
                }

                // static parameters
                if f.context().include_types()
                    && let Some(static_parameters) = signature
                        .generics
                        .as_ref()
                        .and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // dynamic parameters
                write!(f, [list_like("(", ")", ",", &signature.dynamic_parameters)])?;

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [name])?;
                }

                // return type
                if f.context().include_types()
                    && let Some(return_type) = signature.return_type
                {
                    write!(f, [token(":"), space(), return_type])?;
                }

                // body
                if let Some(body) = body {
                    write!(f, [space(), body])?;
                }
            }
        }

        Ok(())
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
