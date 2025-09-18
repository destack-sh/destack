use dyst_language_ast::{NodeTree, PathPool, StringPool};

#[derive(Debug, Copy, Clone)]
pub struct FormatterOptions {
	/// Spaces per indent.
	indent: u32,
	/// Maximum line length.
	line_length: u32,
}

/// A formatter for a NodeTree.
#[derive(Debug, Clone)]
pub struct Formatter<'a> {
    ast: &'a NodeTree,
    strings: &'a StringPool,
    paths: &'a PathPool,
	options: FormatterOptions,
}
