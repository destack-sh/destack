use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::{Address, Opcode, Placement, RegisterSpan, RelocationTag};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one packed value or variant operation.
    pub(super) fn format_aggregate(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::AGGREGATE => self.format_aggregate_new(),
            Opcode::EXTRACT => self.format_extract(),
            Opcode::INSERT => self.format_insert(),
            Opcode::VARIANT_NEW => self.format_variant_new(),
            Opcode::VARIANT_TAG => self.format_variant_tag(),
            Opcode::VARIANT_TAG_LOAD | Opcode::VARIANT_TAG_LOAD_POINTER => {
                self.format_variant_tag_load(opcode)
            }
            _ => Err(FormatError::SyntaxError {
                message: "invalid aggregate opcode",
            }),
        }
    }

    /// Format one packed aggregate construction.
    fn format_aggregate_new(&mut self) -> FormatResult<()> {
        self.write_opcode("aggregate")?;
        self.result_span()?;
        let placements = self
            .operands
            .placements()
            .map_err(FormatError::from)?
            .collect::<Vec<_>>();

        // write each exact physical source placement
        self.write_comma()?;
        self.write_token("[")?;
        for (index, placement) in placements.into_iter().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), soft_line_break_or_space()])?;
            }
            self.write_placement(placement)?;
        }
        self.write_token("]")
    }

    /// Format one value extraction by byte range.
    fn format_extract(&mut self) -> FormatResult<()> {
        self.write_opcode("extract")?;
        self.result_span()?;
        let (source, word_count) = self.register_span_id()?;
        let source = RegisterSpan::new(source, word_count);
        let byte_offset = self.u32()?.to_string();
        let byte_len = self.u32()?.to_string();

        // write the exact source byte range
        self.write_comma()?;
        self.write_span(source)?;
        self.write_comma()?;
        self.write_text(&byte_offset)?;
        self.write_token(":")?;
        self.write_text(&byte_len)
    }

    /// Format one persistent value insertion by byte range.
    fn format_insert(&mut self) -> FormatResult<()> {
        self.write_opcode("insert")?;
        self.result_span()?;
        let (aggregate, aggregate_word_count) = self.register_span_id()?;
        let aggregate = RegisterSpan::new(aggregate, aggregate_word_count);
        let byte_offset = self.u32()?.to_string();
        let byte_len = self.u32()?.to_string();
        let (value, value_word_count) = self.register_span_id()?;
        let value = RegisterSpan::new(value, value_word_count);

        // write the copied aggregate and exact replacement byte range
        self.write_comma()?;
        self.write_span(aggregate)?;
        self.write_comma()?;
        self.write_text(&byte_offset)?;
        self.write_token(":")?;
        self.write_text(&byte_len)?;
        self.write_comma()?;
        self.write_span(value)
    }

    /// Format one variant construction.
    fn format_variant_new(&mut self) -> FormatResult<()> {
        self.write_opcode("variant.new")?;
        self.result_span()?;
        let layout = self.layout()?;
        let case = self.u32()?.to_string();
        let (payload, payload_word_count) = self.register_span_id()?;

        // write the linked layout, case, and optional payload
        self.write_comma()?;
        self.write_text(&layout)?;
        self.write_comma()?;
        self.write_text(&case)?;
        if payload_word_count > 0 {
            self.write_comma()?;
            self.write_span(RegisterSpan::new(payload, payload_word_count))?;
        }

        Ok(())
    }

    /// Format one variant discriminant extraction.
    fn format_variant_tag(&mut self) -> FormatResult<()> {
        self.write_opcode("variant.tag")?;
        self.result_span()?;
        let (variant, word_count) = self.register_span_id()?;
        let variant = RegisterSpan::new(variant, word_count);
        let layout = self.layout()?;

        // write the variant and exact linked layout
        self.write_comma()?;
        self.write_span(variant)?;
        self.write_comma()?;
        self.write_text(&layout)
    }

    /// Format one stored variant discriminant load.
    fn format_variant_tag_load(&mut self, opcode: Opcode) -> FormatResult<()> {
        self.write_opcode("variant.tag.load")?;
        self.result_span()?;
        let variant = self.register_id()?;
        let layout = self.layout()?;

        // write the address register and exact linked layout
        self.write_comma()?;
        let address = if opcode == Opcode::VARIANT_TAG_LOAD_POINTER {
            Address::Pointer
        } else {
            Address::Reference
        };
        self.write_address_register(address, variant)?;
        self.write_comma()?;
        self.write_text(&layout)
    }

    /// Write one physical aggregate placement.
    fn write_placement(&mut self, placement: Placement) -> FormatResult<()> {
        let byte_offset = placement.byte_offset.to_string();
        let byte_len = placement.byte_len.to_string();

        self.write_span(placement.registers)?;
        write!(self.formatter, [space(), token("@"), space()])?;
        self.write_text(&byte_offset)?;
        self.write_token(":")?;
        self.write_text(&byte_len)
    }

    /// Read one relocated runtime layout name.
    fn layout(&mut self) -> FormatResult<String> {
        let (relocation, index) = self.relocation()?;
        if relocation.tag != RelocationTag::LAYOUT {
            return Err(FormatError::SyntaxError {
                message: "variant operation does not reference a layout",
            });
        }

        Ok(format!("l{index}"))
    }
}
