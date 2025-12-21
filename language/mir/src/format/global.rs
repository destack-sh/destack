//! Global formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    Constant, FormatMirNode, Global, GlobalInitializer, Linkage, LocalNodeId, MirFormatter,
    Mutability,
};

impl<'a> FormatMirNode<'a, Global> for Global {
    fn format_node(
        &self,
        _id: LocalNodeId<Global>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let name = f.context().strings.get(self.name);

        // imported globals: extern global @name: type ; mut
        if self.linkage.is_import() {
            write!(
                f,
                [
                    token("extern"),
                    space(),
                    token("global"),
                    space(),
                    token("@"),
                    text(name),
                    token(":"),
                    space(),
                    self.ty
                ]
            )?;
        } else {
            // linkage prefix for exported globals
            if self.linkage == Linkage::Export {
                write!(f, [token("export"), space()])?;
            }

            // local/exported globals: [export] global @name: type = init ; mut
            write!(
                f,
                [
                    token("global"),
                    space(),
                    token("@"),
                    text(name),
                    token(":"),
                    space(),
                    self.ty,
                    space(),
                    token("="),
                    space()
                ]
            )?;

            // format initializer
            if let Some(init) = &self.initializer {
                format_data_init(init, f)?;
            }
        }

        // mutability annotation
        write!(f, [space(), token(";"), space()])?;
        match self.mutability {
            Mutability::Mutable => write!(f, [token("mut")])?,
            Mutability::Immutable => write!(f, [token("const")])?,
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
        GlobalInitializer::Zero => write!(f, [token("zeroinit")]),
        GlobalInitializer::Scalar(constant) => format_constant(constant, f),
        GlobalInitializer::Bytes(bytes) => {
            // format as quoted string, escaping non-printable bytes
            write!(f, [token("\"")])?;
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

/// Format a constant value.
fn format_constant<'a>(constant: &Constant, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    match constant {
        Constant::Boolean { value } => {
            write!(f, [text(if *value { "true" } else { "false" })])
        }
        Constant::Int {
            value,
            width,
            is_signed: _,
        } => {
            write!(f, [text(&format!("{value}i{width}"))])
        }
        Constant::UInt { value, width } => {
            write!(f, [text(&format!("{value}u{width}"))])
        }
        Constant::Float { bits, width } => {
            let value = if *width == 32 {
                f32::from_bits(*bits as u32) as f64
            } else {
                f64::from_bits(*bits)
            };
            write!(f, [text(&format!("{value}f{width}"))])
        }
        Constant::String { value } => {
            write!(f, [text(&format!("{value:?}"))])
        }
        Constant::Char { value } => {
            write!(f, [text(&format!("{value:?}"))])
        }
    }
}
