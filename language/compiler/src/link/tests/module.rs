use std::sync::Arc;

use tspp_artifact::MirOptimized;
use tspp_core::StringPool;
use tspp_mir as mir;
use tspp_program::{Object, Program};
use tspp_source::{File, FileId, FileType, ModuleId, PackageId, Uri};

#[cfg(feature = "native")]
use crate::NativeEmitter;
use crate::{BytecodeEmitter, ObjectEmitter};

use super::super::ProgramLinker;

/// One independently emitted test module.
pub(in crate::link) struct TestModule {
    /// The persistent module identity.
    pub module: ModuleId,
    /// The emitted object.
    pub object: Arc<Object>,
    /// The strings referenced by the object.
    pub strings: StringPool,
}

impl TestModule {
    /// Emit one optimized MIR object under its module identity.
    pub(in crate::link) fn emit(
        module: ModuleId,
        source: &str,
        dependencies: impl IntoIterator<Item = ModuleId>,
    ) -> Self {
        let (optimized, strings) = Self::parse(source);
        let mut analyses = mir::ModuleCache::with_target_layout(optimized.target);

        // emit common metadata and relocatable bytecode
        let object = ObjectEmitter::new(module, &optimized, dependencies, &mut analyses)
            .expect("object emission failed");
        let bytecode = BytecodeEmitter::new(module, &optimized, &object)
            .emit(&mut analyses)
            .expect("MIR should emit bytecode");

        Self {
            module,
            object: Arc::new(object.bytecode(bytecode).build()),
            strings,
        }
    }

    /// Emit one optimized MIR object with native code for the host.
    #[cfg(feature = "native")]
    pub(in crate::link) fn emit_native(
        module: ModuleId,
        source: &str,
        dependencies: impl IntoIterator<Item = ModuleId>,
    ) -> Self {
        let (optimized, strings) = Self::parse(source);
        let mut analyses = mir::ModuleCache::with_target_layout(optimized.target);

        // emit common metadata with canonical bytecode beside native code
        let object = ObjectEmitter::new(module, &optimized, dependencies, &mut analyses)
            .expect("object emission failed");
        let bytecode = BytecodeEmitter::new(module, &optimized, &object)
            .emit(&mut analyses)
            .expect("MIR should emit bytecode");
        let native = NativeEmitter::new(
            module,
            &optimized,
            &object,
            &tspp_repository::Target::native(),
        )
        .expect("native emitter should initialize")
        .emit()
        .expect("MIR should emit native code");

        Self {
            module,
            object: Arc::new(object.bytecode(bytecode).native(native).build()),
            strings,
        }
    }

    /// Parse one MIR source into an optimized module.
    fn parse(source: &str) -> (MirOptimized, StringPool) {
        let file = File::from_text(
            FileId::new(0),
            "test.mir".to_string(),
            Uri::from_string("test.mir"),
            None,
            FileType::Text,
            source.to_string(),
        )
        .expect("test MIR source should load");
        let parsed = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("test MIR should be text");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let (tree, target, mut layouts, dispatch, drops, effects, profile, strings, _) =
            parsed.into_parts();

        // compute target layouts required by object emission
        let mut layout_builder = mir::LayoutBuilder::new(&tree, &mut layouts, target);
        layout_builder
            .layout_reachable_types()
            .expect("test MIR layouts should lower");
        let optimized = MirOptimized {
            target,
            initializer: None,
            tree,
            layouts,
            dispatch,
            drops,
            effects,
            profile,
        };

        (optimized, strings)
    }

    /// Merge the strings referenced by a set of test modules.
    pub(in crate::link) fn merge_strings<'a>(
        modules: impl IntoIterator<Item = &'a Self>,
    ) -> StringPool {
        let strings = StringPool::new();
        for module in modules {
            strings.ensure_all_from(&module.strings);
        }

        strings
    }

    /// Link emitted modules into one test program.
    pub(in crate::link) fn link(package: PackageId, modules: &[&Self]) -> Program {
        let strings = Self::merge_strings(modules.iter().copied());
        let objects = modules
            .iter()
            .map(|module| (module.module, module.object.clone()))
            .collect();
        let linker = ProgramLinker::new(package, objects, &strings)
            .expect("test modules should initialize the Program linker");

        linker.link().expect("test modules should link")
    }
}

impl ProgramLinker<'_> {
    /// Return one function by display name.
    pub(in crate::link) fn function(&self, module: ModuleId, name: &str) -> mir::FunctionId {
        let object = self.object(module);

        object
            .functions()
            .iter()
            .find_map(|function| (self.string(function.name) == name).then_some(function.id))
            .unwrap_or_else(|| panic!("missing function {name}"))
    }

    /// Return one global by display name.
    pub(in crate::link) fn global(&self, module: ModuleId, name: &str) -> mir::GlobalId {
        let object = self.object(module);

        object
            .globals()
            .iter()
            .find_map(|global| (self.string(global.name) == name).then_some(global.id))
            .unwrap_or_else(|| panic!("missing global {name}"))
    }

    /// Return the signed 32-bit integer type.
    pub(in crate::link) fn int32_type(&self, module: ModuleId) -> mir::TypeId {
        let object = self.object(module);

        object
            .types()
            .iter()
            .find_map(|ty| {
                matches!(
                    ty.definition,
                    mir::Type::Int {
                        width: 32,
                        is_signed: true
                    }
                )
                .then_some(ty.id)
            })
            .expect("missing int32 type")
    }
}
