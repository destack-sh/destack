use crate::{LocalNodeId, Rule, Stylesheet, Tree};

/// Print one stylesheet subtree as canonical CSS source.
pub fn print_stylesheet(tree: &Tree, stylesheet: LocalNodeId<Stylesheet>) -> String {
    let mut printer = Printer::new(tree);
    printer.print_stylesheet_id(stylesheet);
    printer.finish()
}

/// Print one CSS rule subtree as canonical CSS source.
pub fn print_rule(tree: &Tree, rule: LocalNodeId<Rule>) -> String {
    let mut printer = Printer::new(tree);
    printer.print_rule_id(rule);
    printer.finish()
}

/// One canonical CSS printer.
#[derive(Debug)]
pub(crate) struct Printer<'a> {
    /// The CSS tree being printed.
    pub(crate) tree: &'a Tree,
    /// The emitted CSS source.
    pub(crate) source: String,
}

impl<'a> Printer<'a> {
    /// Create one CSS printer.
    pub(crate) fn new(tree: &'a Tree) -> Self {
        Self {
            tree,
            source: String::new(),
        }
    }

    /// Finish this print pass.
    pub(crate) fn finish(self) -> String {
        self.source
    }

    /// Print one stylesheet node.
    pub(crate) fn print_stylesheet_id(&mut self, stylesheet_id: LocalNodeId<Stylesheet>) {
        let stylesheet = self.tree.get(stylesheet_id);

        // top level rules
        for rule in &stylesheet.rules {
            self.print_rule_id(*rule);
        }
    }
}
