use std::hash::Hasher;

use tspp_core::StableHasher;
use tspp_dir as dir;
use tspp_dir::NodeFold;
use tspp_repository::{ProviderError, ProviderResult};

use super::ModuleIndexContext;

const ERASED_NAME: dir::StringId = dir::StringId(0);

/// Builder for one module code index.
pub(crate) struct CodeIndexer<'a, 'context> {
    /// The checked module being indexed.
    module: &'a ModuleIndexContext<'context>,
    /// The indexed visible tree.
    view: dir::View<'a>,
    /// Name-insensitive structural fingerprints keyed densely by node id.
    fingerprints: Vec<dir::CodeFingerprint>,
    /// Visible node ids included in the index.
    order: Vec<u32>,
    /// Adjacent executable node pairs.
    pairs: Vec<dir::CodePair>,
}

impl<'a, 'context> CodeIndexer<'a, 'context> {
    /// Build one module code index.
    pub(crate) fn build(
        module: &'a ModuleIndexContext<'context>,
    ) -> ProviderResult<dir::CodeIndex> {
        let view = module.view();
        let mut indexer = Self {
            module,
            view,
            fingerprints: vec![dir::CodeFingerprint::default(); view.next_global_id() as usize],
            order: view.iter_node_ids().map(|node| node.id).collect(),
            pairs: Vec::new(),
        };

        // fingerprint every visible subtree exactly once
        for node in view.iter_node_ids() {
            indexer.fingerprint(node)?;
        }

        // index adjacency only where children form an executable sequence
        for node in view.iter_node_ids() {
            indexer.index_pairs(node)?;
        }

        Ok(dir::CodeIndex::new(
            indexer.fingerprints,
            indexer.order,
            indexer.pairs,
        ))
    }

    /// Build one visible subtree fingerprint with iterative postorder traversal.
    fn fingerprint(&mut self, root: dir::LocalNodeIdAny) -> ProviderResult<dir::CodeFingerprint> {
        let fingerprint = self.fingerprints[root.id as usize];
        if fingerprint.node_count() != 0 {
            return Ok(fingerprint);
        }

        let mut pending = vec![(root, false)];

        // visit children before each owning node
        while let Some((node, is_ready)) = pending.pop() {
            if self.fingerprints[node.id as usize].node_count() != 0 {
                continue;
            }

            if !is_ready {
                pending.push((node, true));
                let children = self.view.direct_children(node).ok_or_else(|| {
                    ProviderError::internal(format!("indexed DIR node {node:?} is not visible"))
                })?;
                pending.extend(children.into_iter().rev().map(|child| (child, false)));

                continue;
            }

            let fingerprint = self.hash_node(node)?;
            self.fingerprints[node.id as usize] = fingerprint;
        }

        Ok(self.fingerprints[root.id as usize])
    }

    /// Hash one node after all structural children have been hashed.
    fn hash_node(&self, node: dir::LocalNodeIdAny) -> ProviderResult<dir::CodeFingerprint> {
        let children = self.view.direct_children(node).ok_or_else(|| {
            ProviderError::internal(format!("indexed DIR node {node:?} is not visible"))
        })?;
        let mut value = self.view.clone_node(node).ok_or_else(|| {
            ProviderError::internal(format!("indexed DIR node {node:?} is not visible"))
        })?;
        self.normalize(node, &mut value);
        value.map_nodes(&mut |_child| Ok::<u32, ProviderError>(0))?;

        // hash the complete authored value without arena-specific child ids
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"tspp.dir.code.v2");
        tspp_serde::hash_into(&value, &mut hasher).map_err(|error| {
            ProviderError::internal(format!("failed to hash DIR node {node:?}: {error}"))
        })?;
        hasher.write_usize(children.len());
        let mut node_count = 1_u32;

        // preserve ordered subtree structure
        for child in children {
            let child = self.fingerprints[child.id as usize];
            hasher.write_u128(child.value());
            hasher.write_u32(child.node_count());
            node_count = node_count.checked_add(child.node_count()).ok_or_else(|| {
                ProviderError::internal("indexed DIR subtree exceeds u32 node capacity")
            })?;
        }

        Ok(dir::CodeFingerprint::new(hasher.finish_u128(), node_count))
    }

    /// Erase authored names whose checked targets determine their identity.
    fn normalize(&self, node: dir::LocalNodeIdAny, value: &mut dir::NodeValue) {
        match value {
            dir::NodeValue::Expression(value) => {
                // erase resolved names and checked control labels
                match value {
                    dir::Expression::Identifier { name } => {
                        let global = node.into_global(self.module.module_id());
                        if self.has_resolved_name(global) {
                            *name = ERASED_NAME;
                        }
                    }
                    dir::Expression::While { label, .. }
                    | dir::Expression::ForEach { label, .. }
                    | dir::Expression::For { label, .. }
                    | dir::Expression::Loop { label, .. }
                    | dir::Expression::Break { label, .. }
                    | dir::Expression::Continue { label } => *label = None,
                    dir::Expression::Infer { name, .. } => *name = None,
                    _ => {}
                }
            }
            dir::NodeValue::Pattern(dir::Pattern::Binding { name, .. }) => {
                *name = ERASED_NAME;
            }
            dir::NodeValue::Parameter(value) => match value {
                dir::Parameter::Named { name, .. } | dir::Parameter::VariadicNamed { name, .. } => {
                    *name = ERASED_NAME
                }
                _ => {}
            },
            dir::NodeValue::GenericParameter(value) => match value {
                dir::GenericParameter::Type { name, .. }
                | dir::GenericParameter::VariadicType { name, .. }
                | dir::GenericParameter::Lifetime { name } => *name = ERASED_NAME,
                dir::GenericParameter::Error => {}
            },
            dir::NodeValue::TypeMappedParameter(value) => {
                value.name = ERASED_NAME;
            }
            dir::NodeValue::Declaration(value) => {
                // canonicalize declaration names paired through checked bindings
                if let Some(name) = value.name_mut().and_then(dir::Name::as_string_id_mut) {
                    *name = ERASED_NAME;
                }
            }
            dir::NodeValue::TypeExpression(value) => {
                let global = node.into_global(self.module.module_id());
                let is_resolved = self.has_resolved_name(global);

                // erase resolved type names while retaining literal lifetimes and member keys
                match value {
                    dir::TypeExpression::Lifetime { name } if is_resolved => {
                        *name = ERASED_NAME;
                    }
                    dir::TypeExpression::Reference { path, .. } if is_resolved => {
                        path.segments.clear();
                    }
                    dir::TypeExpression::Member { name, .. } if is_resolved => {
                        *name = ERASED_NAME;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Return whether one authored name resolves to a declaration or namespace.
    fn has_resolved_name(&self, node: dir::GlobalNodeIdAny) -> bool {
        if self.module.resolutions().name_resolution(node).is_some() {
            return true;
        }

        matches!(
            self.module.resolved().references.get(node),
            Some(
                dir::Reference::Bound(_)
                    | dir::Reference::Namespace { .. }
                    | dir::Reference::Projected { .. }
            )
        )
    }

    /// Index adjacent executable children of one sequence owner.
    fn index_pairs(&mut self, node: dir::LocalNodeIdAny) -> ProviderResult<()> {
        let nodes = self.executable_nodes(node);

        // combine each neighboring subtree pair in source order
        for pair in nodes.windows(2) {
            let first = self.fingerprints[pair[0].id as usize];
            let second = self.fingerprints[pair[1].id as usize];
            let fingerprint =
                dir::CodeFingerprint::concatenate([first, second]).ok_or_else(|| {
                    ProviderError::internal("indexed DIR pair exceeds u32 node capacity")
                })?;
            self.pairs.push(dir::CodePair {
                fingerprint,
                first: pair[0].id,
                second: pair[1].id,
            });
        }

        Ok(())
    }

    /// Return executable children whose adjacency forms one code region.
    fn executable_nodes(&self, node: dir::LocalNodeIdAny) -> Vec<dir::LocalNodeIdAny> {
        match node.ty {
            dir::NodeType::Block => {
                let node = dir::LocalNodeId::<dir::Block>::new(node.id);

                self.view
                    .get(node)
                    .iter_expressions()
                    .map(dir::LocalNodeId::into_any)
                    .collect()
            }
            dir::NodeType::Declaration => {
                let node = dir::LocalNodeId::<dir::Declaration>::new(node.id);
                match self.view.get(node) {
                    dir::Declaration::Global(declaration) => declaration
                        .expressions
                        .iter()
                        .copied()
                        .map(dir::LocalNodeId::into_any)
                        .collect(),
                    dir::Declaration::Module(declaration) => declaration
                        .expressions
                        .iter()
                        .copied()
                        .map(dir::LocalNodeId::into_any)
                        .collect(),
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }
}
