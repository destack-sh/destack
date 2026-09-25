use tspp_artifact::MirOptimized;
use tspp_bytecode as bytecode;
use tspp_core::{EntryRange, Optional};
use tspp_mir::ModuleCache;
use tspp_source::ModuleId;

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

    /// Emit one relocatable bytecode object.
    pub fn emit(&self, analyses: &mut ModuleCache) -> Result<bytecode::Object, EmitError> {
        // allocate the bytecode object tables
        let mut frames = Vec::new();
        let mut registers = Vec::new();
        let mut operations = Vec::new();
        let mut relocations = Vec::new();
        let mut code = Vec::new();
        let mut functions = Vec::with_capacity(self.object.functions().len());

        // emit functions in object order
        for &function_id in self.object.functions() {
            let function = self.optimized.tree.get(function_id);
            if function.body.is_none() {
                functions.push(bytecode::Function::declaration());

                continue;
            }

            // reuse the liveness queried during object frame emission
            let liveness = analyses.liveness(function_id, &self.optimized.tree);
            let emitted = FunctionEmitter::new(
                self.module,
                self.optimized,
                self.object,
                &self.types,
                function_id,
                function,
                &liveness,
            )?
            .emit()?;
            functions.push(self.append(
                emitted,
                &mut frames,
                &mut registers,
                &mut operations,
                &mut relocations,
                &mut code,
            )?);
        }

        // require one physical map for every logical bytecode frame state
        if frames.len() != self.object.frames().len() {
            return Err(self.internal("bytecode frame map count does not match object states"));
        }

        Ok(bytecode::ObjectBuilder::new()
            .functions(functions)
            .frames(frames)
            .registers(registers)
            .operations(operations)
            .relocations(relocations)
            .code(code)
            .build())
    }

    /// Append one emitted function and return its physical object entry.
    fn append(
        &self,
        emitted: FunctionEmission,
        frames: &mut Vec<bytecode::FrameMap>,
        registers: &mut Vec<bytecode::RegisterSpan>,
        operations: &mut Vec<bytecode::CodeOffset>,
        relocations: &mut Vec<bytecode::Relocation>,
        code: &mut Vec<u8>,
    ) -> Result<bytecode::Function, EmitError> {
        let FunctionEmission {
            body,
            frames: emitted_frames,
        } = emitted;

        // append physical maps in canonical logical frame order
        for frame in emitted_frames {
            let Some(state) = self.object.frames().get(frames.len()) else {
                return Err(self.internal("bytecode emitted an unknown frame map"));
            };
            if state.point != frame.point {
                return Err(self.internal("bytecode frame maps are not in object order"));
            }
            let register_start = registers.len() as u32;
            let register_count = frame.registers.len() as u32;
            registers.extend(frame.registers);
            let map = bytecode::FrameMap::new(EntryRange::new(register_start, register_count));
            frames.push(map);
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

        Ok(bytecode::Function::new(
            Optional::some(bytecode::CodeRange {
                byte_offset: code_start,
                byte_len: code_len,
            }),
            EntryRange::new(operation_start, operation_count),
            body.register_count,
        ))
    }

    /// Build one internal bytecode emission diagnostic.
    fn internal(&self, message: &str) -> EmitError {
        EmitError::Internal {
            anchor: self.module.into(),
            module: self.module,
            message: message.to_owned(),
        }
    }
}
