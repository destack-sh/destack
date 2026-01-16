use std::collections::HashMap;

use destack_base::StringPool;
use destack_dir::{GlobalSymbolId, LocalNodeId};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId, TargetId};
use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;

use crate::lower::item::GlobalBinding;
use crate::lower::{BuiltinTypeLayouts, TypeLowerer};

/// Context for lowering a DIR module to MIR.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ModuleLowerer<'a> {
    /// Provide access to the compiler for shared resources.
    pub(crate) compiler: &'a crate::Compiler,
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Identify the profile used for DIR access.
    pub(crate) profile: ProfileId,
    /// Provide access to the source module data.
    pub(crate) module: &'a Module,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to the root expressions for the module.
    pub(crate) dir_roots: &'a [LocalNodeId<dir::Expression>],
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Identify the target backend for lowering.
    pub(crate) target: &'a TargetId,

    /// Build MIR nodes for this module.
    pub(crate) builder: mir::ModuleBuilder,
    /// Map DIR symbols to MIR function ids.
    pub(crate) functions_by_symbol: HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Map DIR symbols to MIR global bindings.
    pub(crate) globals_by_symbol: HashMap<GlobalSymbolId, GlobalBinding>,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: TypeLowerer,
}

#[allow(clippy::too_many_arguments)]
impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(
        compiler: &'a crate::Compiler,
        module: &'a Module,
        profile: ProfileId,
        dir_tree: &'a dir::NodeTree,
        dir_roots: &'a [LocalNodeId<dir::Expression>],
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        target: &'a TargetId,
        pointer_bytes: u8,
    ) -> Self {
        // initialize the module builder
        let mut builder = mir::ModuleBuilder::new();

        // seed the mir string pool with program strings
        let strings = compiler.program.strings.as_ref().clone().into_immutable();
        builder.strings().copy_from_immutable(&strings);

        // create the type lowerer
        let type_lowerer = TypeLowerer::new(&mut builder, pointer_bytes);

        Self {
            compiler,
            module_id: module.id,
            profile,
            module,
            dir_tree,
            dir_roots,
            symbols,
            types,
            target,
            builder,
            functions_by_symbol: HashMap::new(),
            globals_by_symbol: HashMap::new(),
            type_lowerer,
        }
    }

    /// Lower this entire DIR module to MIR (in-place).
    ///
    /// Lowering proceeds in four phases:
    /// 1. Types: lower struct/class type layouts (cached on-demand)
    /// 2. Declarations: lower globals, function/method signatures and bodies
    /// 3. Tables: generate vtables, itabs, RTTI (currently stubbed)
    /// 4. Emit: (bodies are currently lowered inline with declarations)
    pub(crate) fn lower_module(&mut self) -> LowerResult<()> {
        // phase 1: types (lowered lazily as encountered)
        self.lower_types()?;

        // phase 2: declarations (globals + functions)
        self.lower_items()?;

        // phase 3: tables (vtables, itabs, RTTI)
        self.lower_tables()?;

        Ok(())
    }

    /// Phase 1: Lower type declarations.
    ///
    /// Type layouts are cached lazily when first encountered during lowering.
    /// This phase is a no-op since types are lowered on-demand.
    fn lower_types(&mut self) -> LowerResult<()> {
        // initialize builtin string layout for string literals and types
        self.initialize_string_type()?;

        // predeclare nominal layouts for struct and class instance types
        self.predeclare_nominal_layouts()?;

        // types are lowered lazily via TypeLowerer when first accessed
        Ok(())
    }

    /// Phase 2: Lower item declarations (globals, functions, methods).
    ///
    /// Processes all root expressions to lower globals and function bodies.
    fn lower_items(&mut self) -> LowerResult<()> {
        self.predeclare_external_calls()?;
        for expression_id in self.dir_roots.iter().copied() {
            self.lower_root_expression(expression_id)?;
        }
        Ok(())
    }

    /// Finish the module lowering process and return the resulting MIR tree and string pool.
    pub(crate) fn finish(self) -> (mir::NodeTree, StringPool) {
        self.builder.finish_mutable()
    }

    /// Initialize the canonical string type from builtin definitions.
    fn initialize_string_type(&mut self) -> LowerResult<()> {
        // resolve a stable anchor for type lowering
        let Some(anchor) = self
            .dir_roots
            .first()
            .copied()
            .map(|root| root.into_global_any(self.module_id))
            .map(|root| root.into_anchored(Some(self.profile)))
        else {
            return Ok(());
        };

        // ensure builtin layouts are installed
        let mut builtin_layouts = BuiltinTypeLayouts::new(
            self.compiler,
            self.profile,
            &mut self.builder,
            &mut self.type_lowerer,
        );
        builtin_layouts.ensure_string_layout(anchor)?;

        Ok(())
    }
}
