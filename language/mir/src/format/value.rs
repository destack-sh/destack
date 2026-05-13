use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    BlockReference, Constant, FunctionReference, GlobalReference, IntegerReference, LocalReference,
    MirFormatContext, MirFormatter, Place, PlaceOrigin, PlaceProjection, TypeReference, Value,
    ValueReference,
};

fn write_recovery_token<'a>(is_missing: bool, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    let token_text = if is_missing { "<missing>" } else { "<error>" };

    write!(f, [token(token_text)])
}

impl<'a> Format<MirFormatContext<'a>> for Value {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let name = f.context().value_name(*self);
        write!(f, [text(&name)])
    }
}

impl<'a> Format<MirFormatContext<'a>> for ValueReference {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        match self {
            ValueReference::Value(value) => value.format(f),
            ValueReference::Missing => write_recovery_token(true, f),
            ValueReference::Error => write_recovery_token(false, f),
        }
    }
}

impl<'a> Format<MirFormatContext<'a>> for Place {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        if let PlaceOrigin::Value(value) = self.origin
            && self.projections.is_empty()
        {
            return value.format(f);
        }

        write!(f, [token("place"), token("(")])?;
        format_place_origin(&self.origin, f)?;
        for projection in &self.projections {
            write!(f, [token(","), space()])?;
            format_place_projection(projection, f)?;
        }
        write!(f, [token(")")])
    }
}

impl<'a> Format<MirFormatContext<'a>> for TypeReference {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        match self {
            TypeReference::Type(ty) => ty.format(f),
            TypeReference::Missing => write_recovery_token(true, f),
            TypeReference::Error => write_recovery_token(false, f),
        }
    }
}

impl<'a> Format<MirFormatContext<'a>> for BlockReference {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        match self {
            BlockReference::Block(block) => {
                let name = f.context().block_name(*block);
                write!(f, [text(&name)])
            }
            BlockReference::Missing => write_recovery_token(true, f),
            BlockReference::Error => write_recovery_token(false, f),
        }
    }
}

impl<'a> Format<MirFormatContext<'a>> for FunctionReference {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        match self {
            FunctionReference::Function(function) => {
                let name = f.context().function_name(*function).to_string();
                write!(f, [text(&name)])
            }
            FunctionReference::Missing => write_recovery_token(true, f),
            FunctionReference::Error => write_recovery_token(false, f),
        }
    }
}

impl<'a> Format<MirFormatContext<'a>> for GlobalReference {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        match self {
            GlobalReference::Global(global) => {
                let name = f.context().global_name(*global).to_string();
                write!(f, [text(&name)])
            }
            GlobalReference::Missing => write_recovery_token(true, f),
            GlobalReference::Error => write_recovery_token(false, f),
        }
    }
}

impl<'a> Format<MirFormatContext<'a>> for LocalReference {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        match self {
            LocalReference::Local(local) => {
                let index = f.context().local_index(*local);
                write!(f, [text(&format!("local{index}"))])
            }
            LocalReference::Missing => write_recovery_token(true, f),
            LocalReference::Error => write_recovery_token(false, f),
        }
    }
}

impl<'a> Format<MirFormatContext<'a>> for IntegerReference {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        match self {
            IntegerReference::Integer(value) => write!(f, [text(&value.to_string())]),
            IntegerReference::Missing => write_recovery_token(true, f),
            IntegerReference::Error => write_recovery_token(false, f),
        }
    }
}

impl<'a> Format<MirFormatContext<'a>> for Constant {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        match self {
            Constant::Null => write!(f, [text("null")]),
            Constant::Boolean { value } => {
                write!(f, [text(if *value { "true" } else { "false" })])
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
                write!(f, [text(&format!("{value}{suffix}"))])
            }
            Constant::UInt { value, width } => {
                write!(f, [text(&format!("{value}uint{width}"))])
            }
            Constant::Float { bits, width } => {
                let value_str = if *width == 32 {
                    format!("{}float32", f32::from_bits(*bits as u32))
                } else {
                    format!("{}float64", f64::from_bits(*bits))
                };
                write!(f, [text(&value_str)])
            }
            Constant::Char { value } => {
                write!(f, [text(&format!("{value:?}"))])
            }
        }
    }
}

/// Format one MIR place origin.
fn format_place_origin<'a>(origin: &PlaceOrigin, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    match origin {
        PlaceOrigin::Local(local) => local.format(f),
        PlaceOrigin::Global(global) => global.format(f),
        PlaceOrigin::Value(value) => value.format(f),
    }
}

/// Format one MIR place projection.
fn format_place_projection<'a>(
    projection: &PlaceProjection,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match projection {
        PlaceProjection::Static { index } => write!(
            f,
            [
                token("static"),
                token("("),
                text(&index.to_string()),
                token(")")
            ]
        ),
        PlaceProjection::Dynamic { index } => {
            write!(f, [token("dynamic"), token("("), index, token(")")])
        }
        PlaceProjection::Range { start, length } => write!(
            f,
            [
                token("range"),
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
