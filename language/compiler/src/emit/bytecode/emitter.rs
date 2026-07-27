use destack_artifact::{FrameState, MirOptimized};
use destack_bytecode as bytecode;
use destack_core::{EntryRange, Optional};
use destack_mir as mir;
use destack_source::ModuleId;

use crate::{EmitError, ObjectEmitter};

use super::{FunctionEmission, FunctionEmitter, TypeEmitter};

/// Emit one relocatable bytecode object from optimized MIR.
#[derive(Debug)]
pub struct BytecodeEmitter<'a> {
    /// Optimized MIR being emitted.
    optimized: &'a MirOptimized,
    /// Module owning the emitted MIR.
    module: ModuleId,
    /// Object-local identity assignments.
    object: &'a ObjectEmitter,
    /// Physical bytecode type projection.
    types: TypeEmitter<'a>,
}

#[allow(clippy::too_many_arguments)]
impl<'a> BytecodeEmitter<'a> {
    /// Create one bytecode emitter.
    pub fn new(module: ModuleId, optimized: &'a MirOptimized, object: &'a ObjectEmitter) -> Self {
        Self {
            optimized,
            module,
            object,
            types: TypeEmitter::new(module, optimized, object),
        }
    }

    /// Emit one relocatable bytecode object and its logical frame states.
    pub fn emit(&self) -> Result<(bytecode::Object, Vec<FrameState>), EmitError> {
        let functions = self.ordered_functions()?;
        let mut logical_frames = Vec::new();
        let mut frames = Vec::new();
        let mut registers = Vec::new();
        let mut operations = Vec::new();
        let mut relocations = Vec::new();
        let mut code = Vec::new();
        let mut rows = Vec::with_capacity(functions.len());

        // emit functions in object order
        for (function_id, function) in functions {
            if function.body.is_none() {
                rows.push(bytecode::Function::declaration());

                continue;
            }

            let emitted = FunctionEmitter::new(
                self.module,
                self.optimized,
                self.object,
                &self.types,
                function_id,
                function,
            )?
            .emit()?;
            rows.push(Self::append(
                emitted,
                &mut logical_frames,
                &mut frames,
                &mut registers,
                &mut operations,
                &mut relocations,
                &mut code,
            ));
        }

        let object = bytecode::ObjectBuilder::new()
            .functions(rows)
            .frames(frames)
            .registers(registers)
            .operations(operations)
            .relocations(relocations)
            .code(code)
            .build();

        Ok((object, logical_frames))
    }

    /// Return MIR functions in object identity order.
    fn ordered_functions(&self) -> Result<Vec<(mir::FunctionId, &mir::Function)>, EmitError> {
        let mut functions = self
            .optimized
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, function)| {
                let index = self.types.function_id(id)?.index();

                Ok((index, id, function))
            })
            .collect::<Result<Vec<_>, EmitError>>()?;
        functions.sort_unstable_by_key(|(index, _, _)| *index);

        Ok(functions
            .into_iter()
            .map(|(_, id, function)| (id, function))
            .collect())
    }

    /// Append one emitted function and return its physical object row.
    fn append(
        emitted: FunctionEmission,
        logical_frames: &mut Vec<FrameState>,
        frames: &mut Vec<bytecode::FrameMap>,
        registers: &mut Vec<bytecode::RegisterSpan>,
        operations: &mut Vec<bytecode::CodeOffset>,
        relocations: &mut Vec<bytecode::Relocation>,
        code: &mut Vec<u8>,
    ) -> bytecode::Function {
        let FunctionEmission {
            body,
            frames: emitted_frames,
        } = emitted;

        // append matching logical and physical frame states
        for frame in emitted_frames {
            let register_start = registers.len() as u32;
            let register_count = frame.registers.len() as u32;
            registers.extend(frame.registers);
            frames.push(bytecode::FrameMap::new(EntryRange::new(
                register_start,
                register_count,
            )));
            logical_frames.push(FrameState::new(frame.point, frame.types));
        }

        // append logical operation offsets
        let operation_start = operations.len() as u32;
        let operation_count = body.operations.len() as u32;
        operations.extend(body.operations);

        // append encoded bytes and relocate their object offsets
        let code_start = code.len() as u32;
        let code_len = body.code.len() as u32;
        code.extend(body.code);
        relocations.extend(
            body.relocations
                .into_iter()
                .map(|relocation| relocation.rebase(code_start)),
        );

        bytecode::Function::new(
            Optional::some(bytecode::CodeRange {
                byte_offset: code_start,
                byte_len: code_len,
            }),
            EntryRange::new(operation_start, operation_count),
            body.register_count,
        )
    }
}
