use destack_artifact::{
    DiagnosticLike, DirBound, DirCheckedModule, DirExpanded, DirMaterialized, DirParsed, MirLowered,
};
use destack_core::{FxIndexMap, StringPool};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult};

/// Lowering state for one module: the checked DIR read side.
pub(crate) struct ModuleLowerer<'a> {
    /// The module being lowered.
    pub(in crate::lower) module: ModuleId,
    /// The source string pool.
    pub(in crate::lower) strings: &'a StringPool,
    /// The DIR tree.
    pub(in crate::lower) tree: &'a dir::Tree,
    /// The DIR roots.
    pub(in crate::lower) roots: &'a [dir::LocalNodeId<dir::Expression>],
    /// The checked type table.
    pub(in crate::lower) types: dir::TypeTable<'static>,
    /// The checked resolution table.
    pub(in crate::lower) resolutions: dir::ResolutionTable<'static>,
    /// The checked binding table.
    pub(in crate::lower) bindings: dir::BindingTable<'static>,
    /// The checked coercion table.
    pub(in crate::lower) coercions: dir::CoercionTable<'static>,
    /// The MIR function declared for each function symbol.
    pub(in crate::lower) functions: FxIndexMap<dir::LocalSymbolId, mir::FunctionId>,
}

impl<'a> ModuleLowerer<'a> {
    /// Create the lowering state over one materialized module.
    pub(crate) fn new(
        module: ModuleId,
        parsed: &'a DirParsed,
        bound: &DirBound,
        expanded: &DirExpanded,
        checked: &DirCheckedModule,
        materialized: &'a DirMaterialized,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            module,
            strings,
            tree: &parsed.tree,
            roots: materialized.roots.as_ref(),
            types: materialized.type_table(bound, expanded, checked),
            resolutions: materialized.resolution_table(checked),
            bindings: materialized.binding_table(bound, expanded),
            coercions: materialized.coercion_table(checked),
            functions: FxIndexMap::default(),
        }
    }

    /// Lower the module to MIR, returning the artifact and its diagnostics.
    pub(crate) fn lower(
        &mut self,
        pointer_bytes: u8,
    ) -> CompilerResult<(MirLowered, Vec<Box<dyn DiagnosticLike>>)> {
        let mut builder = mir::ModuleBuilder::new();
        builder.set_pointer_bytes(pointer_bytes);
        let mut errors = Vec::new();

        // declare every function header so bodies can call in any order
        let (tree, roots) = (self.tree, self.roots);
        let mut bodies = Vec::new();
        for root in roots {
            let dir::Expression::Declaration(declaration) = tree.get(*root) else {
                continue;
            };
            let dir::Declaration::Function(function) = tree.get(*declaration).clone() else {
                continue;
            };
            let Some(body) = function.body else {
                continue;
            };

            // accumulate unsupported diagnostics; abort on internal failures
            match self.declare_function(&mut builder, *declaration, &function, body) {
                Ok(body) => bodies.push(body),
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // lower every declared body, keeping failures isolated per function
        for body in bodies {
            match FunctionLowerer::run(self, &mut builder, body) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // publish the MIR names into the shared pool: string ids are content hashed
        let (tree, target, types, layouts, dispatch, drops, memory, effects, profile, strings) =
            builder.finish();
        self.strings.ensure_all_from(&strings);

        let lowered = MirLowered {
            tree,
            target,
            types,
            layouts,
            dispatch,
            drops,
            memory,
            effects,
            profile,
        };

        Ok((lowered, errors))
    }
}
