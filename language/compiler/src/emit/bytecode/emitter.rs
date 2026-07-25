use destack_artifact::{FrameState, MirOptimized, Point};
use destack_bytecode as bytecode;
use destack_core::{EntryRange, Optional};
use destack_mir as mir;
use destack_source::ModuleId;

use crate::{EmitError, ObjectEmitter};

use super::{EmittedFunction, FunctionEmitter, TypeEmitter};

/// Emit one relocatable bytecode object from optimized MIR.
#[derive(Debug)]
pub struct BytecodeEmitter<'a> {
    /// Optimized MIR being emitted.
    optimized: &'a MirOptimized,
    /// Module owning the emitted MIR.
    module: ModuleId,
    /// Common object identity assignments.
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

    /// Emit one relocatable bytecode object and its logical frame states.
    pub fn emit(&self) -> Result<(bytecode::Object, Vec<FrameState>), EmitError> {
        let functions = self.ordered_functions()?;
        let mut logical_frames = Vec::new();
        let mut parameters = Vec::new();
        let mut frames = Vec::new();
        let mut registers = Vec::new();
        let mut operations = Vec::new();
        let mut relocations = Vec::new();
        let mut code = Vec::new();
        let mut rows = Vec::with_capacity(functions.len());

        // emit functions in their common object order
        for (function_id, function) in functions {
            if function.body.is_none() {
                rows.push(self.declaration(function, &mut parameters)?);

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
            .build()?;
            rows.push(Self::append(
                emitted,
                &mut logical_frames,
                &mut parameters,
                &mut frames,
                &mut registers,
                &mut operations,
                &mut relocations,
                &mut code,
            ));
        }

        let object = bytecode::ObjectBuilder::new()
            .functions(rows)
            .parameters(parameters)
            .frames(frames)
            .registers(registers)
            .operations(operations)
            .relocations(relocations)
            .code(code)
            .build();

        Ok((object, logical_frames))
    }

    /// Return MIR functions in common object identity order.
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

    /// Build one imported physical function declaration.
    fn declaration(
        &self,
        function: &mir::Function,
        parameters: &mut Vec<bytecode::Parameter>,
    ) -> Result<bytecode::Function, EmitError> {
        let mut register = u16::from(function.environment.is_some());
        let parameter_start = parameters.len() as u32;

        // lay imported parameters out in their physical entry order
        for parameter in &function.parameters {
            let ty = self.types.register_type(parameter.ty)?;
            let registers = bytecode::RegisterSpan::new(bytecode::RegisterId(register), ty.word_count());
            parameters.push(bytecode::Parameter::new(
                registers,
                self.types.type_id(parameter.ty)?,
            ));
            register += ty.word_count();
        }

        let parameter_count = parameters.len() as u32 - parameter_start;
        let result = self.types.type_id(function.return_type)?;

        Ok(bytecode::Function::new(
            Optional::none(),
            EntryRange::new(parameter_start, parameter_count),
            EntryRange::empty(),
            EntryRange::empty(),
            result,
            0,
            Self::coroutine(function.coroutine),
            0,
            0,
        ))
    }

    /// Append one emitted function and return its physical object row.
    #[allow(clippy::too_many_arguments)]
    fn append(
        emitted: EmittedFunction,
        logical_frames: &mut Vec<FrameState>,
        parameters: &mut Vec<bytecode::Parameter>,
        frames: &mut Vec<bytecode::FrameMap>,
        registers: &mut Vec<bytecode::RegisterSpan>,
        operations: &mut Vec<bytecode::CodeOffset>,
        relocations: &mut Vec<bytecode::Relocation>,
        code: &mut Vec<u8>,
    ) -> bytecode::Function {
        let EmittedFunction {
            function,
            coroutine,
            parameters: emitted_parameters,
            result,
            body,
            frames: emitted_frames,
        } = emitted;

        // append physical entry parameters
        let parameter_start = parameters.len() as u32;
        let parameter_count = emitted_parameters.len() as u32;
        parameters.extend(emitted_parameters);

        // append matching logical and physical frame maps
        let frame_start = frames.len() as u32;
        for frame in emitted_frames {
            let register_start = registers.len() as u32;
            let register_count = frame.registers.len() as u32;
            registers.extend(frame.registers);
            frames.push(bytecode::FrameMap::new(
                frame.code_offset,
                EntryRange::new(register_start, register_count),
            ));
            logical_frames.push(FrameState::new(
                Point::new(function, frame.operation),
                frame.types,
            ));
        }
        let frame_count = frames.len() as u32 - frame_start;

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
            EntryRange::new(parameter_start, parameter_count),
            EntryRange::new(frame_start, frame_count),
            EntryRange::new(operation_start, operation_count),
            result,
            body.register_count,
            Self::coroutine(coroutine),
            body.counter_count,
            body.sampler_count,
        )
    }

    /// Map one MIR coroutine form into the stable bytecode representation.
    fn coroutine(coroutine: Option<mir::Coroutine>) -> bytecode::Coroutine {
        match coroutine {
            None => bytecode::Coroutine::NONE,
            Some(mir::Coroutine::Async) => bytecode::Coroutine::ASYNC,
            Some(mir::Coroutine::Generator) => bytecode::Coroutine::GENERATOR,
            Some(mir::Coroutine::AsyncGenerator) => bytecode::Coroutine::ASYNC_GENERATOR,
        }
    }
}
