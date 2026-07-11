use destack_fir::format::FormatNode;

/// Formatter document node stats for one completed FIR document.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct FormatterDocumentStats {
    /// Top level document node count.
    top_level_nodes: u64,
    /// Recursive document node count including interned and best fitting payloads.
    recursive_nodes: u64,
    /// Space nodes.
    spaces: u64,
    /// Line nodes.
    lines: u64,
    /// Expand-parent nodes.
    expand_parents: u64,
    /// Static token nodes.
    tokens: u64,
    /// Dynamic text nodes.
    texts: u64,
    /// Source position nodes.
    source_positions: u64,
    /// Verbatim file slice nodes.
    file_slices: u64,
    /// Line suffix boundary nodes.
    line_suffix_boundaries: u64,
    /// Interned references in the node stream.
    interned_refs: u64,
    /// Interned payloads traversed while counting.
    interned_payloads: u64,
    /// Nodes inside interned payloads.
    interned_payload_nodes: u64,
    /// Best fitting nodes.
    best_fitting: u64,
    /// Best fitting variants traversed while counting.
    best_fitting_variants: u64,
    /// Nodes inside best fitting variants.
    best_fitting_variant_nodes: u64,
    /// Formatting tag nodes.
    tags: u64,
}

impl FormatterDocumentStats {
    /// Count nodes in one completed FIR document.
    pub(super) fn from_nodes(nodes: &[FormatNode<'_>]) -> Self {
        let mut stats = Self {
            top_level_nodes: nodes.len() as u64,
            ..Self::default()
        };
        stats.add_nodes_iterative(nodes);

        stats
    }

    /// Format document stats for terminal output.
    pub(super) fn format(self) -> String {
        format!(
            "top {}, recursive {}, token {}, text {}, space {}, line {}, tag {}, interned refs {}, interned payloads {} / {} nodes, best fitting {} / {} variants / {} nodes",
            self.top_level_nodes,
            self.recursive_nodes,
            self.tokens,
            self.texts,
            self.spaces,
            self.lines,
            self.tags,
            self.interned_refs,
            self.interned_payloads,
            self.interned_payload_nodes,
            self.best_fitting,
            self.best_fitting_variants,
            self.best_fitting_variant_nodes
        )
    }

    /// Add one node slice and its nested payloads to this document stats.
    fn add_nodes_iterative(&mut self, nodes: &[FormatNode<'_>]) {
        let mut pending = vec![nodes];

        while let Some(nodes) = pending.pop() {
            for node in nodes {
                self.recursive_nodes += 1;

                match node {
                    FormatNode::Space => self.spaces += 1,
                    FormatNode::Line(_) => self.lines += 1,
                    FormatNode::ExpandParent => self.expand_parents += 1,
                    FormatNode::Token { .. } => self.tokens += 1,
                    FormatNode::Text { .. } => self.texts += 1,
                    FormatNode::SourcePosition { .. } => self.source_positions += 1,
                    FormatNode::FileSlice { .. } => self.file_slices += 1,
                    FormatNode::LineSuffixBoundary => self.line_suffix_boundaries += 1,
                    FormatNode::Interned(interned) => {
                        self.interned_refs += 1;
                        self.interned_payloads += 1;
                        self.interned_payload_nodes += interned.len() as u64;
                        pending.push(interned);
                    }
                    FormatNode::BestFitting { variants, .. } => {
                        self.best_fitting += 1;
                        self.best_fitting_variants += variants.as_slice().len() as u64;

                        for variant in variants.as_slice() {
                            self.best_fitting_variant_nodes += variant.len() as u64;
                            pending.push(variant);
                        }
                    }
                    FormatNode::Tag(_) => self.tags += 1,
                }
            }
        }
    }
}
