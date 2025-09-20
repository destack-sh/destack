use dyst_language_fir::format::FormatResult;

use crate::{
    DystFormatContext, DystFormatter, FloatType, FormatNode, IntType, Mutability, NodeId,
    PrimitiveType, Type,
};
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Type> for Type {
    fn format_node(
        &self,
        _node_id: NodeId<Type>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Type::Infer => write!(f, [token("_")]),
            Type::Maybe(type_) => write!(f, [token("?"), type_]),
            Type::Not(type_) => write!(f, [token("!"), type_]),
            Type::Never => write!(f, [token("!")]),
            Type::This => write!(f, [token("Self")]),
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
                                        // only use comma separator if the group fits on a single line
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
                write!(
                    f,
                    [
                        if *mutability == Mutability::Mutable {
                            token("&var ")
                        } else {
                            token("&")
                        },
                        target
                    ]
                )
            }
            Type::Virtual(target) => {
                write!(f, [token("$"), target])
            }
            Type::Variadic(target) => {
                write!(f, [token(".."), target])
            }
            Type::Array { element, count } => {
                write!(f, [token("["), count, token("]"), element])
            }
            Type::Slice { element } => {
                write!(f, [token("[]"), element])
            }

            _ => todo!("format type"),
        }
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for PrimitiveType {
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        match self {
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
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_primitive_integer_type() {
        assert_format!(
            "int7",
            "int7",
            |p| p.eat_type(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_primitive_float_type() {
        assert_format!(
            "float32",
            "float32",
            |p| p.eat_type(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_primitive_void_type() {
        assert_format!(
            "void",
            "void",
            |p| p.eat_type(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_primitive_null_type() {
        assert_format!(
            "null",
            "null",
            |p| p.eat_type(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_primitive_maybe_reference_type() {
        assert_format!(
            "?&int32",
            "?&int32",
            |p| p.eat_type(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_primitive_never_type() {
        assert_format!("!", "!", |p| p.eat_type(), DystFormatOptions::default());
    }

    #[test]
    fn test_format_array_type() {
        assert_format!(
            "[7]int32",
            "[7]int32",
            |p| p.eat_type(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_slice_type() {
        assert_format!(
            "[]int32",
            "[]int32",
            |p| p.eat_type(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_path_type() {
        assert_format!(
            "geom.Vector",
            "geom.Vector",
            |p| p.eat_type(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_path_type_with_static_arguments() {
        assert_format!(
            "geom.Vector<Dims: 2, float32>",
            "geom.Vector<Dims: 2, float32>",
            |p| p.eat_type(),
            DystFormatOptions::default()
        );
    }
}
