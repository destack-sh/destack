use crate::{
    FunctionMode, Keyword, LocalNodeId, PrimitiveType, TupleElement, Type, TypeField, TypeLiteral,
    TypeModifier, TypePredicateSubject,
};
use destack_fir::format::FormatResult;

use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::argument::{format_type_parameter_list, list_like};
use crate::format::function::format_function_signature_parameters;
use crate::format::literal::format_string_literal_with_source_span;
use crate::format::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::{FormatNode, JsFormatContext, JsFormatter};

impl<'ast> Format<JsFormatContext<'ast>> for PrimitiveType {
    fn format(&self, f: &mut JsFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            PrimitiveType::Boolean => write!(f, [token("boolean")]),
            PrimitiveType::String => write!(f, [token("string")]),
            PrimitiveType::Bigint => write!(f, [token("bigint")]),
            PrimitiveType::Number => write!(f, [token("number")]),
            PrimitiveType::Symbol => write!(f, [token("symbol")]),
            PrimitiveType::UniqueSymbol => write!(f, [token("unique symbol")]),
        }
    }
}

impl<'ast> Format<JsFormatContext<'ast>> for TypeLiteral {
    fn format(&self, f: &mut JsFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            TypeLiteral::Never => write!(f, [token("never")]),
            TypeLiteral::Any => write!(f, [token("any")]),
            TypeLiteral::Undefined => write!(f, [token("undefined")]),
            TypeLiteral::Unknown => write!(f, [token("unknown")]),
            TypeLiteral::Object => write!(f, [token("object")]),
            TypeLiteral::Void => write!(f, [token("void")]),
            TypeLiteral::Null => write!(f, [token("null")]),
            TypeLiteral::Primitive(primitive) => write!(f, [primitive]),
            TypeLiteral::ScalarLiteral(scalar_literal) => write!(f, [scalar_literal]),
        }
    }
}

impl<'ast> FormatNode<'ast, TypeField> for TypeField {
    fn format_node(
        &self,
        _node_id: LocalNodeId<TypeField>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            TypeField::Field { modifiers, key, ty } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [key])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // type
                write!(f, [token(":"), space(), ty])?;
            }
            TypeField::Method {
                modifiers,
                key,
                signature,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [key])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // mode
                if let Some(mode) = signature.mode
                    && mode == FunctionMode::New
                {
                    write!(f, [Keyword::New, space()])?;
                }
                // static parameters
                if let Some(static_parameters) = signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    format_type_parameter_list(static_parameters, f)?;
                }
                // parameters
                format_function_signature_parameters(signature, f)?;
                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [space(), token("=>"), space(), return_type])?;
                }
            }
            TypeField::IndexSignature {
                modifiers,
                name,
                key_type,
                value_type,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(
                    f,
                    [token("["), *name, token(":"), space(), key_type, token("]")]
                )?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                write!(f, [token(":"), space(), value_type])?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, TupleElement> for TupleElement {
    fn format_node(
        &self,
        _node_id: LocalNodeId<TupleElement>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if self.is_readonly {
            write!(f, [Keyword::Readonly, space()])?;
        }

        if self.is_rest {
            write!(f, [token("...")])?;
        }

        if let Some(label) = self.label {
            write!(f, [label])?;

            if self.is_optional {
                write!(f, [token("?")])?;
            }

            write!(f, [token(":"), space()])?;
        }

        write!(f, [self.ty])?;

        if self.label.is_none() && self.is_optional {
            write!(f, [token("?")])?;
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Type> for Type {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Type>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Type::Scalar(scalar) => {
                write!(f, [scalar])?;
            }
            Type::This => {
                write!(f, [Keyword::This])?;
            }
            Type::Path {
                path,
                static_arguments,
            } => {
                write!(f, [path])?;
                if let Some(static_arguments) = static_arguments {
                    write!(f, [list_like("<", ">", ",", static_arguments)])?;
                }
            }
            Type::Expression(expression) => {
                write!(f, [expression])?;
            }
            Type::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                write!(
                    f,
                    [
                        left,
                        space(),
                        Keyword::Extends,
                        space(),
                        right,
                        space(),
                        token("?"),
                        space(),
                        then_type,
                        space(),
                        token(":"),
                        space(),
                        else_type
                    ]
                )?;
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                write!(f, [token("{")])?;

                match modifiers.readonly {
                    TypeModifier::Present => {
                        write!(f, [Keyword::Readonly, space()])?;
                    }
                    TypeModifier::Add => {
                        write!(f, [token("+"), Keyword::Readonly, space()])?;
                    }
                    TypeModifier::Remove => {
                        write!(f, [token("-"), Keyword::Readonly, space()])?;
                    }
                    TypeModifier::None => {}
                }

                write!(
                    f,
                    [
                        token("["),
                        parameter.name,
                        space(),
                        Keyword::In,
                        space(),
                        parameter.constraint
                    ]
                )?;

                if let Some(key_remap) = parameter.key_remap {
                    write!(f, [space(), Keyword::As, space(), key_remap])?;
                }

                write!(f, [token("]")])?;

                match modifiers.optional {
                    TypeModifier::Present => write!(f, [token("?")])?,
                    TypeModifier::Add => write!(f, [token("+?")])?,
                    TypeModifier::Remove => write!(f, [token("-?")])?,
                    TypeModifier::None => {}
                }

                write!(f, [token(":"), space(), value, token("}")])?;
            }
            Type::Index { left, index } => {
                write!(f, [left, token("["), index, token("]")])?;
            }
            Type::TemplateLiteral(template) => {
                write!(f, [token("`")])?;

                for (index, string) in template.strings.iter().enumerate() {
                    write!(f, [text(f.context().strings.get(*string))])?;

                    if let Some(span) = template.spans.get(index) {
                        write!(f, [token("${"), span, token("}")])?;
                    }
                }

                write!(f, [token("`")])?;
            }
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                write!(f, [Keyword::Import, token("(")])?;
                format_string_literal_with_source_span(*target, None, f)?;
                write!(f, [token(")")])?;

                if let Some(qualifier) = qualifier {
                    write!(f, [token("."), qualifier])?;
                }

                if let Some(static_arguments) = static_arguments {
                    write!(f, [list_like("<", ">", ",", static_arguments)])?;
                }
            }
            Type::Infer { name, constraint } => {
                write!(f, [Keyword::Infer, space(), *name])?;

                if let Some(constraint) = constraint {
                    write!(f, [space(), Keyword::Extends, space(), constraint])?;
                }
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                if *asserts {
                    write!(f, [Keyword::Asserts, space()])?;
                }

                match subject {
                    TypePredicateSubject::Name(name) => write!(f, [*name])?,
                    TypePredicateSubject::This => write!(f, [Keyword::This])?,
                }

                if let Some(target) = target {
                    write!(f, [space(), Keyword::Is, space(), target])?;
                }
            }

            Type::Unary { operator, right } => {
                write!(f, [operator, space(), right])?;
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                write!(f, [left, space(), operator, space(), right])?;
            }

            Type::Array { element } => {
                if let Some(element) = element {
                    write!(f, [element, token("[]")])?;
                } else {
                    write!(f, [token("Array<any>")])?;
                }
            }
            Type::Tuple { elements } => {
                write!(f, [list_like("[", "]", ",", elements)])?;
            }
            Type::Object { properties } => {
                write!(f, [list_like("{", "}", ",", properties).include_space()])?;
            }
            Type::Union { elements } => {
                write!(f, [list_like("|", "|", ",", elements)])?;
            }
            Type::Intersection { elements } => {
                write!(f, [list_like("&", "&", ",", elements)])?;
            }
            Type::Function { signature } => {
                // mode
                if let Some(mode) = signature.mode
                    && mode == FunctionMode::New
                {
                    write!(f, [Keyword::New, space()])?;
                }
                // static parameters
                if let Some(static_parameters) = signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    format_type_parameter_list(static_parameters, f)?;
                }
                // parameters
                format_function_signature_parameters(signature, f)?;
                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [space(), token("=>"), space(), return_type])?;
                }
            }

            Type::Error => {
                write!(f, [token("/* ERROR */")])?;
            }
        }
        Ok(())
    }
}
