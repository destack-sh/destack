use std::collections::HashMap;

use destack_artifact::{
    FrameState, Function, Global, MirOptimized, Object, ObjectBuilder, Point, Type,
};
use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_program::{native, wasm};
use destack_source::ModuleId;

use crate::EmitError;

use super::point::PointIndex;
use super::site::Sites;

/// Emit the common linker input shared by every code form.
#[derive(Debug)]
pub struct ObjectEmitter {
    /// Common relocatable object state.
    object: ObjectBuilder,
    /// Common object type indices keyed by MIR identity.
    type_indices: HashMap<mir::TypeId, usize>,
    /// Common object function indices keyed by MIR identity.
    function_indices: HashMap<mir::FunctionId, usize>,
    /// Common object global indices keyed by MIR identity.
    global_indices: HashMap<mir::GlobalId, usize>,
    /// Common object program points keyed by MIR operation identity.
    points: PointIndex,
    /// Allocation indices keyed by object-local program point.
    allocation_indices: HashMap<Point, u32>,
}

impl ObjectEmitter {
    /// Collect common object state from optimized MIR.
    pub fn new(
        module: ModuleId,
        optimized: &MirOptimized,
        dependencies: impl IntoIterator<Item = ModuleId>,
    ) -> Result<Self, EmitError> {
        let mut type_indices = HashMap::new();
        let types = optimized.tree.iter_nodes::<mir::Type>().enumerate().map(
            |(index, (id, definition))| {
                type_indices.insert(id, index);

                Type {
                    id,
                    fingerprint: optimized.tree.type_fingerprint(id),
                    definition: definition.clone(),
                    symbol: optimized.types.symbol(id),
                    name: optimized.types.display_name(id),
                    lineage: optimized.types.lineage(id).cloned(),
                }
            },
        );
        let mut function_indices = HashMap::new();
        let functions = optimized
            .tree
            .iter_nodes::<mir::Function>()
            .enumerate()
            .map(|(index, (id, function))| {
                function_indices.insert(id, index);

                Function {
                    id,
                    name: function.name,
                    symbol: function.symbol,
                    linkage: function.linkage,
                    lifetimes: function.lifetimes.clone(),
                    parameters: function
                        .parameters
                        .iter()
                        .map(mir::FunctionParameter::signature_parameter)
                        .collect(),
                    result: function.return_type,
                    environment: function.environment,
                    binding: function.binding,
                    coroutine: function.coroutine,
                }
            });
        let mut global_indices = HashMap::new();
        let globals =
            optimized
                .tree
                .iter_nodes::<mir::Global>()
                .enumerate()
                .map(|(index, (id, global))| {
                    global_indices.insert(id, index);

                    Global {
                        id,
                        name: global.name,
                        symbol: global.symbol,
                        ty: global.ty,
                        mutability: global.mutability,
                        space: global.space,
                        linkage: global.linkage,
                        initializer: global.initializer.clone(),
                    }
                });
        let points = PointIndex::build(optimized);
        let sites = Sites::emit(module, optimized, &points)?;
        let allocation_indices = sites.allocation_indices;
        let object = ObjectBuilder::new(optimized.target)
            .dependencies(dependencies)
            .types(types)
            .layouts(optimized.layouts.clone())
            .drops(optimized.drops.clone())
            .dispatch(optimized.dispatch.clone())
            .functions(functions)
            .globals(globals)
            .allocations(sites.allocations)
            .memory(sites.memory)
            .calls(sites.calls)
            .resumes(sites.resumes)
            .edges(sites.edges)
            .suspensions(sites.suspensions)
            .counters(sites.counters)
            .samples(sites.samples);

        Ok(Self {
            object,
            type_indices,
            function_indices,
            global_indices,
            points,
            allocation_indices,
        })
    }

    /// Return the common object index assigned to one MIR type.
    pub(crate) fn type_index(&self, ty: mir::TypeId) -> Option<usize> {
        self.type_indices.get(&ty).copied()
    }

    /// Return the common object index assigned to one MIR function.
    pub(crate) fn function_index(&self, function: mir::FunctionId) -> Option<usize> {
        self.function_indices.get(&function).copied()
    }

    /// Return the common object index assigned to one MIR global.
    pub(crate) fn global_index(&self, global: mir::GlobalId) -> Option<usize> {
        self.global_indices.get(&global).copied()
    }

    /// Return one MIR instruction's object-local program point.
    pub(crate) fn instruction_point(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Point {
        self.points.instruction(instruction)
    }

    /// Return the allocation index assigned to one object-local program point.
    pub(crate) fn allocation_index(&self, point: Point) -> Option<u32> {
        self.allocation_indices.get(&point).copied()
    }

    /// Attach relocatable native code.
    pub fn native(mut self, code: native::Object) -> Self {
        self.object = self.object.native(code);

        self
    }

    /// Attach relocatable WebAssembly code.
    pub fn wasm(mut self, code: wasm::Object) -> Self {
        self.object = self.object.wasm(code);

        self
    }

    /// Build the object with its required bytecode.
    pub fn build(self, bytecode: bytecode::Object, frames: Vec<FrameState>) -> Object {
        self.object.frames(frames).build(bytecode)
    }

    /// Build one invalid object input diagnostic for a module.
    pub(super) fn invalid(module: ModuleId, message: &str) -> EmitError {
        EmitError::UnexpectedConstruct {
            anchor: module.into(),
            module,
            message: message.to_owned(),
        }
    }
}
