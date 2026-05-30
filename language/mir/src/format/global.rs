use destack_core::float_from_bits;
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::attribute::{write_attributes, write_attributes_before_anchor};

use crate::{
    Constant, FormatMirNode, Global, GlobalInitializer, Linkage, LocalNodeId, MirFormatter,
    Mutability, Space,
};

impl<'a> FormatMirNode<'a, Global> for Global {
    fn format_node(
        &self,
        id: LocalNodeId<Global>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let tree = f.context().tree;

        // explicit attributes
        let attributes = tree.attributes(id);

        if !attributes.is_empty() {
            if let Some(keyword_span) = tree.keyword_span(id) {
                write_attributes_before_anchor(
                    attributes,
                    tree.attribute_spans(id),
                    keyword_span.start,
                    tree,
                    f,
                )?;
            } else {
                write_attributes(attributes, f)?;
            }
        }

        // resolve the global name before formatting
        let name = f.context().global_name(id).to_string();

        // declaration modifiers
        if self.linkage.is_import() {
            write!(f, [token("external"), space()])?;
        } else if self.linkage == Linkage::Export {
            write!(f, [token("export"), space()])?;
        }

        if self.mutability == Mutability::Immutable {
            write!(f, [token("readonly"), space()])?;
        }

        // global header
        write!(
            f,
            [token("global"), space(), text(&name), token(":"), space()]
        )?;
        write!(f, [self.ty])?;
        if self.space != Space::Local {
            write!(
                f,
                [
                    token(","),
                    space(),
                    text(&format!("space({})", self.space.label()))
                ]
            )?;
        }

        if !self.linkage.is_import() {
            write!(f, [space(), token("="), space()])?;

            // format initializer
            if let Some(init) = &self.initializer {
                format_data_init(init, f)?;
            }
        }

        Ok(())
    }
}

/// Format a data initializer.
fn format_data_init<'a>(
    init: &GlobalInitializer,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match init {
        GlobalInitializer::Zero => write!(f, [token("zeroInit")]),
        GlobalInitializer::Scalar(constant) => format_constant(constant, f),
        GlobalInitializer::FunctionAddress(function) => {
            write!(f, [token("functionAddress"), space(), function])
        }
        GlobalInitializer::Bytes(bytes) => format_byte_literal(bytes, f),
        GlobalInitializer::Aggregate(elements) => {
            write!(f, [token("{")])?;
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                format_data_init(elem, f)?;
            }
            write!(f, [token("}")])
        }
    }
}

/// Format a byte literal with escaping.
fn format_byte_literal<'a>(bytes: &[u8], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("b"), token("\"")])?;
    for &byte in bytes {
        if byte == b'"' {
            write!(f, [text("\\\"")])?;
        } else if byte == b'\\' {
            write!(f, [text("\\\\")])?;
        } else if byte == b'\n' {
            write!(f, [text("\\n")])?;
        } else if byte == b'\r' {
            write!(f, [text("\\r")])?;
        } else if byte == b'\t' {
            write!(f, [text("\\t")])?;
        } else if byte.is_ascii_graphic() || byte == b' ' {
            write!(f, [text(&String::from(byte as char))])?;
        } else {
            write!(f, [text(&format!("\\x{byte:02x}"))])?;
        }
    }
    write!(f, [token("\"")])
}

/// Format a constant value.
fn format_constant<'a>(constant: &Constant, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    match constant {
        Constant::Null => write!(f, [text("null")]),
        Constant::Boolean { value } => {
            write!(f, [text(if *value { "true" } else { "false" })])
        }
        Constant::Int {
            value,
            width,
            is_signed: _,
        } => {
            write!(f, [text(&format!("{value}int{width}"))])
        }
        Constant::UInt { value, width } => {
            write!(f, [text(&format!("{value}uint{width}"))])
        }
        Constant::Float { bits, format } => {
            let value = float_from_bits(format.format(), *bits);
            write!(f, [text(&format!("{value}{}", format.label()))])
        }
        Constant::Char { value } => {
            write!(f, [text(&format!("{value:?}"))])
        }
    }
}
