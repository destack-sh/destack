use std::sync::Arc;

use tspp_core::StringPool;
use tspp_dir as dir;
use tspp_formatter::format_file_source;
use tspp_repository::FormatterOptions;
use tspp_source::apply_file_patch;

use super::{TestSource, fixture_text, render_diagnostics, test_file};
use crate::{Rewrite, Rewriter};

/// Authored inputs for one rewrite exercise.
pub(crate) struct TestRewriter {
    /// The structural pattern source.
    pattern: String,
    /// The structural replacement source.
    replacement: String,
    /// The candidate source.
    source: String,
    /// The contextual node selector.
    selector: Option<dir::NodeType>,
    /// The predicate sources in authored order.
    predicates: Vec<String>,
    /// Additional checked modules.
    files: Vec<(String, String)>,
}

impl TestRewriter {
    /// Create one expression rewrite exercise.
    pub(crate) fn new(pattern: &str, replacement: &str, source: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            replacement: replacement.to_string(),
            source: fixture_text(source).to_string(),
            selector: None,
            predicates: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Create one contextual rewrite exercise.
    pub(crate) fn context(
        pattern: &str,
        replacement: &str,
        selector: dir::NodeType,
        source: &str,
    ) -> Self {
        Self {
            pattern: pattern.to_string(),
            replacement: replacement.to_string(),
            source: fixture_text(source).to_string(),
            selector: Some(selector),
            predicates: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Add one module to the checked fixture.
    pub(crate) fn file(mut self, path: &str, source: &str) -> Self {
        self.files
            .push((path.to_string(), fixture_text(source).to_string()));

        self
    }

    /// Add one semantic predicate.
    pub(crate) fn guard(mut self, predicate: &str) -> Self {
        self.predicates.push(predicate.to_string());

        self
    }

    /// Assert the complete formatted rewritten source.
    #[track_caller]
    pub(crate) fn assert(self, expected: &str) {
        let rewrite = self.compile();
        let rewriter = rewrite.rewriter();
        let matches = rewrite.source.matches(rewrite.rewrite.pattern());
        let patch = rewriter.rewrite(matches).expect("rewrite test source");
        let rewritten =
            apply_file_patch(rewrite.source.file(), &patch).expect("apply test rewrite");
        let actual = format_file_source(
            rewrite.source.file(),
            &rewritten,
            FormatterOptions::default(),
        )
        .expect("format rewritten source");

        assert_eq!(actual, fixture_text(expected));
    }

    /// Assert the complete rendered rewrite diagnostics.
    #[track_caller]
    pub(crate) fn assert_diagnostics(self, expected: &str) {
        let rewrite = self.compile();
        let rewriter = rewrite.rewriter();
        let matches = rewrite.source.matches(rewrite.rewrite.pattern());
        let diagnostics = rewriter.rewrite(matches).expect_err("rewrite should fail");
        let source = Arc::new(rewrite.source.file().clone());
        let actual = render_diagnostics(&[source], &diagnostics);

        assert_eq!(actual.trim_matches('\n'), expected.trim_matches('\n'));
    }

    /// Compile the authored rewrite exercise.
    #[track_caller]
    fn compile(self) -> TestRewrite {
        let (source, strings) = if self.predicates.is_empty() {
            let strings = Arc::new(StringPool::new());
            let source = TestSource::parse(&self.source, strings.clone());

            (source, strings)
        } else {
            TestSource::checked(&self.source, &self.files)
        };
        let pattern = test_file("<pattern>", &self.pattern);
        let replacement = test_file("<replacement>", &self.replacement);
        let mut rewrite = match self.selector {
            Some(selector) => {
                Rewrite::parse_context(pattern, replacement, selector, strings.clone())
                    .expect("compile test rewrite context")
            }
            None => {
                Rewrite::parse(pattern, replacement, strings.clone()).expect("compile test rewrite")
            }
        };

        // compile predicates against the candidate program strings
        for (index, predicate) in self.predicates.iter().enumerate() {
            let name = format!("<predicate-{}>", index + 1);
            let predicate = test_file(&name, predicate);
            rewrite
                .add_predicate(predicate)
                .expect("compile test rewrite predicate");
        }

        TestRewrite { rewrite, source }
    }
}

/// Compiled state for one rewrite exercise.
struct TestRewrite {
    /// The compiled rewrite.
    rewrite: Rewrite,
    /// The candidate source.
    source: TestSource,
}

impl TestRewrite {
    /// Build the production rewriter for this exercise.
    fn rewriter(&self) -> Rewriter<'_, '_> {
        Rewriter::new(&self.rewrite, self.source.view(), self.source.file())
    }
}
