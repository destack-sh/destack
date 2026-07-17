use destack_core::float_from_bits;
use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use super::r#type::format_type_expanded;

use crate::{
    BlockId, Constant, FunctionId, GlobalId, LocalNodeId, MirFormatContext, MirFormatter, Place,
    PlaceOrigin, Projection, Type, TypeId, Value,
};

impl<'a> Format<'a, MirFormatContext<'a>> for Value {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let name = f.context().value_name(*self)?;
        write!(f, [copied_text(&name)])
    }
}

impl<'a> Format<'a, MirFormatContext<'a>> for Place {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        if let PlaceOrigin::Value(value) = self.origin
            && self.path.is_root()
        {
            return value.format(f);
        }

        write!(f, [token("place"), token("(")])?;
        format_place_origin(&self.origin, f)?;
        for projection in &self.path.projections {
            write!(f, [token(","), space()])?;
            format_place_projection(projection, f)?;
        }
        write!(f, [token(")")])
    }
}

impl<'a> Format<'a, MirFormatContext<'a>> for Constant {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        match self {
            Constant::Null => write!(f, [token("null")]),
            Constant::Undefined => write!(f, [token("undefined")]),
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
pub(crate) fn format_type_id<'a>(ty: TypeId, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    if let Some(name) = f.context().type_alias_name(ty).map(str::to_string) {
        return write!(f, [copied_text(&name)]);
    }

    let node = f.context().tree.get(ty);

    format_type_expanded(f, ty, node)
}

/// Format a block id by canonical MIR name.
pub(crate) fn format_block_id<'a>(
    block: BlockId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let name = f.context().block_name(block);

    write!(f, [copied_text(&name)])
}

/// Format a function id by canonical MIR name.
pub(crate) fn format_function_id<'a>(
    function: FunctionId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let name = f.context().function_name(function).to_string();

    write!(f, [copied_text(&name)])
}

/// Format a global id by canonical MIR name.
pub(crate) fn format_global_id<'a>(
    global: GlobalId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let name = f.context().global_name(global).to_string();

    write!(f, [copied_text(&name)])
}

/// Format one constant with an expected MIR type.
pub(super) fn format_constant_for_type<'a>(
    constant: &Constant,
    ty: LocalNodeId<Type>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let ty = constant_storage_type(ty, f);
    let expected = f.context().tree.get(ty);

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
fn constant_storage_type<'a>(
    ty: LocalNodeId<Type>,
    f: &mut MirFormatter<'a, '_>,
) -> LocalNodeId<Type> {
    let expected = f.context().tree.get(ty);
    if let Type::Newtype { inner, .. } = expected {
        *inner
    } else {
        ty
    }
}

/// Format one MIR place origin.
fn format_place_origin<'a>(origin: &PlaceOrigin, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    match origin {
        PlaceOrigin::Local(local) => {
            let index = f.context().local_index(*local);
            write!(f, [copied_text(&format!("l{index}"))])
        }
        PlaceOrigin::Global(global) => global.format(f),
        PlaceOrigin::Value(value) => value.format(f),
    }
}

/// Format one MIR place projection.
fn format_place_projection<'a>(
    projection: &Projection,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match projection {
        Projection::Field { index } => write!(
            f,
            [
                token("field"),
                token("("),
                copied_text(&index.to_string()),
                token(")")
            ]
        ),
        Projection::Element { index } => write!(
            f,
            [
                token("element"),
                token("("),
                copied_text(&index.to_string()),
                token(")")
            ]
        ),
        Projection::Index { index } => write!(f, [token("index"), token("("), index, token(")")]),
        Projection::AnyElement => {
            write!(f, [token("element"), token("("), token("any"), token(")")])
        }
        Projection::Slice { start, length } => write!(
            f,
            [
                token("slice"),
                token("("),
                start,
                token(","),
                space(),
                length,
                token(")")
            ]
        ),
    }
}
