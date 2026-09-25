use tspp_artifact::MirOptimized;
use tspp_bytecode as bytecode;
use tspp_core::StringId;
use tspp_mir as mir;
use tspp_native as native;
use tspp_program::object::{FrameState, Function, Global, Point, Type};
use tspp_program::{Object, ObjectBuilder};
use tspp_source::ModuleId;
use tspp_webassembly as wasm;

#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
use tspp_program::object::FramePoint;

use crate::EmitError;

use super::frame::FrameEmitter;
use super::point::PointMap;
use super::site::SiteEmitter;

/// The attribute naming the language item a declaration answers.
const LANGUAGE_ITEM_ATTRIBUTE: &str = "languageItem";

/// Relocatable object emitter for optimized MIR.
#[derive(Debug)]
pub struct ObjectEmitter {
    /// Relocatable object under construction.
    object: ObjectBuilder,
    /// MIR types in object-local identity order.
    types: Vec<mir::TypeId>,
    /// MIR functions in object-local identity order.
    functions: Vec<mir::FunctionId>,
    /// MIR globals in object-local identity order.
    globals: Vec<mir::GlobalId>,
    /// Dynamic table identities sorted by concrete and constraint type.
    dynamics: Vec<(mir::TypeId, mir::TypeId, u32)>,
    /// Object program points keyed by MIR operation identity.
    points: PointMap,
    /// Engine-neutral logical frame states.
    frames: Vec<FrameState>,
    /// Allocation points in object-local identity order.
    allocation_points: Vec<Point>,
}

impl ObjectEmitter {
    /// Collect engine-neutral object entries from optimized MIR.
    pub fn new(
        module: ModuleId,
        optimized: &MirOptimized,
        dependencies: impl IntoIterator<Item = ModuleId>,
        analyses: &mut mir::ModuleCache,
    ) -> Result<Self, EmitError> {
        // assign stable object type identities
        let mut types = Vec::new();
        let mut type_ids = Vec::new();
        let tag = mir::AttributeIdentifier::Identifier(StringId::for_text(LANGUAGE_ITEM_ATTRIBUTE));
        for (id, definition) in optimized.tree.types() {
            type_ids.push(id);

            // read the name and language item off the declaration
            let declaration = optimized.tree.type_declaration(id);
            let name = declaration.and_then(|declaration| optimized.tree.get(declaration).name);
            let language_item = declaration.and_then(|declaration| {
                optimized
                    .tree
                    .attributes(declaration)
                    .iter()
                    .find_map(|attribute| {
                        if attribute.name != tag {
                            return None;
                        }

                        let mir::AttributeArgs::Value(mir::AttributeValue::String(key)) =
                            &attribute.args
                        else {
                            return None;
                        };

                        Some(*key)
                    })
            });
            types.push(Type {
                id,
                fingerprint: optimized.tree.type_fingerprint(id),
                definition: definition.clone(),
                symbol: optimized.tree.type_symbol(id),
                name,
                heritage: optimized
                    .tree
                    .type_heritage(id)
                    .filter(|heritage| !heritage.is_empty())
                    .cloned(),
                language_item,
            });
        }

        // assign stable object function identities
        let mut functions = Vec::new();
        let mut function_ids = Vec::new();
        for (id, function) in optimized.tree.iter_nodes::<mir::Function>() {
            if function.is_polymorphic() {
                continue;
            }
            function_ids.push(id);
            functions.push(Function {
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
                binding: function.binding.clone(),
            });
        }

        // assign stable object global identities
        let mut globals = Vec::new();
        let mut global_ids = Vec::new();
        for (id, global) in optimized.tree.iter_nodes::<mir::Global>() {
            global_ids.push(id);
            globals.push(Global {
                id,
                name: global.name,
                symbol: global.symbol,
                ty: global.ty,
                mutability: global.mutability,
                space: global.space,
                linkage: global.linkage,
                initializer: global.initializer.clone(),
            });
        }

        // assign stable object dynamic table identities
        let mut dynamics = optimized
            .dispatch
            .iter_dynamic_tables()
            .enumerate()
            .map(|(index, table)| (table.concrete, table.constraint, index as u32))
            .collect::<Vec<_>>();
        dynamics.sort_unstable_by_key(|(concrete, constraint, _)| (*concrete, *constraint));

        // collect the execution sites and frames under object-local identities
        let points = PointMap::build(optimized);
        let sites = SiteEmitter::emit(module, optimized, &points)?;
        let frames = FrameEmitter::new(module, optimized, &points, &sites).emit(analyses)?;
        let allocation_points = sites.allocations.iter().map(|site| site.point).collect();

        // build the object state shared by every execution form
        let object = ObjectBuilder::new(optimized.target)
            .dependencies(dependencies)
            .types(types)
            .layouts(optimized.layouts.clone())
            .drops(optimized.drops.clone())
            .dispatch(optimized.dispatch.clone())
            .functions(functions)
            .initializer(optimized.initializer)
            .globals(globals)
            .allocations(sites.allocations)
            .memory(sites.memory)
            .calls(sites.calls)
            .edges(sites.edges)
            .counters(sites.counters)
            .samples(sites.samples);

        Ok(Self {
            object,
            types: type_ids,
            functions: function_ids,
            globals: global_ids,
            dynamics,
            points,
            frames,
            allocation_points,
        })
    }

    /// Return the object index assigned to one MIR type.
    pub(crate) fn type_index(&self, ty: mir::TypeId) -> Option<usize> {
        self.types.binary_search(&ty).ok()
    }

    /// Return the object index assigned to one MIR function.
    pub(crate) fn function_index(&self, function: mir::FunctionId) -> Option<usize> {
        self.functions.binary_search(&function).ok()
    }

    /// Return MIR functions in object-local identity order.
    pub(crate) fn functions(&self) -> &[mir::FunctionId] {
        &self.functions
    }

    /// Return the object index assigned to one MIR global.
    pub(crate) fn global_index(&self, global: mir::GlobalId) -> Option<usize> {
        self.globals.binary_search(&global).ok()
    }

    /// Return the object index assigned to one dynamic implementation table.
    pub(crate) fn dynamic_index(
        &self,
        concrete: mir::TypeId,
        constraint: mir::TypeId,
    ) -> Option<u32> {
        self.dynamics
            .binary_search_by_key(&(concrete, constraint), |(concrete, constraint, _)| {
                (*concrete, *constraint)
            })
            .ok()
            .map(|index| self.dynamics[index].2)
    }

    /// Return one MIR instruction's object-local program point.
    pub(crate) fn instruction_point(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Point {
        self.points.instruction(instruction)
    }

    /// Return one MIR terminator's object-local program point.
    pub(crate) fn terminator_point(&self, block: mir::BlockId) -> Point {
        self.points.terminator(block)
    }

    /// Sort blocks into emitted operation order.
    pub(crate) fn order_blocks(&self, blocks: &mut [mir::BlockId]) {
        self.points.order_blocks(blocks);
    }

    /// Return engine-neutral logical frame states.
    pub(crate) fn frames(&self) -> &[FrameState] {
        &self.frames
    }

    /// Return the logical frame state at one operation point.
    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
    pub(crate) fn frame(&self, point: FramePoint) -> Option<&FrameState> {
        self.frames
            .binary_search_by_key(&point, |frame| frame.point)
            .ok()
            .and_then(|index| self.frames.get(index))
    }

    /// Return the object-local logical frame state index at one point.
    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
    pub(crate) fn frame_index(&self, point: FramePoint) -> Option<u32> {
        self.frames
            .binary_search_by_key(&point, |frame| frame.point)
            .ok()
            .map(|index| index as u32)
    }

    /// Return the allocation index assigned to one object-local program point.
    pub(crate) fn allocation_index(&self, point: Point) -> Option<u32> {
        self.allocation_points
            .binary_search(&point)
            .ok()
            .map(|index| index as u32)
    }

    /// Attach relocatable bytecode.
    pub fn bytecode(mut self, code: bytecode::Object) -> Self {
        self.object = self.object.bytecode(code);

        self
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

    /// Build the object with its selected execution forms.
    pub fn build(self) -> Object {
        self.object.frames(self.frames).build()
    }

    /// Return whether releasing one unique allocation destroys values, failing for other types.
    pub(in crate::emit) fn release_destroys(
        module: ModuleId,
        optimized: &MirOptimized,
        ty: mir::TypeId,
    ) -> Result<bool, EmitError> {
        optimized
            .drops
            .release_destroys(ty, &optimized.tree)
            .ok_or_else(|| Self::internal(module, "release of a value outside a unique reference"))
    }

    /// Build one internal object emission diagnostic.
    pub(in crate::emit) fn internal(module: ModuleId, message: &str) -> EmitError {
        EmitError::Internal {
            anchor: module.into(),
            module,
            message: message.to_owned(),
        }
    }
}
