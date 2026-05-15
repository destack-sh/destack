use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{DirBound, DirParsed, MemoryCacheStore};
use destack_core::StringPool;
use destack_dir as dir;
use destack_parser::{Parser, ParserOptions};
use destack_source::{
    File, FileId, FileType, LanguageType, Loader, MemoryFileSystem, ModuleId, PackageId, Span, Uri,
};
use destack_workspace::{HostEnvironment, Module, Repository};

use crate::Compiler;
use crate::tests::snapshot::{DirSnapshotBuilder, DirSnapshotSet, assert_snapshot};

/// One in-memory compiler test module.
#[derive(Debug)]
pub(crate) struct TestModule {
    /// The shared string pool used by the module.
    pub(crate) strings: Arc<StringPool>,
    /// The original module source.
    pub(crate) source: String,
    /// The workspace module.
    pub(crate) module: Module,
    /// The parsed DIR artifact.
    pub(crate) dir_parsed: DirParsed,
    /// The compiler under test.
    pub(crate) compiler: Compiler,
}

impl TestModule {
    /// Parse one in-memory Destack module.
    pub(crate) fn parse(source: &str) -> Self {
        let strings = Arc::new(StringPool::new());
        let package_id = PackageId::from_path(Path::new("compiler-test"));
        let module_id =
            ModuleId::from_relative_path(package_id, PathBuf::from("main.ds").as_path());
        let file_id = FileId::from_logical_str("main.ds");
        let uri = Uri::from_string("memory:///main.ds");

        // parse source
        let file = Arc::new(File::from_text(
            file_id,
            "main.ds".to_string(),
            uri.clone(),
            None,
            FileType::Destack,
            source.to_string(),
        ));
        let mut parser = Parser::lex_file_with_options(
            file,
            LanguageType::Destack,
            ParserOptions::default(),
            strings.clone(),
        );
        let roots = parser.parse();
        let diagnostics = parser.diagnostics.collect();
        assert!(
            diagnostics.is_empty(),
            "compiler test source should parse cleanly: {diagnostics:?}"
        );

        // build parsed DIR artifact
        let (tokens, side_tokens) = parser.take_tokens();
        let anchor_expression = parser.tree.insert(
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(false)),
            Span::empty(file_id),
        );
        let dir_parsed =
            DirParsed::from_tree(parser.tree, roots, tokens, side_tokens, anchor_expression);

        // build compiler context
        let module = Module::blank(
            module_id,
            file_id,
            uri,
            None,
            package_id,
            Some(LanguageType::Destack),
            Loader::Destack,
        );
        let repository = Arc::new(Repository::new(
            PathBuf::new(),
            Arc::new(MemoryCacheStore::new()),
            Arc::new(MemoryFileSystem::new()),
            HostEnvironment::default(),
        ));
        let compiler = Compiler::new(repository);

        Self {
            strings,
            source: source.to_string(),
            module,
            dir_parsed,
            compiler,
        }
    }

    /// Bind this module.
    pub(crate) fn bind(&self) -> DirBound {
        self.compiler
            .bind_dir_parsed(&self.module, &self.dir_parsed)
    }

    /// Assert a bound DIR snapshot.
    pub(crate) fn assert_dir_bound_snapshot(
        &self,
        dir_bound: &DirBound,
        selection: DirSnapshotSet,
        expected: &str,
    ) {
        let mut builder =
            DirSnapshotBuilder::new(&self.source, &self.dir_parsed.tree, &self.strings)
                .with_bindings(&dir_bound.bindings);
        builder.add_bound(selection, dir_bound);
        let snapshot = builder.render();

        assert_snapshot(snapshot, expected);
    }
}
