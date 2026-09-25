use std::collections::HashSet;

use tspp_bytecode as bytecode;
use tspp_mir as mir;
use tspp_program::{CounterId, Object, SamplerId, TypeId};
use tspp_source::ModuleId;

use crate::LinkResult;

use super::BytecodeLinker;

impl<'a, 'b> BytecodeLinker<'a, 'b> {
    /// Resolve one object's encoded identities into final Program ids.
    pub(super) fn link_code(&self, module: ModuleId, object: &Object) -> LinkResult<Vec<u8>> {
        let bytecode = self.object(object)?;
        let mut code = bytecode.code().to_vec();
        let counters = object
            .counters()
            .iter()
            .map(|site| (site.point.function, site.counter))
            .collect::<HashSet<_>>();
        let samplers = object
            .samples()
            .iter()
            .map(|site| (site.point.function, site.sampler))
            .collect::<HashSet<_>>();

        // patch every object-local identity at its exact encoded operand
        for relocation in bytecode.relocations() {
            let start = relocation.byte_offset as usize;
            let byte_len = size_of::<u32>();
            let end = start + byte_len;
            let bytes = code.get(start..end).ok_or_else(|| {
                self.program
                    .invalid_input("bytecode relocation is out of range")
            })?;
            let encoded = u32::from_le_bytes(bytes.try_into().map_err(|_| {
                self.program
                    .invalid_input("bytecode relocation has invalid width")
            })?);
            let linked = self.relocation(
                module,
                object,
                relocation.byte_offset,
                relocation.tag,
                encoded,
                &counters,
                &samplers,
            )?;
            let bytes = code.get_mut(start..end).ok_or_else(|| {
                self.program
                    .invalid_input("bytecode relocation is out of range")
            })?;
            bytes.copy_from_slice(&linked.to_le_bytes());
        }

        Ok(code)
    }

    /// Resolve one encoded object-local identity into its final Program id.
    pub(super) fn relocation(
        &self,
        module: ModuleId,
        object: &Object,
        byte_offset: u32,
        tag: bytecode::RelocationTag,
        index: u32,
        counters: &HashSet<(mir::FunctionId, mir::CounterId)>,
        samplers: &HashSet<(mir::FunctionId, mir::SamplerId)>,
    ) -> LinkResult<u32> {
        if tag == bytecode::RelocationTag::TYPE {
            self.type_id(module, object, index).map(|id| id.0)
        } else if tag == bytecode::RelocationTag::LAYOUT {
            let ty = self.object_type(object, index)?;

            Ok(self.program.layout_id(module, ty).raw())
        } else if tag == bytecode::RelocationTag::FUNCTION {
            let function = object.functions().get(index as usize).ok_or_else(|| {
                self.program
                    .invalid_input("bytecode function operand is absent")
            })?;

            Ok(self.program.function_id(module, function.id).0)
        } else if tag == bytecode::RelocationTag::GLOBAL {
            let global = object.globals().get(index as usize).ok_or_else(|| {
                self.program
                    .invalid_input("bytecode global operand is absent")
            })?;

            Ok(self.program.global_id(module, global.id).0)
        } else if tag == bytecode::RelocationTag::DYNAMIC {
            let table = object
                .dispatch()
                .iter_dynamic_tables()
                .nth(index as usize)
                .ok_or_else(|| {
                    self.program
                        .invalid_input("bytecode dynamic table is absent")
                })?;
            let id = self
                .program
                .dynamic_table_id(module, table.concrete, table.constraint)
                .ok_or_else(|| self.program.invalid_input("linked dynamic table is absent"))?;

            Ok(id.0)
        } else if tag == bytecode::RelocationTag::ALLOCATION {
            object.allocations().get(index as usize).ok_or_else(|| {
                self.program
                    .invalid_input("bytecode allocation site is absent")
            })?;

            Ok(self.program.allocation_id(module, index).0)
        } else if tag == bytecode::RelocationTag::COUNTER {
            let function = self.relocation_function(object, byte_offset)?;

            self.counter_id(module, counters, function, mir::CounterId(index))
                .map(|counter| counter.0)
        } else if tag == bytecode::RelocationTag::SAMPLER {
            let function = self.relocation_function(object, byte_offset)?;

            self.sampler_id(module, samplers, function, mir::SamplerId(index))
                .map(|sampler| sampler.0)
        } else {
            Err(self
                .program
                .invalid_input("unsupported bytecode relocation"))
        }
    }

    /// Return the MIR function that owns one object code offset.
    pub(super) fn relocation_function(
        &self,
        object: &Object,
        byte_offset: u32,
    ) -> LinkResult<mir::FunctionId> {
        for (declaration, function) in object
            .functions()
            .iter()
            .zip(self.object(object)?.functions())
        {
            let Some(code) = function.code() else {
                continue;
            };
            let end = code.byte_offset + code.byte_len;
            if byte_offset >= code.byte_offset && byte_offset < end {
                return Ok(declaration.id);
            }
        }

        Err(self
            .program
            .invalid_input("bytecode relocation has no owning function"))
    }

    /// Resolve one function-local counter declared by an artifact site.
    pub(super) fn counter_id(
        &self,
        module: ModuleId,
        counters: &HashSet<(mir::FunctionId, mir::CounterId)>,
        function: mir::FunctionId,
        counter: mir::CounterId,
    ) -> LinkResult<CounterId> {
        if !counters.contains(&(function, counter)) {
            return Err(self
                .program
                .invalid_input("bytecode counter site is absent"));
        }

        Ok(self.program.counter_id(module, function, counter))
    }

    /// Resolve one function-local sampler declared by an artifact site.
    pub(super) fn sampler_id(
        &self,
        module: ModuleId,
        samplers: &HashSet<(mir::FunctionId, mir::SamplerId)>,
        function: mir::FunctionId,
        sampler: mir::SamplerId,
    ) -> LinkResult<SamplerId> {
        if !samplers.contains(&(function, sampler)) {
            return Err(self
                .program
                .invalid_input("bytecode sampler site is absent"));
        }

        Ok(self.program.sampler_id(module, function, sampler))
    }

    /// Return one object-local MIR type identity.
    pub(super) fn object_type(&self, object: &Object, index: u32) -> LinkResult<mir::TypeId> {
        object
            .types()
            .get(index as usize)
            .map(|ty| ty.id)
            .ok_or_else(|| {
                self.program
                    .invalid_input("bytecode type operand is absent")
            })
    }

    /// Return one final Program type identity.
    pub(super) fn type_id(
        &self,
        module: ModuleId,
        object: &Object,
        index: u32,
    ) -> LinkResult<TypeId> {
        let ty = self.object_type(object, index)?;

        Ok(self.program.type_id(module, ty))
    }
}
