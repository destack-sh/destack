use std::sync::Arc;

use destack_artifact::{MirOptimized, Object};
use destack_core::StringPool;
use destack_mir as mir;
use destack_source::{File, FileId, FileType, ModuleId, Uri};

use crate::lower::LayoutBuilder;
use crate::{BytecodeEmitter, ObjectEmitter};

use super::super::ProgramLinker;

/// One independently emitted test module.
pub(super) struct TestModule {
    /// The persistent module identity.
    pub module: ModuleId,
    /// The emitted object.
    pub object: Arc<Object>,
    /// The strings referenced by the object.
    pub strings: StringPool,
}

impl TestModule {
    /// Emit one optimized MIR object under its module identity.
    pub(super) fn emit(
        module: ModuleId,
        source: &str,
        dependencies: impl IntoIterator<Item = ModuleId>,
    ) -> Self {
        let file = File::from_text(
            FileId::new(0),
            "test.mir".to_string(),
            Uri::from_string("test.mir"),
            None,
            FileType::Text,
            source.to_string(),
        );
        let parsed = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("test MIR should be text");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let (
            mut tree,
            target,
            types,
            mut layouts,
            dispatch,
            drops,
            memory,
            effects,
            profile,
            strings,
            _,
        ) = parsed.into_parts();

        // compute physical layouts required by object emission
        let mut layout_builder = LayoutBuilder::new(module, &mut tree, &mut layouts, target);
        layout_builder
            .layout_reachable_types()
            .expect("test MIR layouts should lower");
        let optimized = MirOptimized {
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

        // emit common metadata and relocatable bytecode
        let object =
            ObjectEmitter::new(module, &optimized, dependencies).expect("object emission failed");
        let bytecode = BytecodeEmitter::new(module, &optimized, &object)
            .emit()
            .expect("MIR should emit bytecode");

        Self {
            module,
            object: Arc::new(object.build(bytecode)),
            strings,
        }
    }

    /// Merge the strings referenced by a set of test modules.
    pub(super) fn merge_strings<'a>(modules: impl IntoIterator<Item = &'a Self>) -> StringPool {
        let strings = StringPool::new();
        for module in modules {
            strings.ensure_all_from(&module.strings);
        }

        strings
    }
}

impl ProgramLinker<'_> {
    /// Return one function by display name.
    pub(super) fn function(&self, module: ModuleId, name: &str) -> mir::FunctionId {
        let object = self.object(module);

        object
            .functions()
            .iter()
            .find_map(|function| (self.string(function.name) == name).then_some(function.id))
            .unwrap_or_else(|| panic!("missing function {name}"))
    }

    /// Return one global by display name.
    pub(super) fn global(&self, module: ModuleId, name: &str) -> mir::GlobalId {
        let object = self.object(module);

        object
            .globals()
            .iter()
            .find_map(|global| (self.string(global.name) == name).then_some(global.id))
            .unwrap_or_else(|| panic!("missing global {name}"))
    }

    /// Return the signed 32-bit integer type.
    pub(super) fn int32_type(&self, module: ModuleId) -> mir::TypeId {
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
