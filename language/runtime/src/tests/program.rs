use std::sync::Arc;

use tspp_artifact::{MirLowered, MirOptimized};
use tspp_compiler::{BytecodeEmitter, NativeEmitter, ObjectEmitter, ProgramLinker};
use tspp_core::StringPool;
use tspp_mir as mir;
use tspp_program as program;
use tspp_source::{DiagnosticSeverity, File, FileId, FileType, ModuleId, PackageId, Uri};

/// MIR program compiled for runtime execution tests.
pub(crate) struct TestProgram {
    /// Parsed MIR before physical layout construction.
    lowered: MirLowered,
    /// Strings referenced by the MIR module.
    strings: StringPool,
    /// Whether to compile this test module to host-native code.
    is_native_compiled: bool,
}

impl TestProgram {
    /// Parse one MIR program.
    pub(crate) fn mir(source: &str) -> Self {
        let file_id = FileId::from_source_bytes(source.as_bytes());
        let file = File::from_text(
            file_id,
            "<runtime-test.tsppm>".to_string(),
            Uri::from_string("<runtime-test.tsppm>"),
            None,
            FileType::Text,
            source.to_string(),
        )
        .expect("runtime test MIR should load");
        let parsed = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("runtime test MIR should be text");
        if parsed
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            panic!("failed to parse runtime test MIR: {:?}", parsed.diagnostics);
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
            is_native_compiled: false,
        }
    }

    /// Compile this test program to host-native code.
    pub(crate) fn compile_native(mut self) -> Self {
        self.is_native_compiled = true;

        self
    }

    /// Attach one destructor for the pointee of its reference parameter in one storage.
    pub(crate) fn destructor(mut self, function_name: &str, storage: mir::Storage) -> Self {
        let function = self.function_id(function_name);
        let declaration = self.lowered.tree.get(function);
        let parameter = declaration
            .parameters
            .first()
            .expect("runtime test destructor should accept one parameter");
        let mir::Type::Reference { pointee, .. } = self.lowered.tree.get(parameter.ty) else {
            panic!("runtime test destructor should accept one reference");
        };
        let ty = *pointee;
        self.lowered.drops.set_destructor(ty, storage, function);

        self
    }

    /// Build one linked Program through the compiler emission and link path.
    pub(crate) fn build(self) -> program::Program {
        // build optimized MIR for the test module
        let package = PackageId::new(0);
        let module = ModuleId::new(package, 0);
        let optimized = self.optimized();

        // emit one relocatable object from the parsed MIR
        let mut analyses = mir::ModuleCache::with_target_layout(optimized.target);
        let emitter = ObjectEmitter::new(module, &optimized, [], &mut analyses)
            .expect("runtime test MIR should emit object metadata");
        let bytecode = BytecodeEmitter::new(module, &optimized, &emitter)
            .emit(&mut analyses)
            .expect("runtime test MIR should emit bytecode");

        // emit native code when the test requests it
        let native = if self.is_native_compiled {
            Some(
                NativeEmitter::new(
                    module,
                    &optimized,
                    &emitter,
                    &tspp_repository::Target::native(),
                )
                .expect("runtime test native emitter should initialize")
                .emit()
                .expect("runtime test MIR should emit native code"),
            )
        } else {
            None
        };

        // assemble the bytecode and native object representations
        let emitter = match native {
            Some(native) => emitter.native(native),
            None => emitter,
        };
        let object = Arc::new(emitter.bytecode(bytecode).build());

        // link the object through the production Program path
        ProgramLinker::new(package, vec![(module, object)], &self.strings)
            .expect("runtime test object should initialize its linker")
            .link()
            .expect("runtime test object should link")
    }

    /// Mark one parsed function as the module initializer.
    pub(crate) fn initializer(mut self, name: &str) -> Self {
        let function = self
            .lowered
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, function)| self.strings.get(function.name) == name)
            .map(|(id, _)| id)
            .expect("runtime test initializer should exist");
        self.lowered.initializer = Some(function);

        self
    }

    /// Complete physical layouts and build optimized MIR for emission.
    fn optimized(&self) -> MirOptimized {
        // compute the physical layouts required by emission
        let tree = self.lowered.tree.clone();
        let mut layouts = self.lowered.layouts.clone();
        let mut builder = mir::LayoutBuilder::new(&tree, &mut layouts, self.lowered.target);
        builder
            .layout_reachable_types()
            .expect("runtime test MIR layouts should build");

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

    /// Return one MIR function by name.
    fn function_id(&self, name: &str) -> mir::FunctionId {
        self.lowered
            .tree
            .iter_nodes::<mir::Function>()
            .find_map(|(id, function)| (self.strings.get(function.name) == name).then_some(id))
            .unwrap_or_else(|| panic!("missing runtime test function {name}"))
    }
}
