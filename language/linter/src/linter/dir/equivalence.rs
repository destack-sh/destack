use std::hash::Hash;

use tspp_core::FxIndexMap;
use tspp_dir as dir;
use tspp_repository::ProviderError;

use super::{Dir, DirModule};

/// One alpha-equivalence comparison between two DIR subtrees.
pub(crate) struct AlphaComparison<'a> {
    /// The left module.
    left: DirModule<'a>,
    /// The right module.
    right: DirModule<'a>,
    /// The paired declaration and reference symbols.
    symbols: Bijection<dir::GlobalSymbolId>,
    /// The paired control targets.
    controls: Bijection<dir::GlobalNodeIdAny>,
}

/// A partial one-to-one correspondence between two value sets.
struct Bijection<T> {
    /// The right value paired with each left value.
    right_by_left: FxIndexMap<T, T>,
    /// The left value paired with each right value.
    left_by_right: FxIndexMap<T, T>,
}

impl<'a> AlphaComparison<'a> {
    /// Create one comparison without known symbol pairs.
    fn empty(left: DirModule<'a>, right: DirModule<'a>) -> Self {
        Self {
            left,
            right,
            symbols: Bijection::default(),
            controls: Bijection::default(),
        }
    }

    /// Create one comparison with known symbol pairs.
    fn new(
        left: DirModule<'a>,
        right: DirModule<'a>,
        symbols: &[(dir::GlobalSymbolId, dir::GlobalSymbolId)],
    ) -> Option<Self> {
        let mut comparison = Self::empty(left, right);

        // seed the bijection with bindings supplied by the caller
        for (left, right) in symbols {
            if !comparison.pair_symbols(*left, *right) {
                return None;
            }
        }

        Some(comparison)
    }

    /// Compare two node sequences under the active correspondence.
    pub(crate) fn compare_nodes(
        &mut self,
        left: &[dir::GlobalNodeIdAny],
        right: &[dir::GlobalNodeIdAny],
    ) -> Result<bool, ProviderError> {
        if left.len() != right.len() {
            return Ok(false);
        }
        if left.iter().any(|node| node.module_id != self.left.id)
            || right.iter().any(|node| node.module_id != self.right.id)
        {
            return Err(ProviderError::internal(
                "alpha comparison received nodes from another module",
            ));
        }

        // compare corresponding roots in source order
        for (left, right) in left.iter().zip(right) {
            if !self.compare(left.local_id, right.local_id)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Compare two optional conditions under the active correspondence.
    pub(crate) fn compare_conditions(
        &mut self,
        left: Option<&dir::Condition>,
        right: Option<&dir::Condition>,
    ) -> Result<bool, ProviderError> {
        let (left, right) = match (left, right) {
            (Some(left), Some(right)) => (left, right),
            (None, None) => return Ok(true),
            _ => return Ok(false),
        };
        if left.operands.len() != right.operands.len() {
            return Ok(false);
        }

        // compare each expression or binding operand in source order
        for (left, right) in left.operands.iter().zip(&right.operands) {
            let (left, right) = match (left, right) {
                (
                    dir::ConditionOperand::Expression { condition: left },
                    dir::ConditionOperand::Expression { condition: right },
                ) => (left.into_any(), right.into_any()),
                (
                    dir::ConditionOperand::Binding {
                        kind: left_kind,
                        mutability: left_mutability,
                        declarator: left,
                    },
                    dir::ConditionOperand::Binding {
                        kind: right_kind,
                        mutability: right_mutability,
                        declarator: right,
                    },
                ) if left_kind == right_kind && left_mutability == right_mutability => {
                    (left.into_any(), right.into_any())
                }
                _ => return Ok(false),
            };
            if !self.compare(left, right)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Compare two DIR subtrees.
    fn compare(
        &mut self,
        left: dir::LocalNodeIdAny,
        right: dir::LocalNodeIdAny,
    ) -> Result<bool, ProviderError> {
        // reject different name-insensitive structure
        let left_fingerprint = self.left.code_fingerprint(left)?;
        let right_fingerprint = self.right.code_fingerprint(right)?;
        if left.ty != right.ty || left_fingerprint != right_fingerprint {
            return Ok(false);
        }

        // register declarations and control owners before comparing descendants
        if !self.pair_declaration_nodes(left, right)? || !self.pair_control_nodes(left, right) {
            return Ok(false);
        }
        if !self.runtime_meaning_matches(left, right)? {
            return Ok(false);
        }

        // compare structural children in their declared order
        let left_children =
            self.left.view().direct_children(left).ok_or_else(|| {
                ProviderError::internal(format!("left node {left:?} is not visible"))
            })?;
        let right_children = self.right.view().direct_children(right).ok_or_else(|| {
            ProviderError::internal(format!("right node {right:?} is not visible"))
        })?;
        if left_children.len() != right_children.len() {
            return Ok(false);
        }
        for (left, right) in left_children.into_iter().zip(right_children) {
            if !self.compare(left, right)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Pair symbols introduced by the two declaration nodes.
    fn pair_declaration_nodes(
        &mut self,
        left: dir::LocalNodeIdAny,
        right: dir::LocalNodeIdAny,
    ) -> Result<bool, ProviderError> {
        let left_symbol = self
            .left
            .bindings
            .declaration_symbol(left.into_global(self.left.id))
            .map(|symbol| symbol.into_global(self.left.id));
        let right_symbol = self
            .right
            .bindings
            .declaration_symbol(right.into_global(self.right.id))
            .map(|symbol| symbol.into_global(self.right.id));

        Ok(match (left_symbol, right_symbol) {
            (Some(left), Some(right)) => self.pair_symbols(left, right),
            (None, None) => true,
            _ => false,
        })
    }

    /// Pair two local symbols while preserving a bijection.
    fn pair_symbols(&mut self, left: dir::GlobalSymbolId, right: dir::GlobalSymbolId) -> bool {
        self.symbols.pair(left, right)
    }

    /// Pair two control targets before their bodies are compared.
    fn pair_control_nodes(
        &mut self,
        left: dir::LocalNodeIdAny,
        right: dir::LocalNodeIdAny,
    ) -> bool {
        if left.ty != dir::NodeType::Expression {
            return true;
        }
        let left_expression = dir::LocalNodeId::<dir::Expression>::new(left.id);
        let right_expression = dir::LocalNodeId::<dir::Expression>::new(right.id);
        let is_left_control_target = self.left.view().get(left_expression).is_control_target();
        let is_right_control_target = self.right.view().get(right_expression).is_control_target();

        // pair corresponding targets and reject asymmetric expression forms
        match (is_left_control_target, is_right_control_target) {
            (true, true) => self.controls.pair(
                left.into_global(self.left.id),
                right.into_global(self.right.id),
            ),
            (false, false) => true,
            _ => false,
        }
    }

    /// Compare resolutions that determine one node's runtime meaning.
    fn runtime_meaning_matches(
        &self,
        left: dir::LocalNodeIdAny,
        right: dir::LocalNodeIdAny,
    ) -> Result<bool, ProviderError> {
        let left_global = left.into_global(self.left.id);
        let right_global = right.into_global(self.right.id);

        // compare reduced node types whenever check assigned them
        let left_type = self.left.types.get_node_type_id(left_global);
        let right_type = self.right.types.get_node_type_id(right_global);
        if !self.optional_types_match(left_type, right_type)? {
            return Ok(false);
        }

        // compare lexical name selections under the binding bijection
        let left_resolution = self.left.resolutions.name_resolution(left_global);
        let right_resolution = self.right.resolutions.name_resolution(right_global);
        let names_match = match (left_resolution, right_resolution) {
            // type literal names compare by their denoted types
            (Some(left), Some(right))
                if left.denoted_type().is_some() || right.denoted_type().is_some() =>
            {
                self.optional_types_match(left.denoted_type(), right.denoted_type())?
            }
            (Some(left), Some(right)) => self.symbol_lists_match(left.symbols(), right.symbols()),
            (None, None) => true,
            _ => false,
        };
        if !names_match {
            return Ok(false);
        }

        // compare resolved declaration and namespace references
        let left_reference = self.left.resolved.references.get(left_global);
        let right_reference = self.right.resolved.references.get(right_global);
        if !self.references_match(left_reference, right_reference) {
            return Ok(false);
        }

        // compare control transfers by their corresponding target
        let left_transfer = self.left.decisions.transfer_decision(left_global);
        let right_transfer = self.right.decisions.transfer_decision(right_global);
        let transfers_match = match (left_transfer, right_transfer) {
            (Some(left), Some(right)) => self.controls.matches(left.into_any(), right.into_any()),
            (None, None) => true,
            _ => false,
        };
        if !transfers_match {
            return Ok(false);
        }

        // compare selected declaration-backed operations
        if left.ty == dir::NodeType::Expression {
            let left = dir::LocalNodeId::<dir::Expression>::new(left.id);
            let right = dir::LocalNodeId::<dir::Expression>::new(right.id);
            if !self.expression_decisions_match(left, right)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Compare operation selections for two expressions.
    fn expression_decisions_match(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        let left_global = left.into_global_any(self.left.id);
        let right_global = right.into_global_any(self.right.id);

        // compare call targets selected after overload resolution
        let left_call = self.left.decisions.call_decision(left_global);
        let right_call = self.right.decisions.call_decision(right_global);
        let calls_match = match (left_call, right_call) {
            (Some(left), Some(right)) => {
                self.symbol_lists_match(&left.target_symbols(), &right.target_symbols())
                    && self
                        .left
                        .dir
                        .types_match(left.return_type(), right.return_type())?
            }
            (None, None) => true,
            _ => false,
        };
        if !calls_match {
            return Ok(false);
        }

        // compare member and subscript declaration targets
        let left_member = self.left.decisions.member_decision(left_global);
        let right_member = self.right.decisions.member_decision(right_global);
        let members_match = match (left_member, right_member) {
            (Some(left), Some(right)) => {
                self.symbol_lists_match(&left.target_symbols(), &right.target_symbols())
                    && left.is_stored() == right.is_stored()
                    && self.left.dir.types_match(left.ty(), right.ty())?
            }
            (None, None) => true,
            _ => false,
        };
        if !members_match {
            return Ok(false);
        }
        let left_subscript = self.left.decisions.subscript_decision(left_global);
        let right_subscript = self.right.decisions.subscript_decision(right_global);
        let subscripts_match = match (left_subscript, right_subscript) {
            (Some(left), Some(right)) => {
                self.symbol_lists_match(&left.target_symbols(), &right.target_symbols())
                    && left.is_stored() == right.is_stored()
                    && self.left.dir.types_match(left.ty(), right.ty())?
            }
            (None, None) => true,
            _ => false,
        };
        if !subscripts_match {
            return Ok(false);
        }

        // compare the builtin domains or declaration-backed operators
        let left_operator = self.left.decisions.operator_decision(left_global);
        let right_operator = self.right.decisions.operator_decision(right_global);
        let operators_match = match (left_operator, right_operator) {
            (Some(left), Some(right)) => {
                left.is_builtin() == right.is_builtin()
                    && self.builtin_operands_match(left, right)?
                    && self.symbol_lists_match(&left.target_symbols(), &right.target_symbols())
                    && self.left.dir.types_match(left.ty(), right.ty())?
            }
            (None, None) => true,
            _ => false,
        };

        Ok(operators_match)
    }

    /// Compare two optional types across their owning modules.
    fn optional_types_match(
        &self,
        left: Option<dir::GlobalTypeId>,
        right: Option<dir::GlobalTypeId>,
    ) -> Result<bool, ProviderError> {
        match (left, right) {
            (Some(left), Some(right)) => self.left.dir.types_match(left, right),
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Compare builtin operand domains selected for two operators.
    fn builtin_operands_match(
        &self,
        left: &dir::OperatorDecision,
        right: &dir::OperatorDecision,
    ) -> Result<bool, ProviderError> {
        match (left.builtin_operands(), right.builtin_operands()) {
            (Some(left), Some(right)) => {
                if left.len() != right.len() {
                    return Ok(false);
                }
                for (left, right) in left.iter().zip(right) {
                    if left.scalar_families != right.scalar_families
                        || !self.left.dir.types_match(left.ty, right.ty)?
                    {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Compare two ordered symbol selections under the active bijection.
    fn symbol_lists_match(
        &self,
        left: &[dir::GlobalSymbolId],
        right: &[dir::GlobalSymbolId],
    ) -> bool {
        left.len() == right.len()
            && left
                .iter()
                .zip(right)
                .all(|(left, right)| self.symbols_match(*left, *right))
    }

    /// Compare two selected symbols under the active binding bijection.
    fn symbols_match(&self, left: dir::GlobalSymbolId, right: dir::GlobalSymbolId) -> bool {
        self.symbols.matches_or_equal(left, right)
    }

    /// Compare two resolved source references under the active binding bijection.
    fn references_match(
        &self,
        left: Option<&dir::Reference>,
        right: Option<&dir::Reference>,
    ) -> bool {
        match (left, right) {
            (Some(dir::Reference::Bound(left)), Some(dir::Reference::Bound(right))) => {
                self.symbol_lists_match(left, right)
            }
            (
                Some(dir::Reference::Namespace { module: left, .. }),
                Some(dir::Reference::Namespace { module: right, .. }),
            ) => left == right,
            (
                Some(dir::Reference::Projected {
                    base: left_base,
                    from: left_from,
                }),
                Some(dir::Reference::Projected {
                    base: right_base,
                    from: right_from,
                }),
            ) => left_from == right_from && self.reference_targets_match(*left_base, *right_base),
            (Some(dir::Reference::Ambiguous(left)), Some(dir::Reference::Ambiguous(right))) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| self.reference_targets_match(*left, *right))
            }
            (Some(dir::Reference::TypeLiteral(left)), Some(dir::Reference::TypeLiteral(right))) => {
                left == right
            }
            (Some(dir::Reference::Missing), Some(dir::Reference::Missing)) | (None, None) => true,
            _ => false,
        }
    }

    /// Compare two scalar source-reference targets under the active binding bijection.
    fn reference_targets_match(
        &self,
        left: dir::ReferenceTarget,
        right: dir::ReferenceTarget,
    ) -> bool {
        match (left, right) {
            (dir::ReferenceTarget::Symbol(left), dir::ReferenceTarget::Symbol(right)) => {
                self.symbols_match(left, right)
            }
            (dir::ReferenceTarget::Namespace(left), dir::ReferenceTarget::Namespace(right)) => {
                left == right
            }
            _ => false,
        }
    }
}

impl<T> Default for Bijection<T> {
    /// Create an empty correspondence.
    fn default() -> Self {
        Self {
            right_by_left: FxIndexMap::default(),
            left_by_right: FxIndexMap::default(),
        }
    }
}

impl<T: Copy + Eq + Hash> Bijection<T> {
    /// Pair two values while preserving a one-to-one correspondence.
    fn pair(&mut self, left: T, right: T) -> bool {
        if self
            .right_by_left
            .get(&left)
            .is_some_and(|paired| *paired != right)
            || self
                .left_by_right
                .get(&right)
                .is_some_and(|paired| *paired != left)
        {
            return false;
        }

        self.right_by_left.insert(left, right);
        self.left_by_right.insert(right, left);

        true
    }

    /// Return whether two values carry this correspondence.
    fn matches(&self, left: T, right: T) -> bool {
        self.right_by_left.get(&left) == Some(&right)
            && self.left_by_right.get(&right) == Some(&left)
    }

    /// Return whether two values correspond or share unmapped identity.
    fn matches_or_equal(&self, left: T, right: T) -> bool {
        match (
            self.right_by_left.get(&left),
            self.left_by_right.get(&right),
        ) {
            (Some(paired), _) => *paired == right,
            (None, Some(_)) => false,
            (None, None) => left == right,
        }
    }
}

impl Dir<'_> {
    /// Create an alpha comparison scoped by two node sequences.
    pub(crate) fn alpha_comparison(
        &self,
        left: &[dir::GlobalNodeIdAny],
        right: &[dir::GlobalNodeIdAny],
    ) -> Result<Option<AlphaComparison<'_>>, ProviderError> {
        let Some(left_module) = self.sequence_module(left)? else {
            return Ok(None);
        };
        let Some(right_module) = self.sequence_module(right)? else {
            return Ok(None);
        };
        let left = left_module.declarations_within(left)?;
        let right = right_module.declarations_within(right)?;
        if left.len() != right.len() {
            return Ok(None);
        }
        let symbols = left.into_iter().zip(right).collect::<Vec<_>>();

        Ok(AlphaComparison::new(left_module, right_module, &symbols))
    }

    /// Return whether two node sequences are alpha-equivalent.
    fn is_alpha_equivalent(
        &self,
        left: &[dir::GlobalNodeIdAny],
        right: &[dir::GlobalNodeIdAny],
    ) -> Result<bool, ProviderError> {
        if left.len() != right.len() {
            return Ok(false);
        }
        if left.is_empty() {
            return Ok(true);
        }
        let Some(mut comparison) = self.alpha_comparison(left, right)? else {
            return Ok(false);
        };

        comparison.compare_nodes(left, right)
    }

    /// Return the common alpha-equivalent prefix length of two node sequences.
    pub(crate) fn alpha_prefix_len(
        &self,
        left: &[dir::GlobalNodeIdAny],
        right: &[dir::GlobalNodeIdAny],
    ) -> Result<usize, ProviderError> {
        let Some(left_module) = self.sequence_module(left)? else {
            return Ok(0);
        };

        let Some(right_module) = self.sequence_module(right)? else {
            return Ok(0);
        };
        let mut comparison = AlphaComparison::empty(left_module, right_module);
        let mut matched = 0;

        // compare in execution order while retaining preceding declaration pairs
        for (left, right) in left.iter().zip(right) {
            if !comparison.compare(left.local_id, right.local_id)? {
                break;
            }
            matched += 1;
        }

        Ok(matched)
    }

    /// Return the common alpha-equivalent prefix length across node sequences.
    pub(crate) fn alpha_common_prefix_len(
        &self,
        sequences: &[&[dir::GlobalNodeIdAny]],
        limit: usize,
    ) -> Result<usize, ProviderError> {
        let Some((first, remaining)) = sequences.split_first() else {
            return Err(ProviderError::internal(
                "alpha prefix comparison has no node sequences",
            ));
        };
        if first.len() < limit {
            return Err(ProviderError::internal(
                "alpha prefix comparison exceeds its first node sequence",
            ));
        }

        // bound the common prefix against every remaining sequence
        let mut common = limit;
        for sequence in remaining {
            if sequence.len() < common {
                return Err(ProviderError::internal(
                    "alpha prefix comparison exceeds one node sequence",
                ));
            }
            common = self.alpha_prefix_len(&first[..common], &sequence[..common])?;
        }

        Ok(common)
    }

    /// Return the common alpha-equivalent suffix length across node sequences.
    pub(crate) fn alpha_common_suffix_len(
        &self,
        sequences: &[&[dir::GlobalNodeIdAny]],
        limit: usize,
    ) -> Result<usize, ProviderError> {
        let Some((first, remaining)) = sequences.split_first() else {
            return Err(ProviderError::internal(
                "alpha suffix comparison has no node sequences",
            ));
        };
        if sequences.iter().any(|sequence| sequence.len() < limit) {
            return Err(ProviderError::internal(
                "alpha suffix comparison exceeds one node sequence",
            ));
        }

        // select the longest suffix equivalent across every sequence
        for len in (1..=limit).rev() {
            let first = &first[first.len() - len..];
            let mut matches = true;
            for sequence in remaining {
                let sequence = &sequence[sequence.len() - len..];
                if !self.is_alpha_equivalent(first, sequence)? {
                    matches = false;
                    break;
                }
            }
            if matches {
                return Ok(len);
            }
        }

        Ok(0)
    }

    /// Return the name-insensitive structural fingerprint of one nonempty node sequence.
    pub(crate) fn code_fingerprint(
        &self,
        nodes: &[dir::GlobalNodeIdAny],
    ) -> Result<dir::CodeFingerprint, ProviderError> {
        let Some(module) = self.sequence_module(nodes)? else {
            return Err(ProviderError::internal(
                "code fingerprint requires a nonempty node sequence",
            ));
        };
        let mut combined = None;

        // concatenate each precomputed subtree fingerprint in sequence order
        for node in nodes {
            let next = module.code_fingerprint(node.local_id)?;
            let next = match combined {
                Some(current) => {
                    dir::CodeFingerprint::concatenate([current, next]).ok_or_else(|| {
                        ProviderError::internal("node sequence exceeds code fingerprint capacity")
                    })?
                }
                None => next,
            };
            combined = Some(next);
        }

        combined.ok_or_else(|| {
            ProviderError::internal("code fingerprint requires a nonempty node sequence")
        })
    }

    /// Return the common module of one node sequence.
    fn sequence_module(
        &self,
        nodes: &[dir::GlobalNodeIdAny],
    ) -> Result<Option<DirModule<'_>>, ProviderError> {
        let Some(first) = nodes.first() else {
            return Ok(None);
        };
        if nodes.iter().any(|node| node.module_id != first.module_id) {
            return Err(ProviderError::internal(
                "node sequence spans multiple modules",
            ));
        }

        self.module(first.module_id).map(Some)
    }
}

impl DirModule<'_> {
    /// Return whether two local subtrees are alpha-equivalent.
    pub(crate) fn is_alpha_equivalent(
        &self,
        left: dir::LocalNodeIdAny,
        right: dir::LocalNodeIdAny,
    ) -> Result<bool, ProviderError> {
        let left = left.into_global(self.id);
        let right = right.into_global(self.id);

        self.dir.is_alpha_equivalent(&[left], &[right])
    }

    /// Return declarations inside one node sequence in binding order.
    fn declarations_within(
        &self,
        nodes: &[dir::GlobalNodeIdAny],
    ) -> Result<Vec<dir::GlobalSymbolId>, ProviderError> {
        if nodes.iter().any(|node| node.module_id != self.id) {
            return Err(ProviderError::internal(
                "node sequence spans multiple modules",
            ));
        }
        let mut symbols = Vec::new();

        // collect declarations in structural order without scanning the module binding table
        for node in nodes {
            self.collect_declarations(node.local_id, &mut symbols)?;
        }

        Ok(symbols)
    }

    /// Collect declaration symbols from one visible subtree in structural order.
    fn collect_declarations(
        &self,
        node: dir::LocalNodeIdAny,
        symbols: &mut Vec<dir::GlobalSymbolId>,
    ) -> Result<(), ProviderError> {
        let global = node.into_global(self.id);
        if let Some(symbol) = self.bindings.declaration_symbol(global) {
            symbols.push(symbol.into_global(self.id));
        }

        // descend through every visible structural child exactly once
        let children = self
            .view()
            .direct_children(node)
            .ok_or_else(|| ProviderError::internal(format!("node {node:?} is not visible")))?;
        for child in children {
            self.collect_declarations(child, symbols)?;
        }

        Ok(())
    }
}
