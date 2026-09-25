use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;

use super::attribute::{format_string_literal, write_attributes, write_attributes_before_anchor};
use super::value::format_constant_for_type;

use crate::{
    FormatNode, Global, GlobalInitializer, Linkage, LocalNodeId, Mutability, Space, Type, TypeId,
    Writer,
};

impl FormatNode for Global {
    fn format_node<'a>(&self, id: LocalNodeId<Global>, f: &mut Writer<'a, '_>) -> FormatResult<()> {
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
        } else if self.linkage == Linkage::Shared {
            write!(f, [token("shared"), space()])?;
        }

        // reject mutable constants
        let is_constant = self.space == Space::Constant;
        if is_constant && self.mutability != Mutability::Immutable {
            return Err(FormatError::SyntaxError {
                message: "constant global is mutable",
            });
        }

        // format storage and mutability modifiers
        if !is_constant && self.mutability == Mutability::Immutable {
            write!(f, [token("readonly"), space()])?;
        }
        if self.space == Space::Shared {
            write!(f, [token("shared"), space()])?;
        }

        // select the declaration noun
        let keyword = if is_constant { "constant" } else { "global" };

        // global header
        write!(
            f,
            [
                token(keyword),
                space(),
                copied_text(&name),
                token(":"),
                space()
            ]
        )?;
        write!(f, [self.ty])?;

        if !self.linkage.is_import() {
            write!(f, [space(), token("="), space()])?;

            // format initializer
            if let Some(init) = &self.initializer {
                format_data_init(init, Some(self.ty), f)?;
            }
        }

        Ok(())
    }
}

/// Format a data initializer.
fn format_data_init<'a>(
    init: &GlobalInitializer,
    ty: Option<TypeId>,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    match init {
        GlobalInitializer::Zero => write!(f, [token("zeroinit")]),
        GlobalInitializer::Scalar(constant) => {
            if let Some(ty) = ty {
                return format_constant_for_type(constant, ty, f);
            }

            write!(f, [constant])
        }
        GlobalInitializer::FunctionAddress(function) => {
            write!(f, [token("functionAddress"), space(), function])
        }
        GlobalInitializer::GlobalAddress(global) => {
            let name = f.context().global_name(*global).to_string();

            write!(f, [token("globalAddress"), space(), copied_text(&name)])
        }
        GlobalInitializer::Bytes(bytes) => format_byte_literal(bytes, f),
        GlobalInitializer::String(value) => {
            let value = f.context().strings.get(*value).to_string();

            format_string_literal(&value, f)
        }
        GlobalInitializer::BigInt(value) => write!(f, [copied_text(&format!("{value}n"))]),
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
    ty: Option<TypeId>,
    index: usize,
    f: &mut Writer<'a, '_>,
) -> Option<TypeId> {
    let ty = ty?;

    match f.context().tree.type_definition(ty) {
        Type::FixedArray { element, .. } | Type::Vector { element, .. } => Some(*element),
        Type::Tuple { elements, .. } => elements.get(index).copied(),
        Type::Struct { fields, .. } => fields
            .get(index)
            .map(|field| f.context().tree.get(*field).ty),
        Type::Newtype { value, .. } => data_init_element_type(Some(*value), index, f),
        _ => None,
    }
}

/// Format a byte literal with escaping.
fn format_byte_literal<'a>(bytes: &[u8], f: &mut Writer<'a, '_>) -> FormatResult<()> {
    write!(f, [token("b"), token("\"")])?;
    for &byte in bytes {
        if byte == b'"' {
            write!(f, [token("\\\"")])?;
        } else if byte == b'\\' {
            write!(f, [token("\\\\")])?;
        } else if byte == b'\n' {
            write!(f, [token("\\n")])?;
        } else if byte == b'\r' {
            write!(f, [token("\\r")])?;
        } else if byte == b'\t' {
            write!(f, [token("\\t")])?;
        } else if byte.is_ascii_graphic() || byte == b' ' {
            write!(f, [copied_text(&String::from(byte as char))])?;
        } else {
            write!(f, [copied_text(&format!("\\x{byte:02x}"))])?;
        }
    }
    write!(f, [token("\"")])
}
