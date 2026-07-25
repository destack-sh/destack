use std::str::FromStr;

use destack_artifact::{EmitFormat, TargetArch};
use destack_mir::{Endian, TargetLayout};
use destack_repository::Target;
use destack_source::TargetId;
use target_lexicon::{Endianness, HOST, Triple};

use crate::Compiler;

impl Compiler {
    /// Resolve the ABI layout selected by one target.
    pub(crate) fn target_layout(
        &self,
        target: &Target,
        target_id: TargetId,
    ) -> Result<TargetLayout, String> {
        // fixed virtual machine ABI
        if target.emit == EmitFormat::Bytecode {
            return Ok(TargetLayout::for_pointer_bytes(8));
        }

        // fixed WebAssembly ABI
        if target.emit == EmitFormat::Wasm {
            let pointer_bytes = match target.native.arch.as_ref() {
                None | Some(TargetArch::Wasm32) => 4,
                Some(TargetArch::Wasm64) => 8,
                Some(architecture) => {
                    return Err(format!(
                        "WebAssembly target cannot use architecture {architecture:?}"
                    ));
                }
            };

            return Ok(TargetLayout::for_pointer_bytes(pointer_bytes));
        }

        // script targets never construct physical MIR
        if target.emit.is_script() {
            return Err(format!("target {target_id} does not have a physical ABI"));
        }

        // explicit native target triple
        if let Some(triple) = target.resolved_target_triple() {
            let triple = Triple::from_str(&triple)
                .map_err(|error| format!("invalid target triple: {error}"))?;

            return Self::layout_from_triple(&triple);
        }

        // explicit native architecture without a complete triple
        if let Some(architecture) = target.native.arch.as_ref() {
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
