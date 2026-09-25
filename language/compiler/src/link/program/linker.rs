use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tspp_core::{StringId, StringPool};
use tspp_heap::DropId;
use tspp_mir as mir;
use tspp_program::{
    AllocationSiteId, CounterId, DropEntry, DynamicTableId, EntryPoint, FunctionId, GlobalId,
    LayoutId, Object, Program, ProgramBuilder, SamplerId, Signature, SignatureId, TypeId,
    VirtualTableId,
};
use tspp_source::{ModuleId, PackageId};

use crate::{LinkError, LinkResult};

use super::super::{BytecodeLinker, NativeLinker};
use super::{
    BindingLinker, DispatchLinker, FrameLinker, FunctionLinker, LayoutLinker, SiteLinker,
    StaticLinker, TypeLinker,
};

/// Build one Program from optimized module objects and an immutable string pool.
#[derive(Debug)]
pub struct ProgramLinker<'a> {
    /// The target root modules, the entry module first.
    roots: Vec<ModuleId>,
    /// Package that owns the linked program.
    package: PackageId,
    /// Module objects in stable link order.
    objects: Vec<(ModuleId, Arc<Object>)>,
    /// Object positions keyed by module id.
    object_ids: HashMap<ModuleId, usize>,
    /// Target ABI layout shared by every object.
    target_layout: mir::TargetLayout,
    /// Program string pool.
    strings: &'a StringPool,
    /// Dense program function ids keyed by module-local MIR function id.
    function_ids: HashMap<(ModuleId, mir::FunctionId), FunctionId>,
    /// Canonical MIR function declarations in program function order.
    functions_by_id: Vec<(ModuleId, mir::FunctionId)>,
    /// Canonical callable signatures in dense signature id order.
    signatures: Vec<Signature>,
    /// Dense signature ids keyed by module-local function id.
    function_signatures: HashMap<(ModuleId, mir::FunctionId), SignatureId>,
    /// Dense signature ids keyed by module-local MIR signature type id.
    type_signatures: HashMap<(ModuleId, mir::TypeId), SignatureId>,
    /// Dense program type ids keyed by module-local MIR type id.
    type_ids: HashMap<(ModuleId, mir::TypeId), TypeId>,
    /// Canonical MIR types in program type order.
    types_by_id: Vec<(ModuleId, mir::TypeId)>,
    /// Dense drop ids keyed by program type id.
    drop_ids: HashMap<TypeId, DropId>,
    /// Program destructors in dense drop id order.
    drops: Vec<DropEntry>,
    /// Dense virtual table ids keyed by canonical concrete type.
    virtual_table_ids: HashMap<TypeId, VirtualTableId>,
    /// Dense dynamic table ids keyed by canonical concrete and constraint types.
    dynamic_table_ids: HashMap<(TypeId, TypeId), DynamicTableId>,
    /// Dense program global ids keyed by module-local MIR global id.
    global_ids: HashMap<(ModuleId, mir::GlobalId), GlobalId>,
    /// Canonical MIR globals in program global order.
    globals_by_id: Vec<(ModuleId, mir::GlobalId)>,
    /// First dense counter id assigned to each canonical function.
    counter_starts: HashMap<(ModuleId, mir::FunctionId), CounterId>,
    /// First dense sampler id assigned to each canonical function.
    sampler_starts: HashMap<(ModuleId, mir::FunctionId), SamplerId>,
    /// First dense allocation site id assigned to each object.
    allocation_starts: HashMap<ModuleId, AllocationSiteId>,
}

impl<'a> ProgramLinker<'a> {
    /// Return the shared repository string pool.
    pub(crate) fn strings(&self) -> &StringPool {
        self.strings
    }

    /// Create one program linker.
    pub fn new(
        package: PackageId,
        objects: Vec<(ModuleId, Arc<Object>)>,
        strings: &'a StringPool,
    ) -> LinkResult<Self> {
        let object_ids = Self::object_ids(package, &objects)?;
        let target_layout = Self::common_layout(package, &objects)?;
        let (type_ids, types_by_id) = TypeLinker::index(&objects);
        let (function_ids, functions_by_id) =
            FunctionLinker::index(package, &objects, &type_ids, strings)?;
        let (signatures, function_signatures, type_signatures) =
            FunctionLinker::signatures(&objects, &type_ids);
        let (drop_ids, drops) = TypeLinker::drops(
            package,
            &objects,
            &type_ids,
            types_by_id.len(),
            &function_ids,
        )?;
        let virtual_table_ids = DispatchLinker::virtual_ids(package, &objects, &type_ids)?;
        let dynamic_table_ids = DispatchLinker::dynamic_ids(&objects, &type_ids);
        let (global_ids, globals_by_id) = StaticLinker::index(package, &objects, &type_ids)?;
        let (counter_starts, sampler_starts) =
            SiteLinker::profile_starts(package, &objects, &functions_by_id)?;
        let allocation_starts = SiteLinker::allocation_starts(&objects);

        Ok(Self {
            package,
            roots: Vec::new(),
            objects,
            object_ids,
            target_layout,
            strings,
            function_ids,
            functions_by_id,
            signatures,
            function_signatures,
            type_signatures,
            type_ids,
            types_by_id,
            drop_ids,
            drops,
            virtual_table_ids,
            dynamic_table_ids,
            global_ids,
            globals_by_id,
            counter_starts,
            sampler_starts,
            allocation_starts,
        })
    }

    /// Set the target root modules, the entry module first.
    pub fn with_roots(mut self, roots: Vec<ModuleId>) -> Self {
        self.roots = roots;

        self
    }

    /// Return the module initializers in dependency order, the roots' last.
    fn initializers(&self) -> Vec<EntryPoint> {
        // walk dependencies before their dependents, roots in their given order
        let roots: Vec<ModuleId> = match self.roots.is_empty() {
            true => self.objects.iter().map(|(module, _)| *module).collect(),
            false => self.roots.clone(),
        };
        let mut visited = HashSet::new();
        let mut ordered = Vec::new();
        let mut stack: Vec<(ModuleId, bool)> =
            roots.iter().rev().map(|root| (*root, false)).collect();
        while let Some((module, expanded)) = stack.pop() {
            if expanded {
                ordered.push(module);
                continue;
            }
            if !visited.insert(module) {
                continue;
            }
            stack.push((module, true));
            for dependency in self.object(module).dependencies().iter().rev() {
                if !visited.contains(dependency) {
                    stack.push((*dependency, false));
                }
            }
        }

        // keep the initializer each ordered module declares
        ordered
            .into_iter()
            .filter_map(|module| {
                let initializer = self.object(module).initializer()?;

                Some(EntryPoint::from(self.function_id(module, initializer)))
            })
            .collect()
    }

    /// Link the program.
    pub fn link(self) -> LinkResult<Program> {
        // project engine-neutral program tables
        let mut frame_linker = FrameLinker::new(&self);
        let frames = frame_linker.link()?;
        let statics = StaticLinker::new(&self).link()?;
        let layouts = LayoutLinker::new(&self).link()?;
        let functions = FunctionLinker::new(&self).link()?;
        let bindings = BindingLinker::new(&self).link()?;
        let sites = SiteLinker::new(&self).link()?;
        let types = TypeLinker::new(&self).link()?;
        let dispatch = DispatchLinker::new(&self).link()?;

        // link each explicitly emitted execution form
        let bytecode = BytecodeLinker::new(&self, &frame_linker).link()?;
        let native = NativeLinker::new(&self, &frame_linker, &statics).link()?;

        // assemble the durable program image
        let package = self.package;
        let initializers = self.initializers();
        let mut program = ProgramBuilder::new(self.target_layout)
            .strings(self.strings, self.string_ids()?)
            .types(types)
            .drops(self.drops)
            .initializers(initializers)
            .layouts(layouts.layouts)
            .frames(frames)
            .functions(functions)
            .bindings(bindings)
            .dispatch(dispatch)
            .sites(sites)
            .traces(layouts.traces)
            .globals(statics.globals)
            .constants(statics.constants)
            .shared_statics(statics.shared)
            .local_statics(statics.local);

        // attach the bytecode form when the package emits it
        if let Some(bytecode) = bytecode {
            program = program.bytecode(bytecode);
        }

        // attach the native form when the package emits it
        if let Some(native) = native {
            program = program.native(native);
        }

        // seal the image
        let program = program.build().map_err(|error| LinkError::InvalidInput {
            anchor: package.into(),
            package,
            context: error.to_string(),
        })?;

        Ok(program)
    }

    /// Return string ids retained by the linked program.
    fn string_ids(&self) -> LinkResult<Vec<StringId>> {
        let mut ids = Vec::new();

        // retain canonical function and export names
        for (module, function_id) in &self.functions_by_id {
            let function = self
                .object(*module)
                .function(*function_id)
                .ok_or_else(|| self.invalid_input(format!("missing function {function_id:?}")))?;
            ids.push(function.name);
            if let Some(binding) = &function.binding {
                ids.push(binding.name);
                ids.extend(binding.requires.iter().copied());
                ids.extend(binding.platforms.iter().copied());
                ids.extend(binding.families.iter().copied());
                ids.extend(binding.hosts.iter().copied());
            }
        }

        // retain names stored in linked layout entries
        for (_, object) in &self.objects {
            for ty in object.types() {
                let Some(layout) = object.layouts().type_layout(ty.id) else {
                    continue;
                };
                ids.extend(layout.shape.fields().iter().filter_map(|field| field.name));
            }
        }

        // retain names stored in linked dynamic dispatch
        for (_, object) in &self.objects {
            // retain the named slots of each dynamic shape
            for shape in object.dispatch().iter_dynamic_shapes() {
                ids.extend(shape.slots.iter().filter_map(|slot| match slot {
                    mir::DynamicSlot::Field { name, .. } => Some(*name),
                    mir::DynamicSlot::Function { name, .. } => *name,
                }));
            }

            // retain the entry names of each dynamic table
            for table in object.dispatch().iter_dynamic_tables() {
                ids.extend(table.names.iter().map(|entry| entry.name));
            }
        }

        // retain native target identity
        for (_, object) in &self.objects {
            let Some(native) = object.native() else {
                continue;
            };
            ids.push(self.strings.intern(native.target()));
            ids.extend(
                native
                    .features()
                    .map(|feature| self.strings.intern(feature)),
            );
        }

        Ok(ids)
    }

    /// Return one invalid program input diagnostic.
    pub(crate) fn invalid_input(&self, context: impl Into<String>) -> LinkError {
        LinkError::InvalidInput {
            anchor: self.package.into(),
            package: self.package,
            context: context.into(),
        }
    }

    /// Return one link type mismatch diagnostic.
    pub(crate) fn type_mismatch(
        &self,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> LinkError {
        LinkError::TypeMismatch {
            anchor: self.package.into(),
            package: self.package,
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Return one unsupported zero initializer diagnostic.
    pub(crate) fn unsupported_zero_initializer(&self, ty: impl Into<String>) -> LinkError {
        LinkError::UnsupportedZeroInitializer {
            anchor: self.package.into(),
            package: self.package,
            ty: ty.into(),
        }
    }

    /// Return one layout overflow diagnostic.
    pub(crate) fn layout_overflow(&self, context: impl Into<String>) -> LinkError {
        LinkError::LayoutOverflow {
            anchor: self.package.into(),
            package: self.package,
            context: context.into(),
        }
    }

    /// Return module objects in stable link order.
    pub(crate) fn objects(&self) -> &[(ModuleId, Arc<Object>)] {
        &self.objects
    }

    /// Return one module object by module id.
    pub(crate) fn object(&self, module: ModuleId) -> &Object {
        let index = self.object_ids[&module];

        &self.objects[index].1
    }

    /// Return one module object's stable link position.
    pub(crate) fn object_index(&self, module: ModuleId) -> usize {
        self.object_ids[&module]
    }

    /// Return the program function id for one module-local MIR function.
    pub(crate) fn function_id(&self, module: ModuleId, function: mir::FunctionId) -> FunctionId {
        self.function_ids[&(module, function)]
    }

    /// Return the program drop id for one module-local MIR type when present.
    pub(crate) fn drop_id(&self, module: ModuleId, ty: mir::TypeId) -> Option<DropId> {
        let ty = self.type_id(module, ty);

        self.drop_ids.get(&ty).copied()
    }

    /// Return MIR functions in dense program function order.
    pub(crate) fn functions_by_id(&self) -> &[(ModuleId, mir::FunctionId)] {
        &self.functions_by_id
    }

    /// Return canonical callable signatures in dense id order.
    pub(crate) fn signatures(&self) -> &[Signature] {
        &self.signatures
    }

    /// Return the signature id for one module-local function.
    pub(crate) fn function_signature_id(
        &self,
        module: ModuleId,
        function: mir::FunctionId,
    ) -> SignatureId {
        self.function_signatures[&(module, function)]
    }

    /// Return the signature id for one module-local signature type.
    pub(crate) fn type_signature_id(
        &self,
        module: ModuleId,
        ty: mir::TypeId,
    ) -> Option<SignatureId> {
        self.type_signatures.get(&(module, ty)).copied()
    }

    /// Return one linked string.
    pub(crate) fn string(&self, string: StringId) -> &str {
        self.strings.get(string)
    }

    /// Intern one Program string.
    pub(crate) fn intern_string(&self, string: &str) -> StringId {
        self.strings.intern(string)
    }

    /// Return the linked target ABI layout.
    pub(crate) const fn target_layout(&self) -> mir::TargetLayout {
        self.target_layout
    }

    /// Return the program type id for one module-local MIR type.
    pub fn type_id(&self, module: ModuleId, ty: mir::TypeId) -> TypeId {
        self.type_ids[&(module, ty)]
    }

    /// Return the program layout id for one module-local MIR type.
    pub(crate) fn layout_id(&self, module: ModuleId, ty: mir::TypeId) -> LayoutId {
        let ty = self.type_id(module, ty);

        // layout ids are nonzero, so they sit one above their zero-based type ids
        LayoutId::new(ty.0 + 1)
    }

    /// Return whether one module-local MIR type has runtime identity.
    pub(crate) fn has_type(&self, module: ModuleId, ty: mir::TypeId) -> bool {
        self.type_ids.contains_key(&(module, ty))
    }

    /// Return the virtual table id for one module-local concrete type.
    pub(crate) fn virtual_table_id(
        &self,
        module: ModuleId,
        concrete: mir::TypeId,
    ) -> Option<VirtualTableId> {
        let concrete = self.type_id(module, concrete);

        self.virtual_table_ids.get(&concrete).copied()
    }

    /// Return the dynamic table id for one concrete type and constraint.
    pub(crate) fn dynamic_table_id(
        &self,
        module: ModuleId,
        concrete: mir::TypeId,
        constraint: mir::TypeId,
    ) -> Option<DynamicTableId> {
        let concrete = self.type_id(module, concrete);
        let constraint = self.type_id(module, constraint);

        self.dynamic_table_ids.get(&(concrete, constraint)).copied()
    }

    /// Return MIR types in dense program type id order.
    pub(crate) fn types_by_id(&self) -> &[(ModuleId, mir::TypeId)] {
        &self.types_by_id
    }

    /// Return the program global id for one module-local MIR global.
    pub(crate) fn global_id(&self, module: ModuleId, global: mir::GlobalId) -> GlobalId {
        self.global_ids[&(module, global)]
    }

    /// Return MIR globals in dense program global id order.
    pub(crate) fn globals_by_id(&self) -> &[(ModuleId, mir::GlobalId)] {
        &self.globals_by_id
    }

    /// Return the first dense counter id assigned to one function.
    pub(crate) fn counter_start(&self, module: ModuleId, function: mir::FunctionId) -> CounterId {
        self.counter_starts[&(module, function)]
    }

    /// Return one dense counter id from its function-local identity.
    pub(crate) fn counter_id(
        &self,
        module: ModuleId,
        function: mir::FunctionId,
        counter: mir::CounterId,
    ) -> CounterId {
        CounterId(self.counter_start(module, function).0 + counter.0)
    }

    /// Return the first dense sampler id assigned to one function.
    pub(crate) fn sampler_start(&self, module: ModuleId, function: mir::FunctionId) -> SamplerId {
        self.sampler_starts[&(module, function)]
    }

    /// Return one dense sampler id from its function-local identity.
    pub(crate) fn sampler_id(
        &self,
        module: ModuleId,
        function: mir::FunctionId,
        sampler: mir::SamplerId,
    ) -> SamplerId {
        SamplerId(self.sampler_start(module, function).0 + sampler.0)
    }

    /// Return one dense allocation id from its object-local identity.
    pub(crate) fn allocation_id(&self, module: ModuleId, allocation: u32) -> AllocationSiteId {
        AllocationSiteId(self.allocation_starts[&module].0 + allocation)
    }

    /// Build module lookups for the object sequence.
    fn object_ids(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
    ) -> LinkResult<HashMap<ModuleId, usize>> {
        let mut ids = HashMap::with_capacity(objects.len());

        // assign each module exactly one object position
        for (index, (module, _)) in objects.iter().enumerate() {
            if ids.insert(*module, index).is_some() {
                return Err(LinkError::invalid_input(
                    package,
                    format!("module {module:?} has multiple objects"),
                ));
            }
        }

        // require the complete dependency closure
        for (module, object) in objects {
            for dependency in object.dependencies() {
                if !ids.contains_key(dependency) {
                    return Err(LinkError::invalid_input(
                        package,
                        format!("module {module:?} requires missing module {dependency:?}"),
                    ));
                }
            }
        }

        Ok(ids)
    }

    /// Resolve the target layout shared by all module objects.
    fn common_layout(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
    ) -> LinkResult<mir::TargetLayout> {
        let Some((_, first)) = objects.first() else {
            return Err(LinkError::invalid_input(
                package,
                "Program has no module objects",
            ));
        };

        // read the ABI of the first object as the shared one
        let target = first.target();

        // require one ABI across every linked module
        for (module, object) in &objects[1..] {
            if object.target() != target {
                return Err(LinkError::invalid_input(
                    package,
                    format!("module {module:?} uses a different target layout"),
                ));
            }
        }

        Ok(target)
    }
}
