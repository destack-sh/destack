use crate::{LocalNodeId, NodeTree, Rule, StyleSheet};

/// One CSS source rendering mode.
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderOptions {
    /// Whether top level separators should be minified.
    pub is_minified: bool,
}

/// Print one stylesheet subtree as canonical CSS source.
pub fn print_stylesheet(tree: &NodeTree, stylesheet: LocalNodeId<StyleSheet>) -> String {
    print_stylesheet_with_options(tree, stylesheet, RenderOptions::default())
}

/// Print one stylesheet subtree as canonical CSS source with options.
pub fn print_stylesheet_with_options(
    tree: &NodeTree,
    stylesheet: LocalNodeId<StyleSheet>,
    options: RenderOptions,
) -> String {
    let mut printer = Printer::new(tree, options);
    printer.print_stylesheet_id(stylesheet);
    printer.finish()
}

/// Print one CSS rule subtree as canonical CSS source with options.
pub fn print_rule_with_options(
    tree: &NodeTree,
    rule: LocalNodeId<Rule>,
    options: RenderOptions,
) -> String {
    let mut printer = Printer::new(tree, options);
    printer.print_rule_id(rule);
    printer.finish()
}

/// One canonical CSS printer.
#[derive(Debug)]
pub(crate) struct Printer<'a> {
    /// The CSS tree being printed.
    pub(crate) tree: &'a NodeTree,
    /// The print options.
    pub(crate) options: RenderOptions,
    /// The emitted CSS source.
    pub(crate) source: String,
}

impl<'a> Printer<'a> {
    /// Create one CSS printer.
    pub(crate) fn new(tree: &'a NodeTree, options: RenderOptions) -> Self {
        Self {
            tree,
            options,
            source: String::new(),
        }
    }

    /// Finish this print pass.
    pub(crate) fn finish(self) -> String {
        self.source
    }

    /// Print one stylesheet node.
    pub(crate) fn print_stylesheet_id(&mut self, stylesheet_id: LocalNodeId<StyleSheet>) {
        let stylesheet = self.tree.get(stylesheet_id);

        // top level rules
        for rule in &stylesheet.rules {
            if !self.source.is_empty() && !self.options.is_minified {
                self.source.push('\n');
            }

            self.print_rule_id(*rule);
        }
    }
}
