use destack_core::{EntryRange, LocalStringPool, Optional, SectionBuilder, StringId};

use crate::{
    CodeRange, Constant, ConstantId, ConstantRelocation, FrameSlot, Function, FunctionId,
    FunctionType, FunctionTypeId, Global, GlobalId, InstructionRelocation, Object, StringEntry,
    Type, TypeId, ValueType,
};

use super::object::Root;

/// Bytecode object under construction.
#[derive(Debug, Default)]
pub struct ObjectBuilder {
    /// Strings interned while this object is built.
    strings: LocalStringPool,

    /// Type symbols.
    types: Vec<Type>,
    /// Function types.
    function_types: Vec<FunctionType>,
    /// Flattened function value types.
    value_types: Vec<ValueType>,

    /// Global declarations and definitions.
    globals: Vec<Global>,
    /// Immutable constants.
    constants: Vec<Constant>,
    /// Concatenated constant bytes.
    constant_bytes: Vec<u8>,

    /// Fixed frame slots.
    frame_slots: Vec<FrameSlot>,
    /// Function declarations and definitions.
    functions: Vec<Function>,
    /// Encoded function bytes.
    code: Vec<u8>,
    /// Relocations inside function bytes.
    instruction_relocations: Vec<InstructionRelocation>,
    /// Relocations inside constant bytes.
    constant_relocations: Vec<ConstantRelocation>,
}

impl ObjectBuilder {
    /// Create an empty bytecode object builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set interned object strings.
    pub fn strings(mut self, strings: LocalStringPool) -> Self {
        self.strings = strings;

        self
    }

    /// Set type symbols.
    pub fn types(mut self, types: impl IntoIterator<Item = Type>) -> Self {
        self.types = types.into_iter().collect();

        self
    }

    /// Set function types.
    pub fn function_types(
        mut self,
        function_types: impl IntoIterator<Item = FunctionType>,
    ) -> Self {
        self.function_types = function_types.into_iter().collect();

        self
    }

    /// Set flattened function value types.
    pub fn value_types(mut self, value_types: impl IntoIterator<Item = ValueType>) -> Self {
        self.value_types = value_types.into_iter().collect();

        self
    }

    /// Set global declarations and definitions.
    pub fn globals(mut self, globals: impl IntoIterator<Item = Global>) -> Self {
        self.globals = globals.into_iter().collect();

        self
    }

    /// Set immutable constants.
    pub fn constants(mut self, constants: impl IntoIterator<Item = Constant>) -> Self {
        self.constants = constants.into_iter().collect();

        self
    }

    /// Set immutable constant bytes.
    pub fn constant_bytes(mut self, constant_bytes: impl Into<Vec<u8>>) -> Self {
        self.constant_bytes = constant_bytes.into();

        self
    }

    /// Set logical frame slots.
    pub fn frame_slots(mut self, frame_slots: impl IntoIterator<Item = FrameSlot>) -> Self {
        self.frame_slots = frame_slots.into_iter().collect();

        self
    }

    /// Set function declarations and definitions.
    pub fn functions(mut self, functions: impl IntoIterator<Item = Function>) -> Self {
        self.functions = functions.into_iter().collect();

        self
    }

    /// Set encoded function bytes.
    pub fn code(mut self, code: impl Into<Vec<u8>>) -> Self {
        self.code = code.into();

        self
    }

    /// Set relocations inside function bytes.
    pub fn instruction_relocations(
        mut self,
        relocations: impl IntoIterator<Item = InstructionRelocation>,
    ) -> Self {
        self.instruction_relocations = relocations.into_iter().collect();

        self
    }

    /// Set relocations inside constant bytes.
    pub fn constant_relocations(
        mut self,
        relocations: impl IntoIterator<Item = ConstantRelocation>,
    ) -> Self {
        self.constant_relocations = relocations.into_iter().collect();

        self
    }

    /// Intern one stable string and return its content id.
    pub(crate) fn intern_string(&mut self, text: &str) -> StringId {
        self.strings.intern(text)
    }

    /// Return the number of type symbols.
    pub(crate) fn type_count(&self) -> usize {
        self.types.len()
    }

    /// Append one type symbol.
    pub(crate) fn push_type(&mut self, name: StringId) -> TypeId {
        let ty = TypeId(self.types.len() as u32);
        self.types.push(Type { name });

        ty
    }

    /// Return one function type under construction.
    pub(crate) fn function_type(&self, id: FunctionTypeId) -> Option<&FunctionType> {
        self.function_types.get(id.index())
    }

    /// Return the number of globals.
    pub(crate) fn global_count(&self) -> usize {
        self.globals.len()
    }

    /// Append one global and return its object-local id.
    pub(crate) fn push_global(&mut self, global: Global) -> GlobalId {
        let id = GlobalId(self.globals.len() as u32);
        self.globals.push(global);

        id
    }

    /// Return the number of immutable constants.
    pub(crate) fn constant_count(&self) -> usize {
        self.constants.len()
    }

    /// Append one immutable constant and return its object-local id.
    pub(crate) fn push_constant(
        &mut self,
        name: Optional<StringId>,
        alignment_bytes: u32,
        bytes: impl AsRef<[u8]>,
    ) -> ConstantId {
        let bytes = bytes.as_ref();
        let start = self.constant_bytes.len() as u32;
        let range = EntryRange::new(start, bytes.len() as u32);
        let id = ConstantId(self.constants.len() as u32);
        self.constant_bytes.extend_from_slice(bytes);
        self.constants.push(Constant {
            name,
            alignment_bytes,
            bytes: range,
        });

        id
    }

    /// Append one relocation relative to an immutable constant.
    pub(crate) fn push_constant_relocation(
        &mut self,
        constant: ConstantId,
        relocation: ConstantRelocation,
    ) {
        let constant = &self.constants[constant.index()];
        self.constant_relocations
            .push(relocation.rebase(constant.bytes.start));
    }

    /// Return the number of logical frame slots.
    pub(crate) fn frame_slot_count(&self) -> usize {
        self.frame_slots.len()
    }

    /// Append one logical frame slot.
    pub(crate) fn push_frame_slot(&mut self, slot: FrameSlot) {
        self.frame_slots.push(slot);
    }

    /// Return the number of functions.
    pub(crate) fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Append one function and return its object-local id.
    pub(crate) fn push_function(&mut self, function: Function) -> FunctionId {
        let id = FunctionId(self.functions.len() as u32);
        self.functions.push(function);

        id
    }

    /// Append one encoded function body and return its code range.
    pub(crate) fn push_code(
        &mut self,
        bytes: &[u8],
        relocations: impl IntoIterator<Item = InstructionRelocation>,
    ) -> CodeRange {
        let byte_offset = self.code.len() as u32;
        let byte_len = bytes.len() as u32;
        self.code.extend_from_slice(bytes);
        self.instruction_relocations.extend(
            relocations
                .into_iter()
                .map(|relocation| relocation.rebase(byte_offset)),
        );

        CodeRange {
            byte_offset,
            byte_len,
        }
    }

    /// Append value types and return their range.
    pub(crate) fn push_value_types(
        &mut self,
        values: impl IntoIterator<Item = ValueType>,
    ) -> EntryRange<ValueType> {
        let start = self.value_types.len();
        self.value_types.extend(values);

        EntryRange::new(start as u32, (self.value_types.len() - start) as u32)
    }

    /// Append one function type and return its object-local id.
    pub(crate) fn push_function_type(
        &mut self,
        name: Optional<StringId>,
        parameters: impl IntoIterator<Item = ValueType>,
        results: impl IntoIterator<Item = ValueType>,
    ) -> FunctionTypeId {
        let parameters = self.push_value_types(parameters);
        let results = self.push_value_types(results);
        let function_type = FunctionTypeId(self.function_types.len() as u32);
        self.function_types.push(FunctionType {
            name,
            parameters,
            results,
        });

        function_type
    }

    /// Build one immutable bytecode object.
    pub fn build(self) -> Object {
        let mut string_entries = Vec::with_capacity(self.strings.len());
        let mut string_bytes = Vec::new();

        // pack strings in stable identity order for binary search
        let mut strings = self.strings.iter().collect::<Vec<_>>();
        strings.sort_unstable_by_key(|(id, _)| *id);
        for (id, text) in strings {
            let start = string_bytes.len() as u32;
            let bytes = EntryRange::new(start, text.len() as u32);
            string_bytes.extend_from_slice(text.as_bytes());
            string_entries.push(StringEntry { id, bytes });
        }

        let mut sections = SectionBuilder::new();
        let mut root = Root::new();
        let root_section = sections.insert([root]);

        // pack strings and type tables
        let strings = sections.insert(string_entries);
        let string_bytes = sections.insert(string_bytes);
        let types = sections.insert(self.types);
        let function_types = sections.insert(self.function_types);
        let value_types = sections.insert(self.value_types);

        // pack global and function tables
        let globals = sections.insert(self.globals);
        let constants = sections.insert(self.constants);
        let constant_bytes = sections.insert(self.constant_bytes);
        let frame_slots = sections.insert(self.frame_slots);
        let functions = sections.insert(self.functions);

        // pack encoded instructions and relocations
        let code = sections.insert(self.code);
        let instruction_relocations = sections.insert(self.instruction_relocations);
        let constant_relocations = sections.insert(self.constant_relocations);

        // finalize the fixed root after all section offsets are known
        root.byte_len = sections.view().byte_len() as u64;
        root.strings = strings;
        root.string_bytes = string_bytes;
        root.types = types;
        root.function_types = function_types;
        root.value_types = value_types;

        root.globals = globals;
        root.constants = constants;
        root.constant_bytes = constant_bytes;
        root.frame_slots = frame_slots;
        root.functions = functions;

        root.code = code;
        root.instruction_relocations = instruction_relocations;
        root.constant_relocations = constant_relocations;
        sections.replace(root_section, [root]);
        let storage = sections.build();

        Object::from_root(root, storage)
    }
}
