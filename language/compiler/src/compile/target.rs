use std::str::FromStr;

use target_lexicon::{Endianness, HOST, Triple};
use tspp_artifact::{Output, TargetArch};
use tspp_mir::{Endian, TargetLayout};
use tspp_repository::Target;
use tspp_source::TargetId;

use crate::Compiler;

impl Compiler {
    /// Resolve the ABI layout selected by one target.
    pub(crate) fn target_layout(
        &self,
        target: &Target,
        target_id: TargetId,
    ) -> Result<TargetLayout, String> {
        // reject targets without a physical Program ABI
        if matches!(target.output, Output::Bundle) {
            return Err(format!("target {target_id} does not have a physical ABI"));
        }

        // explicit target triple
        if let Some(triple) = target.resolved_target_triple() {
            let triple = Triple::from_str(&triple)
                .map_err(|error| format!("invalid target triple: {error}"))?;

            return Self::layout_from_triple(&triple);
        }

        // explicit architecture without a complete triple
        if let Some(architecture) = target.architecture.as_ref() {
            let pointer_bytes = architecture
                .pointer_bytes()
                .ok_or_else(|| format!("unknown pointer width for {architecture:?}"))?;
            let endian = match architecture {
                TargetArch::PowerPc64 | TargetArch::S390x | TargetArch::Mips64 => Endian::Big,
                _ => Endian::Little,
            };
            let mut layout = TargetLayout::for_pointer_bytes(pointer_bytes);
            layout.endian = endian;

            return Ok(layout);
        }

        Self::layout_from_triple(&HOST)
    }

    /// Resolve one ABI layout from a concrete target triple.
    fn layout_from_triple(triple: &Triple) -> Result<TargetLayout, String> {
        let pointer_bytes = triple
            .pointer_width()
            .map_err(|_| "target pointer width is unknown".to_string())?
            .bits()
            / 8;
        let endian = match triple
            .endianness()
            .map_err(|_| "target byte order is unknown".to_string())?
        {
            Endianness::Little => Endian::Little,
            Endianness::Big => Endian::Big,
        };
        let mut layout = TargetLayout::for_pointer_bytes(pointer_bytes);
        layout.endian = endian;

        Ok(layout)
    }
}
