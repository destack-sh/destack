use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::attribute::{write_attributes, write_attributes_before_anchor};

use crate::{
    AddressSpace, Constant, FormatMirNode, Global, GlobalInitializer, Linkage, LocalNodeId,
    MirFormatter, Mutability,
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

        // imported globals
        if self.linkage.is_import() {
            write!(
                f,
                [
                    token("extern"),
                    space(),
                    token("global"),
                    space(),
                    text(&name),
                    token(":"),
                    space(),
                    self.ty
                ]
            )?;

            if self.mutability == Mutability::Immutable {
                write!(f, [token(","), space(), token("readonly")])?;
            }
            if self.space != AddressSpace::Local {
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        text(&format!("space({})", self.space.label()))
                    ]
                )?;
            }
        } else {
            // linkage prefix for exported globals
            if self.linkage == Linkage::Export {
                write!(f, [token("export"), space()])?;
            }

            // local/exported globals
            write!(
                f,
                [
                    token("global"),
                    space(),
                    text(&name),
                    token(":"),
                    space(),
                    self.ty
                ]
            )?;

            if self.mutability == Mutability::Immutable {
                write!(f, [token(","), space(), token("readonly")])?;
            }
            if self.space != AddressSpace::Local {
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        text(&format!("space({})", self.space.label()))
                    ]
                )?;
            }

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
        Constant::Float { bits, width } => {
            let value = if *width == 32 {
                f32::from_bits(*bits as u32) as f64
            } else {
                f64::from_bits(*bits)
            };
            write!(f, [text(&format!("{value}float{width}"))])
        }
        Constant::Char { value } => {
            write!(f, [text(&format!("{value:?}"))])
        }
    }
}
