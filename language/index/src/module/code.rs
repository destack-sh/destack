use std::hash::{Hash, Hasher};

use destack_core::StableHasher;
use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};

/// Builder for one module code index.
pub(crate) struct CodeIndexer<'a> {
    /// The indexed visible tree.
    view: dir::View<'a>,
    /// Candidate fingerprints keyed densely by node id.
    fingerprints: Vec<dir::CodeFingerprint>,
    /// Visible node ids included in the index.
    order: Vec<u32>,
    /// Adjacent executable node pairs.
    pairs: Vec<dir::CodePair>,
}

impl<'a> CodeIndexer<'a> {
    /// Build one module code index.
    pub(crate) fn build(view: dir::View<'a>) -> ProviderResult<dir::CodeIndex> {
        let mut indexer = Self {
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
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"destack.dir.code.v1");
        hasher.write_u8(node.ty as u8);
        self.hash_kind(node, &mut hasher);
        self.hash_scalar(node, &mut hasher);
        hasher.write_usize(children.len());
        let mut nodes = 1_u32;

        // preserve ordered subtree structure
        for child in children {
            let child = self.fingerprints[child.id as usize];
            hasher.write_u64(child.value());
            hasher.write_u32(child.node_count());
            nodes = nodes.checked_add(child.node_count()).ok_or_else(|| {
                ProviderError::internal("indexed DIR subtree exceeds u32 node capacity")
            })?;
        }

        Ok(dir::CodeFingerprint::new(hasher.finish_u64(), nodes))
    }

    /// Hash scalar values whose identity materially narrows code candidates.
    fn hash_scalar(&self, node: dir::LocalNodeIdAny, hasher: &mut StableHasher) {
        if node.ty != dir::NodeType::Expression {
            return;
        }

        let expression = dir::LocalNodeId::<dir::Expression>::new(node.id);

        // retain literal, operator, member key, and optional access identity
        match self.view.get(expression) {
            dir::Expression::Literal(value) => value.hash(hasher),
            dir::Expression::Unary { operator, .. } => {
                hasher.update_len_prefixed(operator.text().as_bytes());
                hasher.write_u8(operator.is_prefix() as u8);
            }
            dir::Expression::Binary { operator, .. } => {
                hasher.update_len_prefixed(operator.text().as_bytes())
            }
            dir::Expression::Assign { operator, .. } => {
                hasher.update_len_prefixed(operator.text().as_bytes())
            }
            dir::Expression::Member {
                name, is_optional, ..
            } => {
                name.hash(hasher);
                is_optional.hash(hasher);
            }
            dir::Expression::Index { is_optional, .. }
            | dir::Expression::Call { is_optional, .. } => is_optional.hash(hasher),
            _ => {}
        }
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

    /// Hash one visible node's concrete authored kind.
    fn hash_kind(&self, node: dir::LocalNodeIdAny, hasher: &mut StableHasher) {
        match node.ty {
            dir::NodeType::Expression => self.hash_typed_kind::<dir::Expression>(node.id, hasher),
            dir::NodeType::TypeExpression => {
                self.hash_typed_kind::<dir::TypeExpression>(node.id, hasher)
            }
            dir::NodeType::Block => self.hash_typed_kind::<dir::Block>(node.id, hasher),
            dir::NodeType::Catch => self.hash_typed_kind::<dir::Catch>(node.id, hasher),
            dir::NodeType::Declaration => self.hash_typed_kind::<dir::Declaration>(node.id, hasher),
            dir::NodeType::Declarator => self.hash_typed_kind::<dir::Declarator>(node.id, hasher),
            dir::NodeType::Property => self.hash_typed_kind::<dir::Property>(node.id, hasher),
            dir::NodeType::TypeMember => self.hash_typed_kind::<dir::TypeMember>(node.id, hasher),
            dir::NodeType::TypeMappedParameter => {
                self.hash_typed_kind::<dir::TypeMappedParameter>(node.id, hasher)
            }
            dir::NodeType::Member => self.hash_typed_kind::<dir::Member>(node.id, hasher),
            dir::NodeType::EnumField => self.hash_typed_kind::<dir::EnumField>(node.id, hasher),
            dir::NodeType::WhereClause => self.hash_typed_kind::<dir::WhereClause>(node.id, hasher),
            dir::NodeType::DependencyItem => {
                self.hash_typed_kind::<dir::DependencyItem>(node.id, hasher)
            }
            dir::NodeType::GenericParameter => {
                self.hash_typed_kind::<dir::GenericParameter>(node.id, hasher)
            }
            dir::NodeType::Parameter => self.hash_typed_kind::<dir::Parameter>(node.id, hasher),
            dir::NodeType::GenericArgument => {
                self.hash_typed_kind::<dir::GenericArgument>(node.id, hasher)
            }
            dir::NodeType::TupleElement => {
                self.hash_typed_kind::<dir::TupleElement>(node.id, hasher)
            }
            dir::NodeType::Argument => self.hash_typed_kind::<dir::Argument>(node.id, hasher),
            dir::NodeType::TreeAttribute => {
                self.hash_typed_kind::<dir::TreeAttribute>(node.id, hasher)
            }
            dir::NodeType::TreeChild => self.hash_typed_kind::<dir::TreeChild>(node.id, hasher),
            dir::NodeType::MatchArm => self.hash_typed_kind::<dir::MatchArm>(node.id, hasher),
            dir::NodeType::Pattern => self.hash_typed_kind::<dir::Pattern>(node.id, hasher),
            dir::NodeType::PatternField => {
                self.hash_typed_kind::<dir::PatternField>(node.id, hasher)
            }
            dir::NodeType::AssignPattern => {
                self.hash_typed_kind::<dir::AssignPattern>(node.id, hasher)
            }
            dir::NodeType::AssignPatternField => {
                self.hash_typed_kind::<dir::AssignPatternField>(node.id, hasher)
            }
            dir::NodeType::Decorator => self.hash_typed_kind::<dir::Decorator>(node.id, hasher),
            dir::NodeType::SwitchCase => self.hash_typed_kind::<dir::SwitchCase>(node.id, hasher),
        }
    }

    /// Hash one typed node's active Rust variant.
    fn hash_typed_kind<T>(&self, node: u32, hasher: &mut StableHasher)
    where
        T: dir::Node,
        dir::Tree: dir::TreeStore<T>,
    {
        let node = dir::LocalNodeId::<T>::new(node);
        let value = self.view.get(node);

        // retain the active variant within the exact BuildId artifact partition
        std::mem::discriminant(value).hash(hasher);
    }
}
