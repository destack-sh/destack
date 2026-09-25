use tspp_core::float_from_bits;
use tspp_fir::format::{Format, FormatError, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;

use super::r#type::{format_generic_arguments, format_parameter, format_type_expanded};

use crate::{
    BlockId, Constant, Formatter, FunctionId, GlobalId, Place, PlaceOrigin, Projection, Type,
    TypeId, Value, Writer,
};

impl<'a> Format<'a, Formatter<'a>> for Place {
    fn format(&self, f: &mut Writer<'a, '_>) -> FormatResult<()> {
        format_place(self.origin, &self.path.projections, f)
    }
}

/// Format a root and its ordered projections.
fn format_place<'a>(
    origin: PlaceOrigin,
    projections: &[Projection],
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    let Some((projection, prefix)) = projections.split_last() else {
        return match origin {
            PlaceOrigin::Local(local) => {
                let index = f.context().local_index(local)?;

                write!(f, [copied_text(&format!("l{index}"))])
            }
            PlaceOrigin::Global(global) => {
                write!(f, [token("@")])?;
                format_global_id(global, f)
            }
            PlaceOrigin::Value(value) => write!(f, [value]),
        };
    };

    match projection {
        Projection::Deref => {
            write!(f, [token("(*")])?;
            format_place(origin, prefix, f)?;
            write!(f, [token(")")])
        }
        Projection::Field { index } => {
            // reuse parentheses supplied by a dereference or case selection
            let is_grouped = matches!(
                prefix.last(),
                Some(Projection::Deref | Projection::Variant { .. })
            );
            if !is_grouped {
                write!(f, [token("(")])?;
            }
            format_place(origin, prefix, f)?;
            if !is_grouped {
                write!(f, [token(")")])?;
            }

            write!(f, [token("."), copied_text(&index.to_string())])
        }
        Projection::Variant { case } => {
            write!(f, [token("(")])?;
            format_place(origin, prefix, f)?;
            write!(
                f,
                [
                    space(),
                    token("as"),
                    space(),
                    copied_text(&case.to_string()),
                    token(")")
                ]
            )
        }
        Projection::Element { index } => {
            format_place(origin, prefix, f)?;
            write!(f, [token("["), copied_text(&index.to_string()), token("]")])
        }
        Projection::Index { index } => {
            format_place(origin, prefix, f)?;
            write!(f, [token("["), index, token("]")])
        }
        Projection::Slice { start, length } => {
            format_place(origin, prefix, f)?;
            write!(
                f,
                [token("["), start, token(";"), space(), length, token("]")]
            )
        }
        Projection::Elements => Err(FormatError::SyntaxError {
            message: "a memory operand must select a concrete element or slice",
        }),
    }
}

impl<'a> Format<'a, Formatter<'a>> for Value {
    fn format(&self, f: &mut Writer<'a, '_>) -> FormatResult<()> {
        write!(f, [copied_text(&format!("v{}", self.0))])
    }
}

impl<'a> Format<'a, Formatter<'a>> for Constant {
    fn format(&self, f: &mut Writer<'a, '_>) -> FormatResult<()> {
        match self {
            Constant::Parameter(index) => format_parameter(*index, f),
            Constant::Null => write!(f, [token("null")]),
            Constant::Undefined => write!(f, [token("undefined")]),
            Constant::Layout { ty, measure } => {
                write!(f, [token(measure.keyword()), space(), *ty])
            }
            Constant::Witness {
                receiver,
                interface,
                member,
            } => {
                let member = f.context().strings.get(*member).to_string();
                write!(
                    f,
                    [
                        token("witness"),
                        space(),
                        *receiver,
                        token(","),
                        space(),
                        *interface,
                        token(","),
                        space(),
                        copied_text(&member)
                    ]
                )
            }
            Constant::Uninit => write!(f, [token("uninit")]),
            Constant::Zeroed => write!(f, [token("zeroed")]),
            Constant::Boolean { value } => {
                write!(f, [token(if *value { "true" } else { "false" })])
            }
            Constant::Int {
                value,
                width,
                is_signed,
            } => {
                let suffix = if *is_signed {
                    format!("int{width}")
                } else {
                    format!("uint{width}")
                };
                write!(f, [copied_text(&format!("{value}{suffix}"))])
            }
            Constant::UInt { value, width } => {
                write!(f, [copied_text(&format!("{value}uint{width}"))])
            }
            Constant::Float { bits, format } => {
                let value = float_from_bits(format.format(), *bits);
                let value_str = format!("{value}{}", format.label());
                write!(f, [copied_text(&value_str)])
            }
            Constant::Char { value } => {
                write!(f, [copied_text(&format!("{value:?}"))])
            }
        }
    }
}

/// Format a type id by canonical MIR name.
pub(crate) fn format_type_id<'a>(ty: TypeId, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    // name an anonymous type met again inside its own expansion
    if f.context().expanding.contains(&ty) {
        return write!(f, [copied_text(&format!("type@{}", ty.0))]);
    }

    // expand the type while marking it as in progress
    f.context_mut().expanding.push(ty);
    let node = f.context().tree.get(ty);
    let result = format_type_expanded(f, ty, node);
    f.context_mut().expanding.pop();

    result
}

/// Format a block id by canonical MIR name.
pub(crate) fn format_block_id<'a>(block: BlockId, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    let name = f.context().block_name(block)?;

    write!(f, [copied_text(&name)])
}

/// Format a function id by canonical MIR name.
pub(crate) fn format_function_id<'a>(
    function: FunctionId,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    let name = f.context().function_name(function).to_string();
    let tree = f.context().tree;
    let arguments = &tree.get(function).arguments;
    write!(f, [copied_text(&name)])?;

    format_generic_arguments(arguments, f)
}

/// Format a global id by canonical MIR name.
pub(crate) fn format_global_id<'a>(global: GlobalId, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    let name = f.context().global_name(global).to_string();

    write!(f, [copied_text(&name)])
}

/// Format one constant with an expected MIR type.
pub(super) fn format_constant_for_type<'a>(
    constant: &Constant,
    ty: TypeId,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    let ty = constant_storage_type(ty, f);
    let expected = f.context().tree.type_definition(ty);

    match (constant, expected) {
        (
            Constant::Int {
                value,
                width,
                is_signed: true,
            },
            Type::Int {
                width: expected_width,
                is_signed: true,
            },
        ) if width == expected_width => write!(f, [copied_text(&value.to_string())]),
        (
            Constant::UInt { value, width },
            Type::Int {
                width: expected_width,
                is_signed: false,
            },
        ) if width == expected_width => write!(f, [copied_text(&value.to_string())]),
        (
            Constant::Int {
                value,
                width,
                is_signed: true,
            },
            Type::Isize,
        ) if *width == f.context().target_layout.pointer_bits() => {
            write!(f, [copied_text(&value.to_string())])
        }
        (Constant::UInt { value, width }, Type::Usize)
            if *width == f.context().target_layout.pointer_bits() =>
        {
            write!(f, [copied_text(&value.to_string())])
        }
        (Constant::Float { bits, format }, Type::Float(expected_format))
            if format == expected_format =>
        {
            let value = float_from_bits(format.format(), *bits);
            write!(f, [copied_text(&value.to_string())])
        }
        _ => constant.format(f),
    }
}

/// Return the storage type used to format one typed constant.
fn constant_storage_type<'a>(ty: TypeId, f: &mut Writer<'a, '_>) -> TypeId {
    let expected = f.context().tree.type_definition(ty);
    if let Type::Newtype { value, .. } = expected {
        *value
    } else {
        ty
    }
}
