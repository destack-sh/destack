use std::sync::Arc;

use bytecode::{BytecodeFormatOptions, format_bytecode};
use tspp_artifact::{
    ArtifactKey, DiagnosticAnchor, DiagnosticContext, DiagnosticDisplay, DiagnosticError,
    DiagnosticLike, DiagnosticRecord, MirLowered, MirOptimized,
};
use tspp_bytecode as bytecode;
use tspp_core::StringPool;
use tspp_mir as mir;
use tspp_program::Object;
use tspp_repository::{ProviderContext, Revision};
use tspp_source::{
    DiagnosticLabel, DiagnosticSeverity, DiagnosticTarget, File, FileId, FileType, ModuleId,
    PackageId, ProfileId, TargetId, Uri,
};

#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
use crate::NativeEmitter;
use crate::tests::snapshot::assert_snapshot;
use crate::{BytecodeEmitter, ObjectEmitter};

/// MIR program under compiler tests.
pub(crate) struct TestProgram {
    /// Lowered MIR artifact.
    pub(crate) lowered: MirLowered,
    /// String pool.
    pub(crate) strings: StringPool,
    /// Test provider context.
    pub(crate) provider: TestMirProvider,
}

impl TestProgram {
    /// Parse one MIR program.
    pub(crate) fn mir(source: &str) -> Self {
        let file_id = FileId::new(0);
        let file = Arc::new(
            File::from_text(
                file_id,
                "<test.tsppm>".to_string(),
                Uri::from_string("<test.tsppm>"),
                None,
                FileType::Text,
                source.to_string(),
            )
            .expect("test MIR source should load"),
        );
        let parsed = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("test MIR should be text");
        if parsed
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            panic!("failed to parse MIR: {:?}", parsed.diagnostics);
        }
        let (tree, target, layouts, dispatch, drops, effects, profile, strings, _) =
            parsed.into_parts();

        Self {
            lowered: MirLowered {
                tree: Arc::new(tree),
                target,
                layouts,
                dispatch,
                drops,
                witnesses: mir::WitnessTable::default(),
                effects,
                profile,
                initializer: None,
            },
            strings,
            provider: TestMirProvider { file },
        }
    }

    /// Return the test module id.
    pub(crate) fn module_id(&self) -> ModuleId {
        test_module_id()
    }

    /// Emit one object and assert its exact bytecode text.
    #[track_caller]
    pub(crate) fn assert_bytecode(&self, expected: &str) -> Object {
        let optimized = self.optimized();
        let mut analyses = mir::ModuleCache::with_target_layout(optimized.target);

        // emit and format the exact relocatable bytecode object
        let object = ObjectEmitter::new(self.module_id(), &optimized, Vec::new(), &mut analyses)
            .expect("test MIR should emit object metadata");
        let mut function_names = optimized
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, function)| {
                let index = object
                    .function_index(id)
                    .expect("emitted function should have an object index");
                let name = self.strings.get(function.name).to_string();

                (index, name)
            })
            .collect::<Vec<_>>();
        function_names.sort_unstable_by_key(|(index, _)| *index);
        let function_names = function_names
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>();
        let bytecode = BytecodeEmitter::new(self.module_id(), &optimized, &object)
            .emit(&mut analyses)
            .expect("test MIR should emit bytecode");
        let formatted =
            format_bytecode(&bytecode, &function_names, BytecodeFormatOptions::default())
                .expect("test bytecode should format");

        assert_snapshot(formatted, expected);

        object.bytecode(bytecode).build()
    }

    /// Assert complete native emission and return the reloaded object.
    #[track_caller]
    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
    pub(crate) fn assert_native(&self, expected: &str) -> tspp_native::Object {
        let optimized = self.optimized();
        let mut analyses = mir::ModuleCache::with_target_layout(optimized.target);
        let object = ObjectEmitter::new(self.module_id(), &optimized, Vec::new(), &mut analyses)
            .expect("test MIR should emit object metadata");
        let emitter = NativeEmitter::new(
            self.module_id(),
            &optimized,
            &object,
            &tspp_repository::Target::native(),
        )
        .expect("host native emitter should initialize");
        let cranelift = emitter
            .format_cranelift()
            .expect("test MIR should lower to Cranelift IR");
        assert_snapshot(cranelift, expected);

        // compile and reload the complete zero-copy object
        let native = NativeEmitter::new(
            self.module_id(),
            &optimized,
            &object,
            &tspp_repository::Target::native(),
        )
        .expect("host native emitter should initialize")
        .emit()
        .expect("test MIR should emit native code");

        tspp_native::Object::from_bytes(native.bytes())
            .expect("emitted native object should reload")
    }

    /// Return the type id with one display name.
    #[track_caller]
    pub(crate) fn type_by_name(&self, name: &str) -> mir::TypeId {
        self.lowered
            .tree
            .types()
            .find_map(|(id, _)| {
                let declaration = self.lowered.tree.type_declaration(id)?;
                let declaration = self.lowered.tree.get(declaration);

                (declaration
                    .name
                    .is_some_and(|id| self.strings.get(id) == name))
                .then_some(id)
            })
            .unwrap_or_else(|| panic!("missing MIR type {name}"))
    }

    /// Return the function id with one name.
    #[track_caller]
    pub(crate) fn function_by_name(&self, name: &str) -> mir::FunctionId {
        self.lowered
            .tree
            .iter_nodes::<mir::Function>()
            .find_map(|(id, function)| (self.strings.get(function.name) == name).then_some(id))
            .unwrap_or_else(|| panic!("missing MIR function {name}"))
    }

    /// Mark one type as having a user-authored drop hook.
    #[track_caller]
    pub(crate) fn mark_drop_hook(&mut self, name: &str, function_name: &str) {
        let ty = self.type_by_name(name);
        let function = self.function_by_name(function_name);
        self.lowered.drops.set_hook(ty, function);
        *self.lowered.effects.upsert_function(function) = mir::FunctionEffect::none();
    }

    /// Register one generated frame destructor.
    #[track_caller]
    pub(crate) fn mark_destructor(&mut self, name: &str, function_name: &str) {
        let ty = self.type_by_name(name);
        let function = self.function_by_name(function_name);

        self.lowered
            .drops
            .set_destructor(ty, mir::Storage::Frame, function);
    }

    /// Register one empty dynamic implementation table.
    #[track_caller]
    pub(crate) fn mark_dynamic(&mut self, concrete: &str, constraint: &str) {
        let concrete = self.type_by_name(concrete);
        let constraint = self.type_by_name(constraint);

        self.lowered
            .dispatch
            .insert_dynamic_table(mir::DynamicTable {
                concrete,
                constraint,
                entries: Vec::new(),
                names: Vec::new(),
            });
    }

    /// Build optimized MIR with target layouts.
    fn optimized(&self) -> MirOptimized {
        let tree = self.lowered.tree.clone();
        let mut layouts = self.lowered.layouts.clone();

        // compute target layouts before exercising the emitters
        let mut builder = mir::LayoutBuilder::new(&tree, &mut layouts, self.lowered.target);
        builder
            .layout_reachable_types()
            .expect("test MIR layouts should lower");

        MirOptimized {
            target: self.lowered.target,
            initializer: self.lowered.initializer,
            tree: mir::Tree::clone(&tree),
            layouts,
            dispatch: self.lowered.dispatch.clone(),
            drops: self.lowered.drops.clone(),
            effects: self.lowered.effects.clone(),
            profile: self.lowered.profile.clone(),
        }
    }
}

/// Provider context used by raw MIR tests.
pub(crate) struct TestMirProvider {
    /// The raw MIR source file.
    pub(crate) file: Arc<File>,
}

impl DiagnosticContext for TestMirProvider {
    /// Resolve one diagnostic anchor into a source label.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        let target = match anchor {
            DiagnosticAnchor::Span(span) => DiagnosticTarget::Span(*span),
            DiagnosticAnchor::File(file) if *file == self.file.id => DiagnosticTarget::File(*file),
            DiagnosticAnchor::File(_)
            | DiagnosticAnchor::Module(_)
            | DiagnosticAnchor::Package(_) => {
                return Err(DiagnosticError::InvalidAnchor {
                    message: format!("raw MIR diagnostic cannot resolve {anchor:?}"),
                });
            }
        };

        Ok(DiagnosticLabel {
            blob: self.file.blob(),
            target,
            message,
        })
    }

    /// Display one diagnostic value.
    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        Ok(format!("{display:?}"))
    }
}

impl ProviderContext for TestMirProvider {
    /// Return the null test revision.
    fn revision(&self) -> Revision {
        Revision::NULL
    }

    /// Return the raw MIR verified artifact key.
    fn artifact_key(&self) -> ArtifactKey {
        ArtifactKey::mir_verified(test_module_id(), test_profile_id(), test_target_id())
    }

    /// Emit already-recorded diagnostics produced by this attempt.
    fn emit_diagnostics(&self, _diagnostics: Vec<DiagnosticRecord>) {}

    /// Emit one diagnostic.
    fn emit(&self, _diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError> {
        Ok(())
    }
}

/// Return the raw MIR test module id.
fn test_module_id() -> ModuleId {
    ModuleId::new(PackageId::new(0), 0)
}

/// Return the raw MIR test profile id.
fn test_profile_id() -> ProfileId {
    ProfileId::new(0)
}

/// Return the raw MIR test target id.
fn test_target_id() -> TargetId {
    TargetId::new(PackageId::new(0), "test")
}
