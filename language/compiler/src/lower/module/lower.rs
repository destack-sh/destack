use destack_artifact::{DiagnosticLike, MirLowered};
use destack_core::{FxIndexMap, FxIndexSet, StringPool};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{FunctionLowerer, LowerModuleState, Nominal};
use crate::{CompilerError, CompilerResult, LowerError};

/// Lowering state for one module: the checked DIR read side.
pub(crate) struct ModuleLowerer<'a> {
    /// The module being lowered.
    pub(in crate::lower) module: ModuleId,
    /// The module whose DIR the current body reads: foreign for instance copies.
    pub(in crate::lower) source: ModuleId,
    /// The source string pool.
    pub(in crate::lower) strings: &'a StringPool,
    /// The sealed check output of every reachable module.
    pub(in crate::lower) modules: FxIndexMap<ModuleId, LowerModuleState>,
    /// The MIR function declared for each callable symbol and instance carrier key.
    pub(in crate::lower) functions:
        FxIndexMap<(dir::GlobalSymbolId, Vec<mir::Type>), mir::FunctionId>,
    /// The MIR nominal lowered for each declaration symbol.
    pub(in crate::lower) nominals: FxIndexMap<dir::GlobalSymbolId, Nominal>,
    /// The reference reserved for each class nominal still lowering.
    pub(in crate::lower) reserved: FxIndexMap<dir::GlobalSymbolId, mir::LocalNodeId<mir::Type>>,
    /// The lifetime slot declared for each parameter of the current function.
    pub(in crate::lower) lifetime_slots: FxIndexMap<dir::LocalGenericParameterId, u16>,
    /// The argument substituted for each generic parameter of the current instance.
    pub(in crate::lower) substitution: FxIndexMap<dir::GlobalGenericParameterId, dir::GlobalTypeId>,
}

impl<'a> ModuleLowerer<'a> {
    /// Create the lowering state over one materialized module.
    pub(crate) fn new(
        module: ModuleId,
        strings: &'a StringPool,
        modules: FxIndexMap<ModuleId, LowerModuleState>,
    ) -> Self {
        Self {
            module,
            source: module,
            strings,
            modules,
            functions: FxIndexMap::default(),
            nominals: FxIndexMap::default(),
            reserved: FxIndexMap::default(),
            lifetime_slots: FxIndexMap::default(),
            substitution: FxIndexMap::default(),
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

        // lower every declared nominal type
        self.lower_nominals(&mut builder)?;

        // declare every callable header so bodies can call in any order
        let mut bodies = Vec::new();
        for index in 0..self.local().roots.len() {
            let root = self.local().roots[index];

            // classify the root form
            let declaration = match *self.local().tree().get(root) {
                dir::Expression::Declaration(declaration) => declaration,
                // imports and exports carry no runtime code
                dir::Expression::Import { .. } | dir::Expression::Export { .. } => continue,
                ref other => {
                    let error = LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: format!("a module-level '{}' statement", other.variant_name()),
                    };
                    match CompilerError::from(error) {
                        CompilerError::Diagnostic(diagnostic) => errors.push(diagnostic),
                        error => return Err(error),
                    }

                    continue;
                }
            };
            self.lower_declaration(&mut builder, declaration, &mut bodies, &mut errors)?;
        }

        // materialize every generic instantiation the scanned bodies demand
        let mut imports = FxIndexSet::default();
        match self.materialize_instances(&mut builder, &bodies) {
            Ok((instances, demanded)) => {
                bodies.extend(instances);
                imports = demanded;
            }
            Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
            Err(error) => return Err(error),
        }

        // declare an import for every foreign callable the bodies call
        match self.declare_imported_functions(&mut builder, imports) {
            Ok(()) => {}
            Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
            Err(error) => return Err(error),
        }

        // lower every declared body, keeping failures isolated per function
        for body in bodies {
            self.lifetime_slots = body.lifetimes.clone();
            self.substitution = body.substitution.clone();
            self.source = body.source;
            match FunctionLowerer::run(self, &mut builder, body) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }
        self.source = self.module;

        // compute layouts for every aggregate type in the module
        self.lower_layouts(&mut builder)?;

        // publish the MIR names into the shared pool, string ids are content hashed
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
