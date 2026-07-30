use std::sync::Arc;

use bytecode::{BytecodeFormatOptions, format_bytecode};
use destack_artifact::{
    ArtifactKey, ArtifactSidecar, DiagnosticAnchor, DiagnosticContext, DiagnosticDisplay,
    DiagnosticError, DiagnosticLike, DiagnosticRecord, MirLowered, MirOptimized, Object,
};
use destack_bytecode as bytecode;
use destack_core::StringPool;
use destack_mir as mir;
use destack_repository::{ProviderContext, Revision};
use destack_source::{
    DiagnosticLabel, DiagnosticSeverity, DiagnosticTarget, File, FileId, FileType, ModuleId,
    PackageId, ProfileId, Span, TargetId, Uri,
};

use crate::lower::LayoutBuilder;
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
        let file = Arc::new(File::from_text(
            file_id,
            "<test.dsm>".to_string(),
            Uri::from_string("<test.dsm>"),
            None,
            FileType::Text,
            source.to_string(),
        ));
        let parsed = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("test MIR should be text");
        if parsed
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            panic!("failed to parse MIR: {:?}", parsed.diagnostics);
        }
        let (tree, target, types, layouts, dispatch, drops, memory, effects, profile, strings, _) =
            parsed.into_parts();

        Self {
            lowered: MirLowered {
                tree,
                target,
                types,
                layouts,
                dispatch,
                drops,
                memory,
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

    /// Return the test profile id.
    pub(crate) fn profile_id(&self) -> ProfileId {
        test_profile_id()
    }

    /// Return the test target id.
    pub(crate) fn target_id(&self) -> TargetId {
        test_target_id()
    }

    /// Emit one object and assert its exact bytecode text.
    #[track_caller]
    pub(crate) fn assert_bytecode(&self, expected: &str) -> Object {
        let mut tree = self.lowered.tree.clone();
        let mut layouts = self.lowered.layouts.clone();

        // complete physical layouts before exercising the emission boundary
        let mut builder = LayoutBuilder::new(
            self.module_id(),
            &mut tree,
            &mut layouts,
            self.lowered.target,
        );
        builder
            .layout_reachable_types()
            .expect("test MIR layouts should lower");

        let optimized = MirOptimized {
            tree,
            target: self.lowered.target,
            types: self.lowered.types.clone(),
            layouts,
            dispatch: self.lowered.dispatch.clone(),
            drops: self.lowered.drops.clone(),
            memory: self.lowered.memory.clone(),
            effects: self.lowered.effects.clone(),
            profile: self.lowered.profile.clone(),
        };

        // emit and format the exact relocatable bytecode object
        let object = ObjectEmitter::new(self.module_id(), &optimized, Vec::new())
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
        let (bytecode, frames) = BytecodeEmitter::new(self.module_id(), &optimized, &object)
            .emit()
            .expect("test MIR should emit bytecode");
        let formatted =
            format_bytecode(&bytecode, &function_names, BytecodeFormatOptions::default())
                .expect("test bytecode should format");

        assert_snapshot(formatted, expected);

        object.build(bytecode, frames)
    }

    /// Return the type id with one display name.
    #[track_caller]
    pub(crate) fn type_by_name(&self, name: &str) -> mir::TypeId {
        self.lowered
            .tree
            .iter_nodes::<mir::Type>()
            .find_map(|(id, _)| {
                let declaration = self.lowered.tree.type_declaration(id)?;
                let declaration = self.lowered.tree.get(declaration);

                (self.strings.get(declaration.name) == name).then_some(id)
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
        let parameter = self.lowered.tree.get(function).parameters[0].ty;
        let mir::Type::Reference { storage, .. } = self.lowered.tree.get(parameter) else {
            panic!("drop hook {function_name} has no reference receiver");
        };
        self.lowered.drops.set_hook(ty, *storage, function);
        self.lowered
            .effects
            .functions
            .insert(function, mir::FunctionEffect::none());
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
        let span = match anchor {
            DiagnosticAnchor::Span(span) => *span,
            DiagnosticAnchor::Symbol(_)
            | DiagnosticAnchor::File(_)
            | DiagnosticAnchor::Module(_)
            | DiagnosticAnchor::Package(_) => Span::empty(self.file.id),
        };

        Ok(DiagnosticLabel {
            content: self.file.content_id(),
            target: DiagnosticTarget::Span(span),
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

    /// Emit one sidecar.
    fn emit_sidecar(&self, _sidecar: ArtifactSidecar) {}

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
