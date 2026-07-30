use std::collections::HashMap;
use std::sync::Arc;

use artifact::{EmitFormat, Object};
use destack_artifact as artifact;
use destack_core::{Optional, StringId, StringPool};
use destack_heap::DropId;
use destack_mir as mir;
use destack_program::{
    AllocationSiteId, CounterId, DropEntry, DynamicTableId, FunctionId, GlobalId, LayoutId,
    Program, ProgramBuilder, SamplerId, Signature, SignatureId, TypeId, VirtualTableId,
};
use destack_source::{ModuleId, PackageId, TargetId};

use crate::{LinkError, LinkResult};

use super::{
    BindingLinker, BytecodeLinker, DispatchLinker, FrameLinker, FunctionLinker, LayoutLinker,
    SiteLinker, StaticLinker, TypeLinker,
};

/// Build one Program from optimized module objects and an immutable string pool.
#[derive(Debug)]
pub struct ProgramLinker<'a> {
    /// Package that owns the linked program.
    package: PackageId,
    /// Target that owns the linked program.
    target: TargetId,
    /// Program format selected by the target.
    format: EmitFormat,
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
    /// Create one program linker.
    pub fn new(
        package: PackageId,
        target: TargetId,
        format: EmitFormat,
        objects: Vec<(ModuleId, Arc<Object>)>,
        strings: &'a StringPool,
    ) -> LinkResult<Self> {
        let object_ids = Self::build_object_ids(package, &objects)?;
        let target_layout = Self::target_layout(package, &objects)?;
        let (type_ids, types_by_id) = Self::build_type_ids(&objects);
        let (function_ids, functions_by_id) =
            Self::build_function_ids(package, &objects, &type_ids, strings)?;
        let (signatures, function_signatures, type_signatures) =
            Self::build_signatures(&objects, &type_ids);
        let (drop_ids, drops) = Self::build_drops(
            package,
            &objects,
            &type_ids,
            types_by_id.len(),
            &function_ids,
        )?;
        let virtual_table_ids = Self::build_virtual_table_ids(package, &objects, &type_ids)?;
        let dynamic_table_ids = Self::build_dynamic_table_ids(&objects, &type_ids);
        let (global_ids, globals_by_id) = Self::build_global_ids(package, &objects, &type_ids)?;
        let (counter_starts, sampler_starts) =
            Self::build_profile_starts(package, &objects, &functions_by_id)?;
        let allocation_starts = Self::build_allocation_starts(&objects);

        Ok(Self {
            package,
            target,
            format,
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

    /// Link the program.
    pub fn link(self) -> LinkResult<Program> {
        // reject Program formats whose generator is unavailable
        match self.format {
            EmitFormat::Bytecode => {}
            EmitFormat::Wasm | EmitFormat::Native => {
                return Err(LinkError::CodeGenerationUnavailable {
                    anchor: self.package.into(),
                    package: self.package,
                    target: self.target,
                    format: self.format.canonical_tag().to_string(),
                });
            }
            EmitFormat::Js => {
                return Err(LinkError::InvalidTarget {
                    anchor: self.package.into(),
                    package: self.package,
                    target: self.target,
                    message: "script targets do not produce Program artifacts".to_string(),
                });
            }
        }

        // project engine-neutral program tables
        let mut frame_linker = FrameLinker::new(&self);
        let frames = frame_linker.link()?;
        let bytecode_linker = BytecodeLinker::new(&self, &frame_linker);
        let bytecode = bytecode_linker.link()?;
        let layouts = LayoutLinker::new(&self).link()?;
        let functions = FunctionLinker::new(&self).link()?;
        let bindings = BindingLinker::new(&self).link()?;
        let sites = SiteLinker::new(&self, &frame_linker).link()?;
        let types = TypeLinker::new(&self).link()?;
        let statics = StaticLinker::new(&self).link()?;
        let dispatch = DispatchLinker::new(&self).link()?;

        // assemble the durable program image
        let program = ProgramBuilder::new(self.target_layout)
            .bytecode(bytecode)
            .strings(self.strings, self.string_ids()?)
            .types(types)
            .drops(self.drops)
            .layouts(layouts.layouts)
            .frames(frames)
            .functions(functions)
            .bindings(bindings)
            .dispatch(dispatch)
            .sites(sites)
            .traces(layouts.traces)
            .globals(statics.globals)
            .constant_space(statics.constants)
            .shared_static_space(statics.shared)
            .local_static_space(statics.local)
            .build();

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

        // retain names stored in linked layout rows
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
            for shape in object.dispatch().iter_dynamic_shapes() {
                ids.extend(shape.slots.iter().filter_map(|slot| match slot {
                    mir::DynamicSlot::Field { name, .. } => Some(*name),
                    mir::DynamicSlot::Function { name, .. } => *name,
                }));
            }

            for table in object.dispatch().iter_dynamic_tables() {
                ids.extend(table.names.iter().map(|entry| entry.name));
            }
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

    /// Return the program type id for one module-local MIR type.
    pub fn type_id(&self, module: ModuleId, ty: mir::TypeId) -> TypeId {
        self.type_ids[&(module, ty)]
    }

    /// Return the program layout id for one module-local MIR type.
    pub(crate) fn layout_id(&self, module: ModuleId, ty: mir::TypeId) -> LayoutId {
        let ty = self.type_id(module, ty);

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

    /// Build first allocation site ids for each object.
    fn build_allocation_starts(
        objects: &[(ModuleId, Arc<Object>)],
    ) -> HashMap<ModuleId, AllocationSiteId> {
        let mut starts = HashMap::with_capacity(objects.len());
        let mut next = 0;

        // assign each object's contiguous allocation site range
        for (module, object) in objects {
            starts.insert(*module, AllocationSiteId(next));
            next += object.allocations().len() as u32;
        }

        starts
    }

    /// Build module lookups for the object sequence.
    fn build_object_ids(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
    ) -> LinkResult<HashMap<ModuleId, usize>> {
        let mut ids = HashMap::with_capacity(objects.len());

        // assign each module exactly one object position
        for (index, (module, _)) in objects.iter().enumerate() {
            if ids.insert(*module, index).is_some() {
                return Err(Self::invalid_input_for(
                    package,
                    format!("module {module:?} has multiple objects"),
                ));
            }
        }

        // require the complete dependency closure
        for (module, object) in objects {
            for dependency in object.dependencies() {
                if !ids.contains_key(dependency) {
                    return Err(Self::invalid_input_for(
                        package,
                        format!("module {module:?} requires missing module {dependency:?}"),
                    ));
                }
            }
        }

        Ok(ids)
    }

    /// Resolve the target layout shared by all module objects.
    fn target_layout(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
    ) -> LinkResult<mir::TargetLayout> {
        let Some((_, first)) = objects.first() else {
            return Err(Self::invalid_input_for(
                package,
                "Program has no module objects",
            ));
        };
        let target = first.target();

        // require one ABI across every linked module
        for (module, object) in &objects[1..] {
            if object.target() != target {
                return Err(Self::invalid_input_for(
                    package,
                    format!("module {module:?} uses a different target layout"),
                ));
            }
        }

        Ok(target)
    }

    /// Build dense function ids from definitions and imported symbols.
    fn build_function_ids(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
        strings: &StringPool,
    ) -> LinkResult<(
        HashMap<(ModuleId, mir::FunctionId), FunctionId>,
        Vec<(ModuleId, mir::FunctionId)>,
    )> {
        let mut ids = HashMap::new();
        let mut functions = Vec::new();
        let mut symbols = HashMap::new();
        let mut bindings = HashMap::new();

        // assign every local and exported definition
        for (module, object) in objects {
            for function in object.functions() {
                let function_id = function.id;
                if function.linkage.is_import() {
                    continue;
                }

                let id = FunctionId::from(functions.len() as u32);
                if function.linkage.is_exported() {
                    let definition = (id, *module, function);
                    if symbols.insert(function.symbol, definition).is_some() {
                        return Err(Self::invalid_input_for(
                            package,
                            format!(
                                "function symbol {:?} has multiple definitions",
                                function.symbol
                            ),
                        ));
                    }
                }

                ids.insert((*module, function_id), id);
                functions.push((*module, function_id));

                // register each program-defined binding implementation once
                if let Some(binding) = &function.binding {
                    let definition = (id, *module, function);
                    if bindings.insert(binding.name, definition).is_some() {
                        let binding = strings.get(binding.name);
                        return Err(Self::invalid_input_for(
                            package,
                            format!("binding '{binding}' has multiple definitions"),
                        ));
                    }
                }
            }
        }

        // assign host bindings as external definitions
        for (module, object) in objects {
            for function in object.functions() {
                let function_id = function.id;
                let Some(binding) = &function.binding else {
                    continue;
                };
                if !function.linkage.is_import() {
                    continue;
                }

                let id = match bindings.get(&binding.name).copied() {
                    Some((id, definition_module, definition)) => {
                        if !Self::function_signatures_match(
                            *module,
                            function,
                            definition_module,
                            definition,
                            type_ids,
                        ) {
                            let binding = strings.get(binding.name);
                            return Err(Self::invalid_input_for(
                                package,
                                format!("binding '{binding}' has conflicting declarations"),
                            ));
                        }
                        if definition.binding.as_deref() != Some(binding.as_ref()) {
                            let binding = strings.get(binding.name);
                            return Err(Self::invalid_input_for(
                                package,
                                format!("binding '{binding}' has conflicting declarations"),
                            ));
                        }

                        id
                    }
                    None => {
                        let id = FunctionId::from(functions.len() as u32);
                        bindings.insert(binding.name, (id, *module, function));
                        functions.push((*module, function_id));
                        id
                    }
                };
                ids.insert((*module, function_id), id);
            }
        }

        // resolve remaining imports against exported definitions
        for (module, object) in objects {
            for function in object.functions() {
                let function_id = function.id;
                if !function.linkage.is_import() || function.binding.is_some() {
                    continue;
                }
                let Some((id, definition_module, definition)) =
                    symbols.get(&function.symbol).copied()
                else {
                    return Err(Self::invalid_input_for(
                        package,
                        format!("function symbol {:?} is undefined", function.symbol),
                    ));
                };
                if !Self::function_signatures_match(
                    *module,
                    function,
                    definition_module,
                    definition,
                    type_ids,
                ) {
                    return Err(Self::invalid_input_for(
                        package,
                        format!(
                            "function symbol {:?} has conflicting signatures",
                            function.symbol
                        ),
                    ));
                }

                ids.insert((*module, function_id), id);
            }
        }

        Ok((ids, functions))
    }

    /// Return whether two functions have the same callable signature.
    fn function_signatures_match(
        left_module: ModuleId,
        left: &artifact::Function,
        right_module: ModuleId,
        right: &artifact::Function,
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> bool {
        if left.lifetimes.len() != right.lifetimes.len()
            || left.parameters.len() != right.parameters.len()
        {
            return false;
        }

        // compare parameter types
        for (left_parameter, right_parameter) in left.parameters.iter().zip(&right.parameters) {
            if !Self::types_match(
                left_module,
                left_parameter.ty,
                right_module,
                right_parameter.ty,
                type_ids,
            ) {
                return false;
            }
        }

        // compare result and optional closure environment types
        let result_matches = Self::types_match(
            left_module,
            left.result,
            right_module,
            right.result,
            type_ids,
        );
        let environment_matches = match (left.environment, right.environment) {
            (Some(left), Some(right)) => {
                Self::types_match(left_module, left, right_module, right, type_ids)
            }
            (None, None) => true,
            _ => false,
        };

        result_matches && environment_matches
    }

    /// Return whether two module-local types denote the same declaration type.
    fn types_match(
        left_module: ModuleId,
        left: mir::TypeId,
        right_module: ModuleId,
        right: mir::TypeId,
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> bool {
        let left_id = type_ids[&(left_module, left)];
        let right_id = type_ids[&(right_module, right)];

        left_id == right_id
    }

    /// Build dense type ids from nominal identity and anonymous structure.
    fn build_type_ids(
        objects: &[(ModuleId, Arc<Object>)],
    ) -> (
        HashMap<(ModuleId, mir::TypeId), TypeId>,
        Vec<(ModuleId, mir::TypeId)>,
    ) {
        let mut ids = HashMap::new();
        let mut types = Vec::new();
        let mut fingerprints = HashMap::new();

        // canonicalize nominal identities and anonymous structural types
        for (module, object) in objects {
            for ty in object.types() {
                if object.layouts().layout_id(ty.id).is_none() {
                    continue;
                }

                let id = *fingerprints.entry(ty.fingerprint).or_insert_with(|| {
                    let id = TypeId::from(types.len() as u32);
                    types.push((*module, ty.id));
                    id
                });

                ids.insert((*module, ty.id), id);
            }
        }

        (ids, types)
    }

    /// Build one canonical signature table for declarations and signature types.
    fn build_signatures(
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> (
        Vec<Signature>,
        HashMap<(ModuleId, mir::FunctionId), SignatureId>,
        HashMap<(ModuleId, mir::TypeId), SignatureId>,
    ) {
        let mut signatures = Vec::new();
        let mut function_signatures = HashMap::new();
        let mut type_signatures = HashMap::new();

        // assign callable declarations first in stable object order
        for (module, object) in objects {
            for function in object.functions() {
                let signature = Signature {
                    parameters: function
                        .parameters
                        .iter()
                        .map(|parameter| type_ids[&(*module, parameter.ty)])
                        .collect(),
                    result: type_ids[&(*module, function.result)],
                };
                let id = Self::insert_signature(&mut signatures, signature);
                function_signatures.insert((*module, function.id), id);
            }
        }

        // assign explicit signature types through the same canonical table
        for (module, object) in objects {
            for ty in object.types() {
                let mir::Type::FunctionSignature {
                    parameters, result, ..
                } = &ty.definition
                else {
                    continue;
                };
                let signature = Signature {
                    parameters: parameters
                        .iter()
                        .map(|parameter| type_ids[&(*module, parameter.ty)])
                        .collect(),
                    result: type_ids[&(*module, *result)],
                };
                let id = Self::insert_signature(&mut signatures, signature);
                type_signatures.insert((*module, ty.id), id);
            }
        }

        (signatures, function_signatures, type_signatures)
    }

    /// Insert one canonical signature or return its existing dense id.
    fn insert_signature(signatures: &mut Vec<Signature>, signature: Signature) -> SignatureId {
        let index = signatures
            .iter()
            .position(|existing| *existing == signature)
            .unwrap_or_else(|| {
                signatures.push(signature);

                signatures.len() - 1
            });

        SignatureId(index as u32)
    }

    /// Build dense drop identities and destructor functions in program type order.
    fn build_drops(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
        type_count: usize,
        function_ids: &HashMap<(ModuleId, mir::FunctionId), FunctionId>,
    ) -> LinkResult<(HashMap<TypeId, DropId>, Vec<DropEntry>)> {
        let mut functions = HashMap::new();

        // resolve every specialized destructor into canonical program identity
        for (module, object) in objects {
            for (&(ty, storage), &function) in &object.drops().destructors {
                if matches!(storage, mir::Storage::Global(_)) {
                    return Err(Self::invalid_input_for(
                        package,
                        format!("type {ty:?} defines a destructor for global storage"),
                    ));
                }

                let ty = type_ids[&(*module, ty)];
                let function_id = function_ids[&(*module, function)];
                let key = (ty, storage);

                if let Some((previous_module, previous_function)) = functions.get(&key) {
                    let previous_id = function_ids[&(*previous_module, *previous_function)];
                    if previous_id != function_id {
                        return Err(Self::invalid_input_for(
                            package,
                            format!("type {ty:?} has multiple {storage:?} destructors"),
                        ));
                    }
                } else {
                    functions.insert(key, (*module, function));
                }
            }
        }

        // assign drop ids in canonical program type order
        let mut ids = HashMap::new();
        let mut drops = Vec::new();
        for index in 0..type_count {
            let ty = TypeId::from(index as u32);
            let frame = functions
                .remove(&(ty, mir::Storage::Frame))
                .map(|(module, function)| function_ids[&(module, function)]);
            let local = functions
                .remove(&(ty, mir::Storage::Heap(mir::Space::Local)))
                .map(|(module, function)| function_ids[&(module, function)]);
            let shared = functions
                .remove(&(ty, mir::Storage::Heap(mir::Space::Shared)))
                .map(|(module, function)| function_ids[&(module, function)]);
            if frame.is_none() && local.is_none() && shared.is_none() {
                continue;
            }

            ids.insert(ty, DropId::from_index(drops.len() as u32));
            drops.push(DropEntry {
                frame: Optional::from(frame),
                local: Optional::from(local),
                shared: Optional::from(shared),
            });
        }

        Ok((ids, drops))
    }

    /// Build dense virtual table ids from canonical concrete types.
    fn build_virtual_table_ids(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> LinkResult<HashMap<TypeId, VirtualTableId>> {
        let mut ids = HashMap::new();

        // require every virtual receiver to expose its dispatch id
        for (module, object) in objects {
            for table in object.dispatch().iter_virtual_tables() {
                let layout = object
                    .layouts()
                    .type_layout(table.concrete)
                    .ok_or_else(|| {
                        Self::invalid_input_for(
                            package,
                            format!("missing layout for virtual type {:?}", table.concrete),
                        )
                    })?;
                let mir::LayoutShape::Object(object_layout) = &layout.shape else {
                    return Err(Self::invalid_input_for(
                        package,
                        format!(
                            "virtual type {:?} does not have an object layout",
                            table.concrete
                        ),
                    ));
                };
                let Some(dispatch_offset) = object_layout.dispatch_offset else {
                    return Err(Self::invalid_input_for(
                        package,
                        format!("virtual type {:?} has no dispatch offset", table.concrete),
                    ));
                };
                let dispatch_end = u64::from(dispatch_offset) + size_of::<u32>() as u64;
                if dispatch_end > u64::from(layout.size) {
                    return Err(Self::invalid_input_for(
                        package,
                        format!(
                            "virtual type {:?} has a dispatch offset outside its layout",
                            table.concrete
                        ),
                    ));
                }

                let concrete = type_ids[&(*module, table.concrete)];
                let next = VirtualTableId(ids.len() as u32);
                ids.entry(concrete).or_insert(next);
            }
        }

        // require every dispatch field to have one canonical virtual table
        for (module, object) in objects {
            for ty in object.types() {
                let Some(layout) = object.layouts().type_layout(ty.id) else {
                    continue;
                };
                let mir::LayoutShape::Object(layout) = &layout.shape else {
                    continue;
                };
                if layout.dispatch_offset.is_none() {
                    continue;
                }

                let concrete = type_ids[&(*module, ty.id)];
                if !ids.contains_key(&concrete) {
                    return Err(Self::invalid_input_for(
                        package,
                        format!("object type {:?} has no virtual table", ty.id),
                    ));
                }
            }
        }

        Ok(ids)
    }

    /// Build dense dynamic table ids from canonical concrete and constraint types.
    fn build_dynamic_table_ids(
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> HashMap<(TypeId, TypeId), DynamicTableId> {
        let mut ids = HashMap::new();

        // assign one row to each canonical implementation pair
        for (module, object) in objects {
            for table in object.dispatch().iter_dynamic_tables() {
                let key = (
                    type_ids[&(*module, table.concrete)],
                    type_ids[&(*module, table.constraint)],
                );
                let next = DynamicTableId(ids.len() as u32);
                ids.entry(key).or_insert(next);
            }
        }

        ids
    }

    /// Build dense global ids from definitions and imported symbols.
    fn build_global_ids(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> LinkResult<(
        HashMap<(ModuleId, mir::GlobalId), GlobalId>,
        Vec<(ModuleId, mir::GlobalId)>,
    )> {
        let mut ids = HashMap::new();
        let mut globals = Vec::new();
        let mut symbols = HashMap::new();

        // assign local and exported definitions
        for (module, object) in objects {
            for global in object.globals() {
                let global_id = global.id;
                if global.linkage.is_import() {
                    continue;
                }

                let id = GlobalId::from(globals.len() as u32);
                if global.linkage.is_exported() {
                    let definition = (id, *module, global);
                    if symbols.insert(global.symbol, definition).is_some() {
                        return Err(Self::invalid_input_for(
                            package,
                            format!("global symbol {:?} has multiple definitions", global.symbol),
                        ));
                    }
                }

                ids.insert((*module, global_id), id);
                globals.push((*module, global_id));
            }
        }

        // resolve imports against exported definitions
        for (module, object) in objects {
            for global in object.globals() {
                let global_id = global.id;
                if !global.linkage.is_import() {
                    continue;
                }
                let Some((id, definition_module, definition)) =
                    symbols.get(&global.symbol).copied()
                else {
                    return Err(Self::invalid_input_for(
                        package,
                        format!("global symbol {:?} is undefined", global.symbol),
                    ));
                };
                if !Self::types_match(
                    *module,
                    global.ty,
                    definition_module,
                    definition.ty,
                    type_ids,
                ) || global.mutability != definition.mutability
                    || global.storage != definition.storage
                {
                    return Err(Self::invalid_input_for(
                        package,
                        format!(
                            "global symbol {:?} has conflicting declarations",
                            global.symbol
                        ),
                    ));
                }

                ids.insert((*module, global_id), id);
            }
        }

        Ok((ids, globals))
    }

    /// Assign contiguous Program counter and sampler ranges in function order.
    fn build_profile_starts(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
        functions: &[(ModuleId, mir::FunctionId)],
    ) -> LinkResult<(
        HashMap<(ModuleId, mir::FunctionId), CounterId>,
        HashMap<(ModuleId, mir::FunctionId), SamplerId>,
    )> {
        let mut counter_starts = HashMap::new();
        let mut sampler_starts = HashMap::new();
        let mut counts = HashMap::new();
        let mut counter_start = 0u32;
        let mut sampler_start = 0u32;

        // initialize every object-local function profile range
        for (module, object) in objects {
            for function in object.functions() {
                counts.insert((*module, function.id), (0, 0));
            }

            // derive local counter widths from their semantic sites
            for site in object.counters() {
                let count = counts
                    .get_mut(&(*module, site.point.function))
                    .ok_or_else(|| {
                        Self::invalid_input_for(package, "counter function is absent")
                    })?;
                let end = site
                    .counter
                    .0
                    .checked_add(1)
                    .ok_or_else(|| Self::invalid_input_for(package, "counter id overflow"))?;
                count.0 = count.0.max(end);
            }

            // derive local sampler widths from their semantic sites
            for site in object.samples() {
                let count = counts
                    .get_mut(&(*module, site.point.function))
                    .ok_or_else(|| {
                        Self::invalid_input_for(package, "sampler function is absent")
                    })?;
                let end = site
                    .sampler
                    .0
                    .checked_add(1)
                    .ok_or_else(|| Self::invalid_input_for(package, "sampler id overflow"))?;
                count.1 = count.1.max(end);
            }
        }

        // assign one contiguous profile range to each canonical function
        for &(module, function) in functions {
            let (counter_count, sampler_count) = counts
                .get(&(module, function))
                .copied()
                .ok_or_else(|| Self::invalid_input_for(package, "missing bytecode function"))?;

            counter_starts.insert((module, function), CounterId(counter_start));
            sampler_starts.insert((module, function), SamplerId(sampler_start));
            counter_start = counter_start
                .checked_add(counter_count)
                .ok_or_else(|| Self::invalid_input_for(package, "counter id overflow"))?;
            sampler_start = sampler_start
                .checked_add(sampler_count)
                .ok_or_else(|| Self::invalid_input_for(package, "sampler id overflow"))?;
        }

        Ok((counter_starts, sampler_starts))
    }

    /// Build one invalid input diagnostic before the linker exists.
    fn invalid_input_for(package: PackageId, context: impl Into<String>) -> LinkError {
        LinkError::InvalidInput {
            anchor: package.into(),
            package,
            context: context.into(),
        }
    }
}
