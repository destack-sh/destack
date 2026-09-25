use tspp_core::{BitSet, FxIndexMap};

use crate::{CallComponentTable, LinkEdgeKind, LinkTable, Symbol};

/// Whole-program reference graph in compressed sparse row form over a dense symbol index.
#[derive(Debug, Default)]
pub struct LinkSupergraph {
    /// Every defined symbol, sorted so each position is its dense id.
    symbols: Vec<Symbol>,
    /// Per-source start offset into `edge_targets` (length is symbols + 1).
    edge_offsets: Vec<u32>,
    /// Edge heads as dense ids, grouped by source.
    edge_targets: Vec<u32>,
    /// Reference kind parallel to `edge_targets`.
    edge_kinds: Vec<LinkEdgeKind>,
}

impl LinkSupergraph {
    /// Stitch the dense cross-module graph from per-module link graphs.
    pub fn build<'a>(members: impl IntoIterator<Item = &'a LinkTable>) -> Self {
        let members: Vec<&LinkTable> = members.into_iter().collect();

        // collect every defined symbol, sorted to assign dense ids
        let mut symbols: Vec<Symbol> = members
            .iter()
            .flat_map(|graph| graph.nodes().map(|(symbol, _)| symbol))
            .collect();
        symbols.sort_unstable();
        symbols.dedup();

        // index each symbol to its dense id for edge translation
        let index: FxIndexMap<Symbol, u32> = symbols
            .iter()
            .enumerate()
            .map(|(dense, symbol)| (*symbol, dense as u32))
            .collect();

        // count each source's edges that land on a defined symbol
        let mut edge_offsets = vec![0u32; symbols.len() + 1];
        for graph in &members {
            for (symbol, _) in graph.nodes() {
                let source = index[&symbol] as usize;
                for edge in graph.edges(symbol) {
                    if index.contains_key(&edge.target) {
                        edge_offsets[source + 1] += 1;
                    }
                }
            }
        }

        // prefix sum the counts into compressed sparse row start offsets
        for source in 0..symbols.len() {
            edge_offsets[source + 1] += edge_offsets[source];
        }

        // fill each source's edges at its advancing cursor
        let edge_count = edge_offsets[symbols.len()] as usize;
        let mut edge_targets = vec![0u32; edge_count];
        let mut edge_kinds = vec![LinkEdgeKind::Call; edge_count];
        let mut cursor: Vec<u32> = edge_offsets[..symbols.len()].to_vec();
        for graph in &members {
            for (symbol, _) in graph.nodes() {
                let source = index[&symbol] as usize;
                for edge in graph.edges(symbol) {
                    if let Some(&target) = index.get(&edge.target) {
                        let at = cursor[source] as usize;
                        edge_targets[at] = target;
                        edge_kinds[at] = edge.kind;
                        cursor[source] += 1;
                    }
                }
            }
        }

        Self {
            symbols,
            edge_offsets,
            edge_targets,
            edge_kinds,
        }
    }

    /// Return the program's defined symbols in dense-id order.
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Return the dense id of a symbol, if the program defines it.
    pub fn index_of(&self, symbol: Symbol) -> Option<usize> {
        self.symbols.binary_search(&symbol).ok()
    }

    /// Mark every symbol reachable from the roots.
    pub fn reachable(&self, roots: &[Symbol]) -> BitSet {
        let mut live = BitSet::new(self.symbols.len());
        let mut worklist: Vec<usize> = Vec::new();

        // seed the worklist with the defined roots
        for &root in roots {
            if let Some(dense) = self.index_of(root)
                && live.insert(dense)
            {
                worklist.push(dense);
            }
        }

        // walk outgoing edges until no new symbol is discovered
        while let Some(source) = worklist.pop() {
            for &target in self.successors(source) {
                if live.insert(target as usize) {
                    worklist.push(target as usize);
                }
            }
        }

        live
    }

    /// Count program-wide incoming references to each symbol, by dense id.
    pub fn reference_counts(&self) -> Vec<u32> {
        let mut counts = vec![0u32; self.symbols.len()];
        for &target in &self.edge_targets {
            counts[target as usize] += 1;
        }

        counts
    }

    /// Mark every symbol whose address is taken anywhere in the program.
    pub fn address_taken(&self) -> BitSet {
        let mut taken = BitSet::new(self.symbols.len());
        for (position, kind) in self.edge_kinds.iter().enumerate() {
            if *kind == LinkEdgeKind::Address {
                taken.insert(self.edge_targets[position] as usize);
            }
        }

        taken
    }

    /// Mark every defined symbol internal to the program (not an external root).
    pub fn internal(&self, roots: &[Symbol]) -> BitSet {
        // collect the dense ids of the program's external roots
        let mut is_root = BitSet::new(self.symbols.len());
        for &root in roots {
            if let Some(dense) = self.index_of(root) {
                is_root.insert(dense);
            }
        }

        // classify unexported symbols as internal
        let mut internal = BitSet::new(self.symbols.len());
        for dense in 0..self.symbols.len() {
            if !is_root.contains(dense) {
                internal.insert(dense);
            }
        }

        internal
    }

    /// Condense call edges into strongly connected components.
    pub fn call_components(&self) -> CallComponentTable {
        // select the call edges from the symbol graph
        let (offsets, targets) = self.call_edges();

        CallComponentTable::analyse(&offsets, &targets)
    }

    /// Return the call-only edge graph in dense CSR form.
    fn call_edges(&self) -> (Vec<u32>, Vec<u32>) {
        let mut edge_offsets = vec![0u32; self.symbols.len() + 1];

        // count direct call edges per symbol
        for source in 0..self.symbols.len() {
            let start = self.edge_offsets[source] as usize;
            let end = self.edge_offsets[source + 1] as usize;
            let count = self.edge_kinds[start..end]
                .iter()
                .filter(|kind| **kind == LinkEdgeKind::Call)
                .count();
            edge_offsets[source + 1] = count as u32;
        }

        // prefix sum the counts into CSR offsets
        for source in 0..self.symbols.len() {
            edge_offsets[source + 1] += edge_offsets[source];
        }

        // copy direct call targets into the call-only graph
        let mut edge_targets = Vec::with_capacity(edge_offsets[self.symbols.len()] as usize);
        for source in 0..self.symbols.len() {
            let start = self.edge_offsets[source] as usize;
            let end = self.edge_offsets[source + 1] as usize;
            for position in start..end {
                if self.edge_kinds[position] == LinkEdgeKind::Call {
                    edge_targets.push(self.edge_targets[position]);
                }
            }
        }

        (edge_offsets, edge_targets)
    }

    /// Return the dense ids reachable in one hop from a source.
    fn successors(&self, source: usize) -> &[u32] {
        let start = self.edge_offsets[source] as usize;
        let end = self.edge_offsets[source + 1] as usize;

        &self.edge_targets[start..end]
    }
}
