use clap::ValueEnum;

/// Individual dump format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DumpKind {
    /// Dump the file representation (colorized source).
    #[value(alias = "f")]
    File,
    /// Dump the node representation.
    #[value(alias = "n")]
    Node,
    /// Dump the symbol representation.
    #[value(alias = "s")]
    Symbol,
    /// Dump all representations.
    #[value(alias = "a")]
    All,
}

/// Dump format configuration parsed from CLI args.
#[derive(Debug, Clone)]
pub struct DumpFormat {
    kinds: Vec<DumpKind>,
}

impl DumpFormat {
    /// Create from a list of dump kinds.
    pub fn new(kinds: Vec<DumpKind>) -> Self {
        Self { kinds }
    }

    /// Whether the format includes the file representation.
    pub fn includes_file(&self) -> bool {
        self.kinds
            .iter()
            .any(|k| matches!(k, DumpKind::File | DumpKind::All))
    }

    /// Whether the format includes the node representation.
    pub fn includes_node(&self) -> bool {
        self.kinds
            .iter()
            .any(|k| matches!(k, DumpKind::Node | DumpKind::All))
    }

    /// Whether the format includes the symbol representation.
    pub fn includes_symbol(&self) -> bool {
        self.kinds
            .iter()
            .any(|k| matches!(k, DumpKind::Symbol | DumpKind::All))
    }
}

impl From<Vec<DumpKind>> for DumpFormat {
    fn from(kinds: Vec<DumpKind>) -> Self {
        Self::new(kinds)
    }
}
