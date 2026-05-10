use crate::{
    Keyword, LocalNodeId, MappedTypeModifier, PrimitiveType, TupleElement, TypeExpression,
    TypeLiteral, TypeMember, TypePredicateSubject,
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

impl<'ast> FormatNode<'ast, TypeMember> for TypeMember {
    fn format_node(
        &self,
        _node_id: LocalNodeId<TypeMember>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            TypeMember::Field { modifiers, key, ty } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [key])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // type
                write!(f, [token(":"), space(), ty])?;
            }
            TypeMember::Method {
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
                // generic parameters
                if !signature.generic_parameters.is_empty() {
                    format_type_parameter_list(&signature.generic_parameters, f)?;
                }
                // parameters
                format_function_signature_parameters(signature, f)?;
                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [space(), token("=>"), space(), return_type])?;
                }
            }
            TypeMember::CallSignature {
                modifiers,
                signature,
            } => {
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;

                if !signature.generic_parameters.is_empty() {
                    format_type_parameter_list(&signature.generic_parameters, f)?;
                }

                write!(f, [token("(")])?;

                list_like("", "", ",", &signature.parameters)
                    .include_space()
                    .format(f)?;

                write!(f, [token(")")])?;

                if let Some(return_type) = signature.return_type {
                    write!(f, [space(), token("=>"), space(), return_type])?;
                }
            }
            TypeMember::ConstructSignature {
                modifiers,
                signature,
            } => {
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                write!(f, [Keyword::New, space()])?;

                if !signature.generic_parameters.is_empty() {
                    format_type_parameter_list(&signature.generic_parameters, f)?;
                }

                write!(f, [token("(")])?;

                list_like("", "", ",", &signature.parameters)
                    .include_space()
                    .format(f)?;

                write!(f, [token(")")])?;

                if let Some(return_type) = signature.return_type {
                    write!(f, [space(), token("=>"), space(), return_type])?;
                }
            }
            TypeMember::IndexSignature {
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

impl<'ast> FormatNode<'ast, TypeExpression> for TypeExpression {
    fn format_node(
        &self,
        _node_id: LocalNodeId<TypeExpression>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            TypeExpression::Scalar(scalar) => {
                write!(f, [scalar])?;
            }
            TypeExpression::This => {
                write!(f, [Keyword::This])?;
            }
            TypeExpression::Path {
                path,
                generic_arguments,
            } => {
                write!(f, [path])?;
                if !generic_arguments.is_empty() {
                    write!(f, [list_like("<", ">", ",", generic_arguments)])?;
                }
            }
            TypeExpression::Readonly { target_type } => {
                write!(f, [Keyword::Readonly, space(), target_type])?;
            }
            TypeExpression::KeyOf { target_type } => {
                write!(f, [Keyword::Keyof, space(), target_type])?;
            }
            TypeExpression::Must { target_type } => {
                write!(f, [target_type, token("!")])?;
            }
            TypeExpression::AsComptime { target_type } => {
                write!(f, [target_type, space(), token("as comptime")])?;
            }
            TypeExpression::Not { target_type } => {
                write!(f, [token("!"), target_type])?;
            }
            TypeExpression::In { left, right } => {
                write!(f, [left, space(), Keyword::In, space(), right])?;
            }
            TypeExpression::Extends { left, right } => {
                write!(f, [left, space(), Keyword::Extends, space(), right])?;
            }
            TypeExpression::Implements { left, right } => {
                write!(f, [left, space(), Keyword::Implements, space(), right])?;
            }
            TypeExpression::Conditional {
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
            TypeExpression::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                write!(f, [token("{")])?;

                match modifiers.readonly {
                    MappedTypeModifier::Present => {
                        write!(f, [Keyword::Readonly, space()])?;
                    }
                    MappedTypeModifier::Add => {
                        write!(f, [token("+"), Keyword::Readonly, space()])?;
                    }
                    MappedTypeModifier::Remove => {
                        write!(f, [token("-"), Keyword::Readonly, space()])?;
                    }
                    MappedTypeModifier::None => {}
                }

                write!(
                    f,
                    [
                        token("["),
                        parameter.name,
                        space(),
                        Keyword::In,
                        space(),
                        parameter.source_type
                    ]
                )?;

                if let Some(key_remap) = parameter.key_remap {
                    write!(f, [space(), Keyword::As, space(), key_remap])?;
                }

                write!(f, [token("]")])?;

                match modifiers.optional {
                    MappedTypeModifier::Present => write!(f, [token("?")])?,
                    MappedTypeModifier::Add => write!(f, [token("+?")])?,
                    MappedTypeModifier::Remove => write!(f, [token("-?")])?,
                    MappedTypeModifier::None => {}
                }

                if let Some(value) = value {
                    write!(f, [token(":"), space(), *value])?;
                }

                write!(f, [token("}")])?;
            }
            TypeExpression::Index { left, index } => {
                write!(f, [left, token("["), index, token("]")])?;
            }
            TypeExpression::TemplateLiteral(template) => {
                write!(f, [token("`")])?;

                for (index, string) in template.strings.iter().enumerate() {
                    write!(f, [text(f.context().strings.get(*string))])?;

                    if let Some(span) = template.spans.get(index) {
                        write!(f, [token("${"), span, token("}")])?;
                    }
                }

                write!(f, [token("`")])?;
            }
            TypeExpression::Import {
                target,
                qualifier,
                generic_arguments,
            } => {
                write!(f, [Keyword::Import, token("(")])?;
                format_string_literal_with_source_span(*target, None, f)?;
                write!(f, [token(")")])?;

                if let Some(qualifier) = qualifier {
                    write!(f, [token("."), qualifier])?;
                }

                if !generic_arguments.is_empty() {
                    write!(f, [list_like("<", ">", ",", generic_arguments)])?;
                }
            }
            TypeExpression::Infer { name, constraint } => {
                write!(f, [Keyword::Infer, space(), *name])?;

                if let Some(constraint) = constraint {
                    write!(f, [space(), Keyword::Extends, space(), constraint])?;
                }
            }
            TypeExpression::Predicate {
                asserts,
                subject,
                target,
            } => {
                if *asserts {
                    write!(f, [Keyword::Asserts, space()])?;
                }

                match subject {
                    TypePredicateSubject::Identifier(name) => write!(f, [*name])?,
                    TypePredicateSubject::This => write!(f, [Keyword::This])?,
                }

                if let Some(target) = target {
                    write!(f, [space(), Keyword::Is, space(), target])?;
                }
            }

            TypeExpression::Array { element } => {
                write!(f, [element, token("[]")])?;
            }
            TypeExpression::Tuple { elements } => {
                write!(f, [list_like("[", "]", ",", elements)])?;
            }
            TypeExpression::Object { members } => {
                write!(f, [list_like("{", "}", ",", members).include_space()])?;
            }
            TypeExpression::Union { elements } => {
                write!(f, [list_like("|", "|", ",", elements)])?;
            }
            TypeExpression::Intersection { elements } => {
                write!(f, [list_like("&", "&", ",", elements)])?;
            }
            TypeExpression::FunctionTypeDeclaration(signature) => {
                // generic parameters
                if !signature.generic_parameters.is_empty() {
                    format_type_parameter_list(&signature.generic_parameters, f)?;
                }
                // parameters
                write!(f, [token("(")])?;
                list_like("", "", ",", &signature.parameters).format(f)?;
                write!(f, [token(")")])?;
                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [space(), token("=>"), space(), return_type])?;
                }
            }
            TypeExpression::ConstructorTypeDeclaration(signature) => {
                if signature.is_abstract {
                    write!(f, [Keyword::Abstract, space()])?;
                }

                write!(f, [Keyword::New, space()])?;

                if !signature.generic_parameters.is_empty() {
                    format_type_parameter_list(&signature.generic_parameters, f)?;
                }

                write!(f, [token("(")])?;
                list_like("", "", ",", &signature.parameters).format(f)?;
                write!(f, [token(")")])?;

                if let Some(return_type) = signature.return_type {
                    write!(f, [space(), token("=>"), space(), return_type])?;
                }
            }

            TypeExpression::Error => {
                write!(f, [token("/* ERROR */")])?;
            }
        }
        Ok(())
    }
}
