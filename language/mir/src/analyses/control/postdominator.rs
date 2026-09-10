use std::sync::Arc;

use super::control::ControlGraph;
use super::dominance::DominatorTree;
use crate::{Analysis, Block, ControlTable, LocalNodeId, Mutation};

/// Postdominator tree for all blocks, rooted at terminal blocks and selected exitless blocks.
#[derive(Debug, Clone)]
pub struct PostdominatorTable {
    /// Control flow and the shared mapping from MIR ids to compact indices.
    control: Arc<ControlTable>,
    /// Dominator links and intervals, with the virtual exit after the function blocks.
    tree: DominatorTree,
}

impl PostdominatorTable {
    /// Analyse control flow to find postdominators, including regions that cannot reach an exit.
    pub fn analyse(control: Arc<ControlTable>) -> Self {
        // reverse the edges in compact function indices
        let count = control.graph.len() as u32;
        let mut offsets = Vec::with_capacity(count as usize + 2);
        let mut successors = Vec::new();
        offsets.push(0);

        // append the original predecessors as reverse successors
        for block in 0..count {
            successors.extend_from_slice(control.graph.predecessors(block));
            offsets.push(successors.len() as u32);
        }

        // connect the virtual exit to each selected root
        successors.extend(Self::select_roots(&control.graph));
        offsets.push(successors.len() as u32);
        let graph = ControlGraph::new(offsets, successors);

        // compute dominance from the virtual exit
        let postorder = graph.postorder(count);
        let tree = DominatorTree::build(Some(count), &postorder, &graph);

        Self { control, tree }
    }

    /// Return the immediate postdominator, or none when the parent is the virtual exit.
    pub fn immediate_postdominator(&self, block: LocalNodeId<Block>) -> Option<LocalNodeId<Block>> {
        let index = self.control.index(block);
        let parent = self.tree.nodes[index as usize].parent;

        parent
            .filter(|&parent| parent != self.control.graph.len() as u32)
            .map(|parent| self.control.block(parent))
    }

    /// Return whether one block postdominates another in the graph with selected exits.
    pub fn postdominates(
        &self,
        postdominator: LocalNodeId<Block>,
        block: LocalNodeId<Block>,
    ) -> bool {
        let postdominator = self.control.index(postdominator);
        let block = self.control.index(block);

        self.tree.dominates(postdominator, block)
    }

    /// Select terminal blocks and nonredundant roots in regions that cannot reach a terminal.
    fn select_roots(graph: &ControlGraph) -> Vec<u32> {
        // visit terminal blocks before selecting roots in exitless regions
        let count = graph.len() as u32;
        let exits = (0..count).filter(|&block| graph.successors(block).is_empty());
        let mut roots = Vec::new();
        let mut visited = vec![false; graph.len()];
        let mut stack = Vec::new();
        let mut forward = Vec::new();

        // cover each block by walking backwards from a selected root
        for start in exits.chain(0..count) {
            if visited[start as usize] {
                continue;
            }

            // select the last forward DFS visit when no terminal is reachable
            let mut root = start;
            if !graph.successors(start).is_empty() {
                stack.push(start);
                while let Some(block) = stack.pop() {
                    if visited[block as usize] {
                        continue;
                    }

                    // record this visit so its mark can be removed before the reverse walk
                    visited[block as usize] = true;
                    forward.push(block);
                    root = block;

                    // order successors by function position to keep root selection stable
                    let offset = stack.len();
                    stack.extend_from_slice(graph.successors(block));
                    stack[offset..].sort_unstable();
                }

                // clear only the temporary forward visits
                for block in forward.drain(..) {
                    visited[block as usize] = false;
                }
            }

            // mark every block that can reach the selected root
            roots.push(root);
            stack.push(root);
            while let Some(block) = stack.pop() {
                if !visited[block as usize] {
                    visited[block as usize] = true;
                    stack.extend_from_slice(graph.predecessors(block));
                }
            }
        }

        // index root membership for the redundancy searches
        let mut is_root = vec![false; graph.len()];
        for &root in &roots {
            is_root[root as usize] = true;
        }
        visited.fill(false);
        let mut index = 0;

        // remove a nonterminal root when it can reach another selected root
        while index < roots.len() {
            let root = roots[index];
            if graph.successors(root).is_empty() {
                index += 1;
                continue;
            }

            // search forward until another root is found
            let mut is_redundant = false;
            stack.push(root);
            while let Some(block) = stack.pop() {
                if visited[block as usize] {
                    continue;
                }

                // stop at a distinct root, including one in the same cycle
                if block != root && is_root[block as usize] {
                    is_redundant = true;
                    break;
                }

                // record each visit for the next search
                visited[block as usize] = true;
                forward.push(block);
                stack.extend_from_slice(graph.successors(block));
            }

            // clear the search without scanning unrelated blocks
            stack.clear();
            for block in forward.drain(..) {
                visited[block as usize] = false;
            }

            // recheck the last root when it replaces a redundant root
            if is_redundant {
                is_root[root as usize] = false;
                roots.swap_remove(index);
            }
            // advance after retaining a root
            else {
                index += 1;
            }
        }

        roots
    }
}

impl Analysis for PostdominatorTable {
    const INVALIDATED_BY: Mutation = ControlTable::INVALIDATED_BY;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Make the join postdominate both branches and their entry.
    #[test]
    fn test_find_common_postdominators() {
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
        let table = PostdominatorTable::analyse(control);

        // collect each block's immediate postdominator and complete postdominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.postdominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_postdominator(block), ancestors)
        });
        let expected = [
            (entry, Some(join), vec![entry, join]),
            (left, Some(join), vec![left, join]),
            (right, Some(join), vec![right, join]),
            (join, None, vec![join]),
        ];
        assert_eq!(actual, expected);
    }

    /// Keep a return and an unreachable terminator as separate exits.
    #[test]
    fn test_separate_terminal_exits() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => left | right

left:
    jump done

right:
    unreachable

done:
    return
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, left, right, done]: [_; 4] = function.blocks().try_into().unwrap();
        let blocks = [entry, left, right, done];
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = PostdominatorTable::analyse(control);

        // collect each block's immediate postdominator and complete postdominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.postdominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_postdominator(block), ancestors)
        });
        let expected = [
            (entry, None, vec![entry]),
            (left, Some(done), vec![left, done]),
            (right, None, vec![right]),
            (done, None, vec![done]),
        ];
        assert_eq!(actual, expected);
    }

    /// Keep the return separate from a reachable infinite loop and include disconnected blocks.
    #[test]
    fn test_separate_returning_and_exitless_paths() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => done | header

header:
    jump body

body:
    jump header

done:
    return

dead:
    jump done
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, header, body, done, dead]: [_; 5] = function.blocks().try_into().unwrap();
        let blocks = [entry, header, body, done, dead];
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = PostdominatorTable::analyse(control);

        // collect each block's immediate postdominator and complete postdominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.postdominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_postdominator(block), ancestors)
        });
        let expected = [
            (entry, None, vec![entry]),
            (header, Some(body), vec![header, body]),
            (body, None, vec![body]),
            (done, None, vec![done]),
            (dead, Some(done), vec![done, dead]),
        ];
        assert_eq!(actual, expected);
    }

    /// Discard an exitless root that can reach another selected root.
    #[test]
    fn test_remove_redundant_exitless_roots() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => left | merge

left:
    jump middle

middle:
    jump merge

merge:
    jump spin

spin:
    jump spin
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, left, middle, merge, spin]: [_; 5] = function.blocks().try_into().unwrap();
        let blocks = [entry, left, middle, merge, spin];
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = PostdominatorTable::analyse(control);

        // collect each block's immediate postdominator and complete postdominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.postdominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_postdominator(block), ancestors)
        });
        let expected = [
            (entry, Some(merge), vec![entry, merge, spin]),
            (left, Some(middle), vec![left, middle, merge, spin]),
            (middle, Some(merge), vec![middle, merge, spin]),
            (merge, Some(spin), vec![merge, spin]),
            (spin, None, vec![spin]),
        ];
        assert_eq!(actual, expected);
    }

    /// Select an exitless root by function block order despite reversed branch targets.
    #[test]
    fn test_select_exitless_roots_in_function_order() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump header

header:
    branch v0 => right | left

left:
    jump header

right:
    jump header
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, header, left, right]: [_; 4] = function.blocks().try_into().unwrap();
        let blocks = [entry, header, left, right];
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = PostdominatorTable::analyse(control);

        // collect each block's immediate postdominator and complete postdominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.postdominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_postdominator(block), ancestors)
        });
        let expected = [
            (entry, Some(header), vec![entry, header, left]),
            (header, Some(left), vec![header, left]),
            (left, None, vec![left]),
            (right, Some(header), vec![header, left, right]),
        ];
        assert_eq!(actual, expected);
    }

    /// Find postdominators along paths to a loop's terminal exit.
    #[test]
    fn test_find_postdominators_in_loop_with_exit() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump header

header:
    branch v0 => body | done

body:
    jump header

done:
    return
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, header, body, done]: [_; 4] = function.blocks().try_into().unwrap();
        let blocks = [entry, header, body, done];
        let control = Arc::new(ControlTable::analyse(function, &tree));
        let table = PostdominatorTable::analyse(control);

        // collect each block's immediate postdominator and complete postdominator set
        let actual = blocks.map(|block| {
            let ancestors = blocks
                .into_iter()
                .filter(|&candidate| table.postdominates(candidate, block))
                .collect::<Vec<_>>();

            (block, table.immediate_postdominator(block), ancestors)
        });
        let expected = [
            (entry, Some(header), vec![entry, header, done]),
            (header, Some(done), vec![header, done]),
            (body, Some(header), vec![header, body, done]),
            (done, None, vec![done]),
        ];
        assert_eq!(actual, expected);
    }
}
