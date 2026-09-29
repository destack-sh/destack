use tspp_bytecode as bytecode;
use tspp_core::EntryRange;

use crate::CompilerResult;

use super::BytecodeLinker;
use crate::invalid_program_input;

impl<'a, 'b> BytecodeLinker<'a, 'b> {
    /// Link bytecode frame maps into canonical Program frame state order.
    pub(super) fn link_frames(
        &self,
    ) -> CompilerResult<(Vec<bytecode::FrameMap>, Vec<bytecode::RegisterSpan>)> {
        let mut sources = Vec::new();

        // resolve every object-local map to its canonical frame state
        for (module, object) in self.program.objects() {
            let bytecode = self.object(object)?;
            if object.frames().len() != bytecode.frames().len() {
                return Err(invalid_program_input(
                    "logical and physical frame counts differ",
                ));
            }

            for (state, map) in object.frames().iter().zip(bytecode.frames()) {
                let state = self
                    .frames
                    .state(*module, state.point)
                    .ok_or_else(|| invalid_program_input("linked frame state is absent"))?;
                sources.push((state, map, bytecode.registers()));
            }
        }

        // order source maps and prepare canonical output
        sources.sort_unstable_by_key(|(state, _, _)| *state);
        let mut frames = Vec::with_capacity(sources.len());
        let mut registers = Vec::new();

        // append each map under its matching dense Program frame state id
        for (state, map, source_registers) in sources {
            if state.index() != frames.len() {
                return Err(invalid_program_input(
                    "bytecode frame maps are not canonical",
                ));
            }

            // append the physical spans under their canonical state identity
            let register_start = registers.len() as u32;
            let source_registers = map.registers(source_registers);
            registers.extend_from_slice(source_registers);
            frames.push(bytecode::FrameMap::new(EntryRange::new(
                register_start,
                source_registers.len() as u32,
            )));
        }

        Ok((frames, registers))
    }
}
