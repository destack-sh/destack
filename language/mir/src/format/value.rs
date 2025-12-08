//! Value and constant formatting.

use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Constant, MirFormatContext, MirFormatter, Value};

impl<'a> Format<MirFormatContext<'a>> for Value {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        write!(f, [text(&format!("v{}", self.0))])
    }
}

impl<'a> Format<MirFormatContext<'a>> for Constant {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        match self {
            Constant::Boolean { value } => {
                write!(f, [text(if *value { "true" } else { "false" })])
            }
            Constant::Int {
                value,
                width,
                is_signed,
            } => {
                let suffix = if *is_signed {
                    format!("i{width}")
                } else {
                    format!("u{width}")
                };
                write!(f, [text(&format!("{value}{suffix}"))])
            }
            Constant::UInt { value, width } => {
                write!(f, [text(&format!("{value}u{width}"))])
            }
            Constant::Float { bits, width } => {
                let value_str = if *width == 32 {
                    format!("{}f32", f32::from_bits(*bits as u32))
                } else {
                    format!("{}f64", f64::from_bits(*bits))
                };
                write!(f, [text(&value_str)])
            }
            Constant::String { value } => {
                // Escape special characters for display
                write!(f, [text(&format!("{value:?}"))])
            }
            Constant::Char { value } => {
                write!(f, [text(&format!("{value:?}"))])
            }
        }
    }
}
