use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::attribute::{write_attributes, write_attributes_before_anchor};
use super::value::format_constant_for_type;

use crate::{
    FormatMirNode, Global, GlobalInitializer, Linkage, LocalNodeId, MirFormatter, Mutability,
    Space, Type,
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
                format_data_init(init, self.ty.ty(), f)?;
            }
        }

        Ok(())
    }
}

/// Format a data initializer.
fn format_data_init<'a>(
    init: &GlobalInitializer,
    ty: Option<LocalNodeId<crate::Type>>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match init {
        GlobalInitializer::Zero => write!(f, [token("zeroInit")]),
        GlobalInitializer::Scalar(constant) => {
            if let Some(ty) = ty {
                return format_constant_for_type(constant, ty, f);
            }

            write!(f, [constant])
        }
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
                let element_type = data_init_element_type(ty, i, f);
                format_data_init(elem, element_type, f)?;
            }
            write!(f, [token("}")])
        }
    }
}

/// Return the expected type for one aggregate initializer element.
fn data_init_element_type<'a>(
    ty: Option<LocalNodeId<Type>>,
    index: usize,
    f: &mut MirFormatter<'a, '_>,
) -> Option<LocalNodeId<Type>> {
    let ty = ty?;

    match f.context().tree.get(ty) {
        Type::Array { element, .. }
        | Type::Vector { element, .. }
        | Type::Tensor { element, .. } => element.ty(),
        Type::Tuple { elements, .. } => elements.get(index).and_then(|element| element.ty()),
        Type::Struct { fields, .. } => fields
            .get(index)
            .and_then(|field| f.context().tree.get(*field).ty.ty()),
        Type::Newtype { inner, .. } => data_init_element_type(inner.ty(), index, f),
        _ => None,
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
