use std::collections::HashMap;

use destack_core::{SectionPacker, StringPool};
use destack_heap as heap;
use destack_mir as mir;
use destack_program::{
    FunctionId, GlobalId, Program, ProgramHeader, StringTable, TraceTable, TypeId,
};
use destack_source::PackageId;

use crate::{LinkError, LinkResult};

use super::vm::VmLinker;
use super::{DispatchLinker, FunctionLinker, LayoutLinker, StaticLinker, TypeLinker};

/// Build one program from one MIR tree and immutable string pool.
#[derive(Debug)]
pub struct ProgramLinker {
    /// Package that owns the linked program.
    package: PackageId,
    /// Local heap options baked into the program header.
    heap_options: heap::HeapOptions,
    /// Shared heap options baked into the program header.
    shared_heap_options: heap::SharedHeapOptions,
    /// MIR tree being linked.
    tree: mir::Tree,
    /// Target ABI layout for this program.
    target_layout: mir::TargetLayout,
    /// MIR type table for names, lineage, and type attachments.
    type_table: mir::TypeTable,
    /// MIR layout table produced by lower and optimization.
    layout_table: mir::LayoutTable,
    /// MIR dispatch table produced by lower and optimization.
    dispatch_table: mir::DispatchTable,
    /// Program string pool.
    strings: StringPool,
    /// Dense program function ids keyed by MIR function id.
    function_ids: HashMap<mir::FunctionId, FunctionId>,
    /// Dense program type ids keyed by MIR type id.
    type_ids: HashMap<mir::TypeId, TypeId>,
    /// MIR type ids keyed by program type id.
    types_by_id: Vec<mir::TypeId>,
    /// Dense program global ids keyed by MIR global id.
    global_ids: HashMap<mir::GlobalId, GlobalId>,
}

impl ProgramLinker {
    /// Create one program linker.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        package: PackageId,
        tree: mir::Tree,
        target_layout: mir::TargetLayout,
        types: mir::TypeTable,
        layouts: mir::LayoutTable,
        dispatch: mir::DispatchTable,
        strings: StringPool,
        heap_options: heap::HeapOptions,
        shared_heap_options: heap::SharedHeapOptions,
    ) -> Self {
        let function_ids = Self::build_function_ids(&tree);
        let (type_ids, types_by_id) = Self::build_type_ids(&tree);
        let global_ids = Self::build_global_ids(&tree);

        Self {
            package,
            heap_options,
            shared_heap_options,
            tree,
            target_layout,
            type_table: types,
            layout_table: layouts,
            dispatch_table: dispatch,
            strings,
            function_ids,
            type_ids,
            types_by_id,
            global_ids,
        }
    }

    /// Build the program.
    pub fn build(self) -> LinkResult<Program> {
        let mut sections = SectionPacker::new();

        // project program tables before lowering VM code
        let functions = FunctionLinker::new(&self.tree, &self).link(&mut sections);
        let layouts = LayoutLinker::new(
            &self.tree,
            &self.target_layout,
            &self.type_table,
            &self.layout_table,
            &self,
        )
        .link(&mut sections)?;
        let types = TypeLinker::new(&self.tree, &self.target_layout, &self.type_table, &self)
            .link(&mut sections, &layouts.ids)?;
        let statics = StaticLinker::new(&self.tree, &self.target_layout, &self, &layouts.storage)
            .link(&mut sections)?;
        let dispatch = DispatchLinker::new(&self.dispatch_table, &self).link(&mut sections);

        // lower VM code and frame metadata
        let vm = VmLinker::new(
            &self.tree,
            &self.target_layout,
            &self.type_table,
            &self.layout_table,
            &self.heap_options,
            &self.shared_heap_options,
            &self,
            &layouts.storage,
            &layouts.trace_maps,
        )
        .link(&mut sections)?;

        // project trace rows after VM lowering consumes compiler trace maps
        let traces = TraceTable::pack(&mut sections, &layouts.trace_maps);
        let strings = StringTable::from_pool(&mut sections, &self.strings);
        let info = None;

        // assemble the durable program
        let package = self.package;
        let (directory, storage) = sections.finish();
        let header = ProgramHeader::new(
            directory,
            self.target_layout,
            self.heap_options,
            self.shared_heap_options,
            strings,
            types,
            layouts.layouts,
            vm.frames,
            functions,
            dispatch,
            traces,
            statics.globals,
            info,
            statics.constants,
            statics.shared,
            statics.local,
            vm.code,
            None,
        );
        let program = Program::new(header, storage).map_err(|error| LinkError::InvalidInput {
            anchor: package.into(),
            package,
            context: error.to_string(),
        })?;

        Ok(program)
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

    /// Return one invalid instruction diagnostic.
    pub(crate) fn invalid_instruction(&self, context: impl Into<String>) -> LinkError {
        LinkError::InvalidInstruction {
            anchor: self.package.into(),
            package: self.package,
            context: context.into(),
        }
    }

    /// Return one invalid cast diagnostic.
    pub(crate) fn invalid_cast(&self, context: impl Into<String>) -> LinkError {
        LinkError::InvalidCast {
            anchor: self.package.into(),
            package: self.package,
            context: context.into(),
        }
    }

    /// Return one invalid field access diagnostic.
    pub(crate) fn invalid_field_access(&self, index: u32, field_count: usize) -> LinkError {
        LinkError::InvalidFieldAccess {
            anchor: self.package.into(),
            package: self.package,
            index,
            field_count,
        }
    }

    /// Return one invalid pointer type diagnostic.
    pub(crate) fn invalid_pointer_type(&self, actual: impl Into<String>) -> LinkError {
        LinkError::InvalidPointerType {
            anchor: self.package.into(),
            package: self.package,
            actual: actual.into(),
        }
    }

    /// Return one unsupported instruction diagnostic.
    pub(crate) fn unsupported_instruction(&self, name: impl Into<String>) -> LinkError {
        LinkError::UnsupportedInstruction {
            anchor: self.package.into(),
            package: self.package,
            name: name.into(),
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

    /// Return one undefined function diagnostic.
    pub(crate) fn undefined_function(&self, function: impl Into<String>) -> LinkError {
        LinkError::UndefinedFunction {
            anchor: self.package.into(),
            package: self.package,
            function: function.into(),
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

    /// Return one internal link diagnostic.
    pub(crate) fn internal(&self, message: impl Into<String>) -> LinkError {
        LinkError::Internal {
            anchor: self.package.into(),
            package: self.package,
            message: message.into(),
        }
    }

    /// Return the program function id for one MIR function.
    pub(crate) fn function_id(&self, function: mir::FunctionId) -> FunctionId {
        self.function_ids[&function]
    }

    /// Return the number of program function ids.
    pub(crate) fn function_count(&self) -> usize {
        self.function_ids.len()
    }

    /// Return the program type id for one MIR type.
    pub(crate) fn type_id(&self, ty: mir::TypeId) -> TypeId {
        self.type_ids[&ty]
    }

    /// Return the input type id for one program type id.
    pub(crate) fn type_by_id(&self, ty: TypeId) -> Option<mir::TypeId> {
        self.types_by_id.get(ty.index()).copied()
    }

    /// Return the program global id for one MIR global.
    pub(crate) fn global_id(&self, global: mir::GlobalId) -> GlobalId {
        self.global_ids[&global]
    }

    /// Build dense function ids from MIR storage order.
    fn build_function_ids(tree: &mir::Tree) -> HashMap<mir::FunctionId, FunctionId> {
        tree.iter_nodes::<mir::Function>()
            .enumerate()
            .map(|(index, (function, _))| (function, FunctionId::from(index as u32)))
            .collect()
    }

    /// Build dense type ids and their inverse MIR map.
    fn build_type_ids(tree: &mir::Tree) -> (HashMap<mir::TypeId, TypeId>, Vec<mir::TypeId>) {
        let types_by_id = tree
            .iter_nodes::<mir::Type>()
            .map(|(ty, _)| ty)
            .collect::<Vec<_>>();
        let type_ids = types_by_id
            .iter()
            .copied()
            .enumerate()
            .map(|(index, ty)| (ty, TypeId::from(index as u32)))
            .collect();

        (type_ids, types_by_id)
    }

    /// Build dense global ids from MIR storage order.
    fn build_global_ids(tree: &mir::Tree) -> HashMap<mir::GlobalId, GlobalId> {
        tree.iter_nodes::<mir::Global>()
            .enumerate()
            .map(|(index, (global, _))| (global, GlobalId::from(index as u32)))
            .collect()
    }
}
