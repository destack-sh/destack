use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Constant, MirFormatContext, MirFormatter, Value};

impl<'a> Format<MirFormatContext<'a>> for Value {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let index = f.context().value_index(*self);
        write!(f, [text(&format!("v{index}"))])
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
