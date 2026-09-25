use tspp_bytecode as bytecode;
use tspp_program::Object;

use crate::LinkResult;

use super::super::program::{FrameLinker, ProgramLinker};

/// Link relocatable bytecode objects into one executable code image.
#[derive(Debug)]
pub(crate) struct BytecodeLinker<'a, 'b> {
    /// Program linker owning final runtime identities.
    pub(super) program: &'a ProgramLinker<'b>,
    /// Canonical frame state projection.
    pub(super) frames: &'a FrameLinker<'b>,
}

impl<'a, 'b> BytecodeLinker<'a, 'b> {
    /// Create one bytecode linker.
    pub(crate) fn new(program: &'a ProgramLinker<'b>, frames: &'a FrameLinker<'b>) -> Self {
        Self { program, frames }
    }

    /// Link bytecode when every object carries that code form.
    pub(crate) fn link(&self) -> LinkResult<Option<bytecode::CodeBuilder>> {
        // inspect bytecode coverage across the complete Program
        let object_count = self.program.objects().len();
        let bytecode_count = self
            .program
            .objects()
            .iter()
            .filter(|(_, object)| object.bytecode().is_some())
            .count();

        // omit bytecode only when no object carries it
        if bytecode_count == 0 {
            return Ok(None);
        }

        // reject incomplete code forms
        if bytecode_count != object_count {
            return Err(self
                .program
                .invalid_input("only some linked objects contain bytecode"));
        }

        // relocate every object in stable link order
        let mut object_code = Vec::with_capacity(self.program.objects().len());
        for (module, object) in self.program.objects() {
            object_code.push(self.link_code(*module, object)?);
        }

        let (frames, registers) = self.link_frames()?;
        let mut functions = Vec::with_capacity(self.program.functions_by_id().len());
        let mut operations = Vec::new();
        let mut code = Vec::new();

        // place physical functions in dense Program function order
        for &(module, function) in self.program.functions_by_id() {
            let object_index = self.program.object_index(module);
            let object = self.program.object(module);
            let source = self.function(object, function)?;
            let linked = self.link_function(
                function,
                object,
                source,
                &object_code[object_index],
                &mut operations,
                &mut code,
            )?;
            functions.push(linked);
        }

        Ok(Some(
            bytecode::CodeBuilder::new()
                .functions(functions)
                .frames(frames)
                .registers(registers)
                .operations(operations)
                .code(code),
        ))
    }

    /// Return the bytecode emitted for one linked object.
    pub(super) fn object<'c>(&self, object: &'c Object) -> LinkResult<&'c bytecode::Object> {
        object
            .bytecode()
            .ok_or_else(|| self.program.invalid_input("object has no bytecode"))
    }
}
