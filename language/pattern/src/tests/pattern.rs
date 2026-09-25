use std::sync::Arc;

use tspp_core::StringPool;
use tspp_dir as dir;
use tspp_source::{DiagnosticCollection, File};

use super::{TestCompilation, test_file};
use crate::Pattern;

/// One authored pattern compilation fixture.
pub(crate) struct TestPattern {
    /// The strings shared by every authored input.
    strings: Arc<StringPool>,
    /// The structural pattern source.
    pattern: Arc<File>,
    /// The contextual node selector.
    selector: Option<dir::NodeType>,
    /// The predicate sources in authored order.
    predicates: Vec<Arc<File>>,
}

impl TestPattern {
    /// Create one expression pattern fixture.
    pub(crate) fn new(text: &str) -> Self {
        Self {
            strings: Arc::new(StringPool::new()),
            pattern: test_file("<pattern>", text),
            selector: None,
            predicates: Vec::new(),
        }
    }

    /// Create one contextual pattern fixture.
    pub(crate) fn context(text: &str, selector: dir::NodeType) -> Self {
        Self {
            strings: Arc::new(StringPool::new()),
            pattern: test_file("<pattern>", text),
            selector: Some(selector),
            predicates: Vec::new(),
        }
    }

    /// Add one predicate source.
    pub(crate) fn predicate(mut self, text: &str) -> Self {
        let index = self.predicates.len();
        let name = if index == 0 {
            "<predicate>".to_string()
        } else {
            format!("<predicate-{}>", index + 1)
        };
        self.predicates.push(test_file(&name, text));

        self
    }

    /// Compile every authored input.
    pub(crate) fn compile(self) -> TestCompilation {
        let result = self.compile_pattern();
        let mut files = vec![self.pattern.clone()];
        files.extend(self.predicates.iter().cloned());

        TestCompilation::new(self.strings, files, result)
    }

    /// Compile the structural pattern and its predicates.
    fn compile_pattern(&self) -> Result<Pattern, DiagnosticCollection> {
        let mut pattern = match self.selector {
            Some(selector) => {
                Pattern::parse_context(self.pattern.clone(), selector, self.strings.clone())?
            }
            None => Pattern::parse(self.pattern.clone(), self.strings.clone())?,
        };

        // compile predicates into the shared metavariable table
        for predicate in &self.predicates {
            pattern.add_predicate(predicate.clone())?;
        }

        Ok(pattern)
    }
}
