use crate::{
    Asynchrony, Declaration, DependencyBinding, EnumField, InterfaceHeritage, Keyword, LocalNodeId,
    TypeExpression, Visibility,
};
use destack_fir::format::FormatResult;

use destack_fir::prelude::*;
use destack_fir::{format_args, write};

use crate::format::argument::{format_type_parameter_list, list_like};
use crate::format::block::format_block_of_statements;
use crate::format::function::format_function_signature_parameters;
use crate::{FormatNode, JsFormatContext, JsFormatter};

/// Format a super type clause.
pub(crate) fn format_super_type_clause<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<TypeExpression>],
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

/// Format an interface heritage clause.
pub(crate) fn format_interface_heritage_clause<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    heritage_items: &[InterfaceHeritage],
) -> FormatResult<()> {
    assert!(!heritage_items.is_empty());

    write!(f, [space(), Keyword::Extends, space()])?;
    write!(
        f,
        [group(&format_args![
            if_group_breaks(&token("(")),
            soft_block_indent(&format_with(|f| {
                for (index, heritage) in heritage_items.iter().enumerate() {
                    if index > 0 {
                        write!(f, [token(","), soft_line_break_or_space()])?;
                    }

                    write!(f, [heritage.expression])?;

                    if !heritage.type_arguments.is_empty() {
                        write!(f, [list_like("<", ">", ",", &heritage.type_arguments)])?;
                    }
                }

                write!(f, [if_group_breaks(&token(","))])?;
                Ok(())
            })),
            if_group_breaks(&token(")")),
        ])]
    )
}

impl<'ast> Format<JsFormatContext<'ast>> for Visibility {
    fn format(&self, f: &mut JsFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Visibility::Public => write!(f, [Keyword::Public]),
            Visibility::Protected => write!(f, [Keyword::Protected]),
            Visibility::Private => write!(f, [Keyword::Private]),
        }
    }
}

impl<'ast> Format<JsFormatContext<'ast>> for DependencyBinding {
    fn format(&self, f: &mut JsFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            DependencyBinding::Named => write!(f, [Keyword::Export]),
            DependencyBinding::Default => write!(f, [Keyword::Export, space(), Keyword::Default]),
            DependencyBinding::Namespace => write!(f, [Keyword::Export]),
        }
    }
}

impl<'ast> FormatNode<'ast, Declaration> for Declaration {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Declaration>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if !f.context().include_types() && self.is_type_only() {
            return Ok(());
        }

        match self {
            Declaration::Global(global) => {
                let statements = &global.statements;

                // ambient
                if global.is_ambient {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Global])?;

                // body
                write!(f, [space()])?;
                write!(
                    f,
                    [
                        token("{"),
                        hard_line_break(),
                        block_indent(&format_with(|f| format_block_of_statements(f, statements))),
                        hard_line_break(),
                        token("}"),
                    ]
                )?;
            }
            Declaration::Namespace(namespace) => {
                let statements = &namespace.statements;

                // export
                if let Some(export) = namespace.export {
                    write!(f, [export, space()])?;
                }

                // ambient
                if namespace.is_ambient {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Namespace])?;

                // name / key
                if let Some(name) = namespace.name {
                    write!(f, [space(), name])?;
                }

                // body
                write!(f, [space()])?;
                write!(
                    f,
                    [
                        token("{"),
                        hard_line_break(),
                        block_indent(&format_with(|f| format_block_of_statements(f, statements))),
                        hard_line_break(),
                        token("}"),
                    ]
                )?;
            }
            Declaration::Type(ty) => {
                let generic_parameters = &ty.generic_parameters;
                let value = ty.value;

                // export
                if let Some(export) = ty.export {
                    write!(f, [export, space()])?;
                }

                // keyword
                write!(f, [Keyword::Type])?;

                // name / key
                if let Some(name) = ty.name {
                    write!(f, [space(), name])?;
                }

                // generic parameters
                if !generic_parameters.is_empty() {
                    format_type_parameter_list(generic_parameters, f)?;
                }

                // value
                write!(f, [space(), token("="), space(), value])?;
            }
            Declaration::Class(class) => {
                let generic_parameters = &class.generic_parameters;
                let extends_expression = class.extends_expression;
                let extends_generic_arguments = &class.extends_generic_arguments;
                let implements_types = &class.implements_types;
                let members = &class.members;

                // export
                if let Some(export) = class.export {
                    write!(f, [export, space()])?;
                }

                // ambient
                if class.is_ambient {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // abstract
                if class.is_abstract {
                    write!(f, [Keyword::Abstract, space()])?;
                }

                // keyword
                write!(f, [Keyword::Class])?;

                // name / key
                if let Some(name) = class.name {
                    write!(f, [space(), name])?;
                }

                // generic parameters
                if f.context().include_types() && !generic_parameters.is_empty() {
                    format_type_parameter_list(generic_parameters, f)?;
                }

                // extends expression
                if let Some(extends_expression) = extends_expression {
                    write!(f, [space(), Keyword::Extends, space(), extends_expression])?;

                    if f.context().include_types() && !extends_generic_arguments.is_empty() {
                        write!(f, [list_like("<", ">", ",", extends_generic_arguments)])?;
                    }
                }

                // implements types
                if f.context().include_types() && !implements_types.is_empty() {
                    format_super_type_clause(f, Keyword::Implements, implements_types)?;
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;
                write!(
                    f,
                    [block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(members)
                        .finish()))]
                )?;
                write!(f, [hard_line_break(), token("}"),])?;
            }
            Declaration::Interface(interface) => {
                let generic_parameters = &interface.generic_parameters;
                let extends = &interface.extends;
                let members = &interface.members;

                // export
                if let Some(export) = interface.export {
                    write!(f, [export, space()])?;
                }

                // ambient
                if interface.is_ambient {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Interface])?;

                // name / key
                if let Some(name) = interface.name {
                    write!(f, [space(), name])?;
                }

                // generic parameters
                if !generic_parameters.is_empty() {
                    format_type_parameter_list(generic_parameters, f)?;
                }

                // extends
                if !extends.is_empty() {
                    format_interface_heritage_clause(f, extends)?;
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;
                write!(
                    f,
                    [block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(members)
                        .finish()))]
                )?;
                write!(f, [hard_line_break(), token("}"),])?;
            }
            Declaration::Enum(enum_declaration) => {
                let fields = &enum_declaration.fields;

                // export
                if let Some(export) = enum_declaration.export {
                    write!(f, [export, space()])?;
                }

                // ambient
                if enum_declaration.is_ambient {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Enum])?;

                // name / key
                if let Some(name) = enum_declaration.name {
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
            Declaration::Function(function) => {
                let signature = &function.signature;
                let body = function.body;

                // export
                if let Some(export) = function.export {
                    write!(f, [export, space()])?;
                }

                // ambient
                if function.is_ambient {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // abstract
                if function.is_abstract {
                    write!(f, [Keyword::Abstract, space()])?;
                }

                // asynchrony
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // keyword
                write!(f, [Keyword::Function])?;

                // generator
                if signature.is_generator {
                    write!(f, [token("*")])?;
                }

                // name / key
                if let Some(name) = function.name {
                    write!(f, [space(), name])?;
                }

                // generic parameters
                if f.context().include_types() && !signature.generic_parameters.is_empty() {
                    format_type_parameter_list(&signature.generic_parameters, f)?;
                }

                // parameters
                format_function_signature_parameters(signature, f)?;

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
        _node_id: LocalNodeId<EnumField>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [self.name])?;
        if let Some(value) = self.value {
            write!(f, [space(), token("="), space(), value])?;
        }
        Ok(())
    }
}
