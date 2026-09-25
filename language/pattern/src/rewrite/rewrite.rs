use std::sync::Arc;

use tspp_core::StringPool;
use tspp_dir as dir;
use tspp_source::{DiagnosticCollection, File};

use crate::compile::{Compiler, FragmentRole};
use crate::{Pattern, Replacement};

/// A search pattern and structural replacement.
#[derive(Debug)]
pub struct Rewrite {
    /// The search pattern.
    pub(crate) pattern: Pattern,
    /// The structural replacement.
    pub(crate) replacement: Replacement,
}

impl Rewrite {
    /// Parse an expression rewrite.
    pub fn parse(
        pattern: Arc<File>,
        replacement: Arc<File>,
        strings: Arc<StringPool>,
    ) -> Result<Self, DiagnosticCollection> {
        let mut compiler = Compiler::new(strings);
        let pattern = compiler.compile_expression(pattern, FragmentRole::Pattern)?;
        let replacement_fragment =
            compiler.compile_expression(replacement.clone(), FragmentRole::Replacement)?;
        let pattern = compiler.finish(pattern);

        Ok(Self {
            pattern,
            replacement: Replacement {
                file: replacement,
                fragment: replacement_fragment,
            },
        })
    }

    /// Parse a contextual rewrite with one selected node type.
    pub fn parse_context(
        pattern: Arc<File>,
        replacement: Arc<File>,
        selector: dir::NodeType,
        strings: Arc<StringPool>,
    ) -> Result<Self, DiagnosticCollection> {
        let mut compiler = Compiler::new(strings);
        let pattern = compiler.compile_context(pattern, selector, FragmentRole::Pattern)?;
        let replacement_fragment =
            compiler.compile_context(replacement.clone(), selector, FragmentRole::Replacement)?;
        let pattern = compiler.finish(pattern);

        Ok(Self {
            pattern,
            replacement: Replacement {
                file: replacement,
                fragment: replacement_fragment,
            },
        })
    }

    /// Add one predicate expression to the search pattern.
    pub fn add_predicate(&mut self, file: Arc<File>) -> Result<(), DiagnosticCollection> {
        self.pattern.add_predicate(file)
    }

    /// Return the search pattern.
    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    /// Return the structural replacement.
    pub fn replacement(&self) -> &Replacement {
        &self.replacement
    }
}
