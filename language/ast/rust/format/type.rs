use dyst_fir::format::FormatResult;

use crate::r#let::FormatScopedMutability;
use crate::{
    DystFormatContext, DystFormatter, FloatType, FormatNode, IntType, Mutability, NodeId,
    PrimitiveType, ScopedMutability, Type,
};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Type> for Type {
    fn format_node(
        &self,
        node_id: NodeId<Type>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Type::Infer => write!(f, [token("_")]),
            Type::Maybe(type_) => write!(f, [type_, token("?")]),
            Type::Not(type_) => write!(f, [token("!"), type_]),
            Type::Never => write!(f, [token("!")]),
            Type::Self_ => write!(f, [token("Self")]),
            Type::Primitive(primitive) => write!(f, [primitive]),
            Type::Path {
                path,
                static_arguments,
            } => {
                if let Some(arguments) = static_arguments {
                    write!(
                        f,
                        [
                            path,
                            group(&format_args![
                                token("<"),
                                soft_block_indent(&format_with(|f| f
                                    .join_with(&format_args![
                                        if_group_fits_on_line(&token(",")),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(arguments)
                                    .finish())),
                                token(">")
                            ])
                        ]
                    )
                } else {
                    write!(f, [path])
                }
            }
            Type::Reference { mutability, target } => {
                write!(f, [token("&")])?;
                write!(
                    f,
                    [FormatScopedMutability::implicit_const(mutability.clone()),]
                )?;
                match mutability {
                    ScopedMutability::Unscoped {
                        mutability: Mutability::Immutable,
                    } => {}
                    _ => write!(f, [space()])?,
                }
                write!(f, [target])
            }
            Type::Virtual(target) => {
                write!(f, [token("$"), target])
            }
            Type::Variadic(target) => {
                write!(f, [token(".."), target])
            }
            Type::Array { element, count } => {
                write!(f, [element, token("["), count, token("]")])
            }
            Type::Slice { element } => {
                write!(f, [element, token("[]")])
            }
            Type::Tuple(tuple) => write!(f, [tuple]),
            Type::InlineStruct(struct_) => write!(f, [struct_]),
            Type::InlineEnum(enum_) => write!(f, [enum_]),
            Type::InlineUnion(union) => write!(f, [union]),
            Type::Union(types) => write!(
                f,
                [format_with(|f| f
                    .join_with(&token(" | "))
                    .entries(types)
                    .finish())]
            ),
            Type::Intersection(types) => write!(
                f,
                [format_with(|f| f
                    .join_with(&token(" & "))
                    .entries(types)
                    .finish())]
            ),
            Type::Function(function) => write!(f, [function]),
        }?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for PrimitiveType {
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        match self {
            PrimitiveType::Undefined => write!(f, [token("undefined")]),
            PrimitiveType::Void => write!(f, [token("void")]),
            PrimitiveType::Null => write!(f, [token("null")]),
            PrimitiveType::Boolean => write!(f, [token("boolean")]),
            PrimitiveType::Character => write!(f, [token("char")]),
            PrimitiveType::Int(int_type) => write!(f, [int_type]),
            PrimitiveType::Float(float_type) => write!(f, [float_type]),
        }
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for IntType {
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        if self.is_signed {
            write!(f, [token("int"), text(&self.width.to_string())])
        } else {
            write!(f, [token("uint"), text(&self.width.to_string())])
        }
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for FloatType {
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        match self {
            FloatType::Float32 => write!(f, [token("float32")]),
            FloatType::Float64 => write!(f, [token("float64")]),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, TypeParserOptions, assert_format};

    #[test]
    fn test_format_primitive_integer_type() {
        assert_format!(
            "int7",
            "int7",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_primitive_float_type() {
        assert_format!(
            "float32",
            "float32",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_primitive_void_type() {
        assert_format!(
            "void",
            "void",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_primitive_null_type() {
        assert_format!(
            "null",
            "null",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_primitive_maybe_reference_type() {
        assert_format!(
            "?&int32",
            "&int32?",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_primitive_never_type() {
        assert_format!(
            "!",
            "!",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_array_type() {
        assert_format!(
            "[7]int32",
            "int32[7]",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_slice_type() {
        assert_format!(
            "[]int32",
            "int32[]",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_array_type_postfix_input() {
        assert_format!(
            "int32[7]",
            "int32[7]",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_slice_type_postfix_input() {
        assert_format!(
            "int32[]",
            "int32[]",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_path_type() {
        assert_format!(
            "geom.Vector",
            "geom.Vector",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_path_type_with_static_arguments() {
        assert_format!(
            "geom.Vector<Dims: 2, float32>",
            "geom.Vector<Dims: 2, float32>",
            |p| p.eat_type(TypeParserOptions::default()),
            DystFormatOptions::default()
        );
    }
}
