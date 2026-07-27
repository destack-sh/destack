use std::sync::{Arc, Mutex};

use destack_core::StringPool;
use destack_dir as dir;
use destack_parser::{CommentRetention, Parser};
use destack_source::{
    DiagnosticCollection, File, FileId, FileType, LanguageType, ModuleId, PrintOptions, Span, Uri,
    print_diagnostics,
};

use super::TestProgram;
use crate::ProgramContext;

/// Candidate source used by pattern tests.
pub(crate) enum TestSource {
    /// Authoritative parsed DIR for structural tests.
    Parsed {
        /// The authored candidate file.
        file: Arc<File>,
        /// The authoritative candidate DIR.
        tree: Box<dir::Tree>,
    },
    /// Checked DIR for semantic predicate tests.
    Checked {
        /// The authored candidate file.
        file: Arc<File>,
        /// The candidate module.
        module: ModuleId,
        /// The checked test program.
        program: ProgramContext,
    },
}

impl TestSource {
    /// Parse one authoritative candidate source.
    pub(crate) fn parse(text: &str, strings: Arc<StringPool>) -> Self {
        let source = fixture_text(text).to_string();
        let file = test_file("<pattern-test>", source);
        let mut parser = Parser::lex_file_with_comment_retention(
            file.clone(),
            LanguageType::Destack,
            CommentRetention::Ignore,
            strings,
        );
        let roots = parser.parse();
        assert!(parser.errors.is_empty(), "{:?}", parser.errors);
        parser.tree.index_parents(&roots);
        parser.publish_strings();

        Self::Parsed {
            file,
            tree: Box::new(parser.tree),
        }
    }

    /// Compile one candidate source through checked DIR.
    pub(crate) fn checked(
        text: &str,
        dependencies: &[(String, String)],
    ) -> (Self, Arc<StringPool>) {
        let source = fixture_text(text);
        let (file, module, program, strings) = TestProgram::compile(source, dependencies);

        (
            Self::Checked {
                file,
                module,
                program,
            },
            strings,
        )
    }

    /// Return the candidate source file.
    pub(crate) fn file(&self) -> &File {
        match self {
            Self::Parsed { file, .. } | Self::Checked { file, .. } => file,
        }
    }

    /// Return the candidate DIR.
    pub(crate) fn tree(&self) -> &dir::Tree {
        match self {
            Self::Parsed { tree, .. } => tree,
            Self::Checked {
                module, program, ..
            } => program.module(*module).expect("checked test module").tree(),
        }
    }

    /// Return a view over the candidate DIR.
    pub(crate) fn view(&self) -> dir::View<'_> {
        match self {
            Self::Parsed { tree, .. } => dir::View::new(tree),
            Self::Checked {
                module, program, ..
            } => program.module(*module).expect("checked test module").view(),
        }
    }

    /// Return the checked program and candidate module when available.
    pub(crate) fn program(&self) -> Option<(&ProgramContext, ModuleId)> {
        match self {
            Self::Parsed { .. } => None,
            Self::Checked {
                module, program, ..
            } => Some((program, *module)),
        }
    }

    /// Return the authored candidate source.
    pub(crate) fn text(&self) -> &str {
        self.file().text()
    }

    /// Return the exact authored text of one node.
    pub(crate) fn node_text(&self, node: dir::LocalNodeIdAny) -> &str {
        let span = self.tree().get_span_by_id(node.id).expect("node span");

        self.file().get_span_str(span).expect("node source")
    }

    /// Return the source span of one node.
    pub(crate) fn node_span(&self, node: dir::LocalNodeIdAny) -> Span {
        self.tree().get_span_by_id(node.id).expect("node span")
    }

    /// Return the byte offset starting the line containing one source offset.
    pub(crate) fn line_start(&self, offset: u32) -> u32 {
        let offset = offset as usize;
        let source = self.file().text().as_bytes();
        let start = source[..offset]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);

        start as u32
    }
}

/// Create one authored test file.
pub(crate) fn test_file(name: &str, text: impl Into<String>) -> Arc<File> {
    let uri_name = name.trim_matches(['<', '>']);

    Arc::new(File::from_text(
        FileId::from_logical_str(name),
        name.to_string(),
        Uri::from_string(format!("destack:{uri_name}")),
        None,
        FileType::Destack,
        text.into(),
    ))
}

/// Render complete source diagnostics for authored test files.
pub(crate) fn render_diagnostics(
    files: &[Arc<File>],
    diagnostics: &DiagnosticCollection,
) -> String {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let output = lines.clone();
    let writer = Arc::new(move |line: &str| {
        output
            .lock()
            .expect("diagnostic output should lock")
            .push(line.to_string());
    });
    let options = PrintOptions::new()
        .with_color(false)
        .with_skip_summary(true)
        .with_line_writer(writer);
    let file_for_id = |file_id| files.iter().find(|file| file.id == file_id).cloned();

    print_diagnostics(&file_for_id, diagnostics, options).expect("diagnostics should render");

    lines
        .lock()
        .expect("diagnostic output should lock")
        .join("\n")
}

/// Remove one opening raw fixture delimiter newline.
pub(crate) fn fixture_text(source: &str) -> &str {
    source.strip_prefix('\n').unwrap_or(source)
}
