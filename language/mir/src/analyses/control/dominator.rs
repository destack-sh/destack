use std::sync::Arc;

use tspp_core::{FxIndexMap, FxIndexSet};

use super::dominance::DominatorTree;
use crate::{Analysis, Block, ControlTable, LocalNodeId, Mutation};

/// Dominator tree for the blocks reachable from one function entry.
#[derive(Debug, Clone)]
pub struct DominatorTable {
    /// Control flow and the shared mapping from MIR ids to compact indices.
    control: Arc<ControlTable>,
    /// Dominator links and intervals in the control graph's compact indices.
    tree: DominatorTree,
}

impl DominatorTable {
    /// Analyse control flow to find dominators and ordered tree intervals.
    pub fn analyse(control: Arc<ControlTable>) -> Self {
        let tree = DominatorTree::build(control.entry, &control.postorder, &control.graph);

        Self { control, tree }
    }

    /// Return the immediate dominator, or none for entry and unreachable blocks.
    pub fn immediate_dominator(&self, block: LocalNodeId<Block>) -> Option<LocalNodeId<Block>> {
        let index = self.control.index(block) as usize;
        let node = &self.tree.nodes[index];

        node.parent.map(|index| self.control.block(index))
    }

    /// Return the first dominator-tree child in reverse postorder.
    pub fn child(&self, block: LocalNodeId<Block>) -> Option<LocalNodeId<Block>> {
        let index = self.control.index(block) as usize;
        let node = &self.tree.nodes[index];

        node.child.map(|index| self.control.block(index))
    }

    /// Return the next dominator-tree sibling.
    pub fn sibling(&self, block: LocalNodeId<Block>) -> Option<LocalNodeId<Block>> {
        let index = self.control.index(block) as usize;
        let node = &self.tree.nodes[index];

        node.sibling.map(|index| self.control.block(index))
    }

    /// Return whether a reachable block dominates another reachable block.
    pub fn dominates(&self, dominator: LocalNodeId<Block>, block: LocalNodeId<Block>) -> bool {
        let dominator = self.control.index(dominator);
        let block = self.control.index(block);

        self.tree.dominates(dominator, block)
    }

    /// Return whether a block dominates another distinct block.
    pub fn strictly_dominates(
        &self,
        dominator: LocalNodeId<Block>,
        block: LocalNodeId<Block>,
    ) -> bool {
        dominator != block && self.dominates(dominator, block)
    }

    /// Return the tree entry number, or none for an unreachable block.
    pub fn entry(&self, block: LocalNodeId<Block>) -> Option<u32> {
        let index = self.control.index(block) as usize;
        let entry = self.tree.nodes[index].entry;

        (entry != 0).then_some(entry)
    }

    /// Return the tree exit number, or none for an unreachable block.
    pub fn exit(&self, block: LocalNodeId<Block>) -> Option<u32> {
        let index = self.control.index(block) as usize;
        let exit = self.tree.nodes[index].exit;

        (exit != 0).then_some(exit)
    }

    /// Compute dominance frontiers for every reachable block.
    pub fn frontiers(&self) -> FxIndexMap<LocalNodeId<Block>, FxIndexSet<LocalNodeId<Block>>> {
        // allocate frontier sets in the graph's compact index space
        let mut frontiers = vec![FxIndexSet::default(); self.tree.nodes.len()];

        // walk each reachable predecessor up to the target's immediate dominator
        for &block in &self.control.postorder {
            let parent = self.tree.nodes[block as usize].parent;

            // skip predecessors outside the entry traversal
            for &predecessor in self.control.graph.predecessors(block) {
                if self.tree.nodes[predecessor as usize].entry == 0 {
                    continue;
                }

                // include self-frontiers and edges returning to the entry
                let mut runner = Some(predecessor);
                while let Some(current) = runner {
                    if Some(current) == parent {
                        break;
                    }

                    // stop when an earlier predecessor already visited this ancestor chain
                    if !frontiers[current as usize].insert(block) {
                        break;
                    }

                    // ascend after adding the target to this block's frontier
                    runner = self.tree.nodes[current as usize].parent;
                }
            }
        }

        // expose MIR block ids in function order
        frontiers
            .into_iter()
            .enumerate()
            .filter_map(|(index, frontier)| {
                // omit unreachable blocks from the frontier table
                if self.tree.nodes[index].entry == 0 {
                    return None;
                }

                // translate each frontier member to its MIR id
                let block = self.control.block(index as u32);
                let frontier = frontier
                    .into_iter()
                    .map(|index| self.control.block(index))
                    .collect();

                Some((block, frontier))
            })
            .collect()
    }
}

impl Analysis for DominatorTable {
    const INVALIDATED_BY: Mutation = ControlTable::INVALIDATED_BY;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Find the common dominator of two branches and their join.
    #[test]
    fn test_find_common_dominators() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => left | right

left:
    jump join

right:
    jump join

join:
    return
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, left, right, join]: [_; 4] = function.blocks().try_into().unwrap();
        let blocks = [entry, left, right, join];
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = DominatorTable::analyse(control);

        // collect each block's immediate dominator and complete dominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.dominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_dominator(block), ancestors)
        });
        let expected = [
            (entry, None, vec![entry]),
            (left, Some(entry), vec![entry, left]),
            (right, Some(entry), vec![entry, right]),
            (join, Some(entry), vec![entry, join]),
        ];
        assert_eq!(actual, expected);
    }

    /// Exclude an unreachable predecessor from the dominator tree.
    #[test]
    fn test_exclude_unreachable_blocks() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(): void {
entry:
    jump done

done:
    return

dead:
    jump done
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, done, dead]: [_; 3] = function.blocks().try_into().unwrap();
        let blocks = [entry, done, dead];
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = DominatorTable::analyse(control);

        // collect each block's immediate dominator and complete dominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.dominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_dominator(block), ancestors)
        });
        let expected = [
            (entry, None, vec![entry]),
            (done, Some(entry), vec![entry, done]),
            (dead, None, vec![]),
        ];
        assert_eq!(actual, expected);
        assert_eq!(
            (
                table.child(dead),
                table.sibling(dead),
                table.entry(dead),
                table.exit(dead)
            ),
            (None, None, None, None),
        );
    }

    /// Find dominators in a loop entered through two different blocks.
    #[test]
    fn test_find_dominators_in_irreducible_loop() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => left | right

left:
    jump merge

right:
    jump merge

merge:
    branch v0 => left | done

done:
    return
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, left, right, merge, done]: [_; 5] = function.blocks().try_into().unwrap();
        let blocks = [entry, left, right, merge, done];
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = DominatorTable::analyse(control);

        // collect each block's immediate dominator and complete dominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.dominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_dominator(block), ancestors)
        });
        let expected = [
            (entry, None, vec![entry]),
            (left, Some(entry), vec![entry, left]),
            (right, Some(entry), vec![entry, right]),
            (merge, Some(entry), vec![entry, merge]),
            (done, Some(merge), vec![entry, merge, done]),
        ];
        assert_eq!(actual, expected);
    }

    /// Preserve the dominator chain through nested loops.
    #[test]
    fn test_find_dominators_in_nested_loops() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump outer

outer:
    branch v0 => inner | done

inner:
    branch v0 => body | latch

body:
    jump inner

latch:
    jump outer

done:
    return
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, outer, inner, body, latch, done]: [_; 6] =
            function.blocks().try_into().unwrap();
        let blocks = [entry, outer, inner, body, latch, done];
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = DominatorTable::analyse(control);

        // collect each block's immediate dominator and complete dominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.dominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_dominator(block), ancestors)
        });
        let expected = [
            (entry, None, vec![entry]),
            (outer, Some(entry), vec![entry, outer]),
            (inner, Some(outer), vec![entry, outer, inner]),
            (body, Some(inner), vec![entry, outer, inner, body]),
            (latch, Some(inner), vec![entry, outer, inner, latch]),
            (done, Some(outer), vec![entry, outer, done]),
        ];
        assert_eq!(actual, expected);
    }

    /// Visit dominator children in reverse postorder after reordering function blocks.
    #[test]
    fn test_order_dominator_children() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => left | right

left:
    jump join

right:
    jump join

join:
    return
}
"#,
        );
        let mut function = tree.get(function_id).clone();
        let [entry, left, right, join]: [_; 4] = function.blocks().try_into().unwrap();
        let blocks = [entry, left, right, join];
        function.replace_blocks(blocks.into_iter().rev().collect(), &tree);
        let control = Arc::new(ControlTable::analyse(&function, &tree));
        let table = DominatorTable::analyse(control);

        let actual = blocks.map(|block| (block, table.child(block), table.sibling(block)));
        assert_eq!(
            actual,
            [
                (entry, Some(right), None),
                (left, None, Some(join)),
                (right, None, Some(left)),
                (join, None, None),
            ]
        );
    }

    /// Put the join in both branch frontiers.
    #[test]
    fn test_find_frontiers_at_join() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => left | right

left:
    jump join

right:
    jump join

join:
    return
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, left, right, join]: [_; 4] = function.blocks().try_into().unwrap();
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = DominatorTable::analyse(control);

        let expected = FxIndexMap::from_iter([
            (entry, FxIndexSet::default()),
            (left, FxIndexSet::from_iter([join])),
            (right, FxIndexSet::from_iter([join])),
            (join, FxIndexSet::default()),
        ]);
        assert_eq!(table.frontiers(), expected);
    }

    /// Include self-frontiers and entry backedges while excluding unreachable predecessors.
    #[test]
    fn test_find_frontiers_at_backedges() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => loop | done

loop:
    branch v0 => loop | entry(v0)

done:
    return

dead:
    jump loop
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, header, done, _]: [_; 4] = function.blocks().try_into().unwrap();
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = DominatorTable::analyse(control);

        let expected = FxIndexMap::from_iter([
            (entry, FxIndexSet::from_iter([entry])),
            (header, FxIndexSet::from_iter([entry, header])),
            (done, FxIndexSet::default()),
        ]);
        assert_eq!(table.frontiers(), expected);
    }

    /// Find common dominators when branches have cross edges before rejoining.
    #[test]
    fn test_find_dominators_across_cross_edges() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump fork

fork:
    branch v0 => first | second

first:
    jump merge

second:
    branch v0 => hub | merge

merge:
    jump hub

hub:
    branch v0 => tail | done

tail:
    jump done

done:
    return
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, fork, first, second, merge, hub, tail, done]: [_; 8] =
            function.blocks().try_into().unwrap();
        let blocks = [entry, fork, first, second, merge, hub, tail, done];
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = DominatorTable::analyse(control);

        // collect each block's immediate dominator and complete dominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.dominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_dominator(block), ancestors)
        });
        let expected = [
            (entry, None, vec![entry]),
            (fork, Some(entry), vec![entry, fork]),
            (first, Some(fork), vec![entry, fork, first]),
            (second, Some(fork), vec![entry, fork, second]),
            (merge, Some(fork), vec![entry, fork, merge]),
            (hub, Some(fork), vec![entry, fork, hub]),
            (tail, Some(hub), vec![entry, fork, hub, tail]),
            (done, Some(hub), vec![entry, fork, hub, done]),
        ];
        assert_eq!(actual, expected);
    }

    /// Put a shared exit in each nested branch's frontier.
    #[test]
    fn test_find_frontiers_for_nested_branches() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump first

first:
    branch v0 => second | join

second:
    branch v0 => third | join

third:
    jump join

join:
    return
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, first, second, third, join]: [_; 5] = function.blocks().try_into().unwrap();
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = DominatorTable::analyse(control);

        let expected = FxIndexMap::from_iter([
            (entry, FxIndexSet::default()),
            (first, FxIndexSet::default()),
            (second, FxIndexSet::from_iter([join])),
            (third, FxIndexSet::from_iter([join])),
            (join, FxIndexSet::default()),
        ]);
        assert_eq!(table.frontiers(), expected);
    }
}
