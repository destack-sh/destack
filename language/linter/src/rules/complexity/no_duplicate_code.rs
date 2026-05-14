use crate::LintMeta;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use destack_core::{StableHasher, StringPool};
use destack_dir as dir;
use destack_source::{FileType, ModuleId, Span};

use crate::rules::common::{
    stable_hash_bool, stable_hash_bytes, stable_hash_char, stable_hash_debug, stable_hash_f64,
    stable_hash_i64, stable_hash_none, stable_hash_token_hashed_value, stable_hash_usize,
};
use crate::{LintReport, LintRule, LintWorkspaceContext, declare_lint};

declare_lint! {
    /// Warn on duplicate and near duplicate code blocks.
    ///
    /// Repeated logic across modules makes maintenance harder and usually
    /// indicates missing abstractions.
    #[lint(
        id = "no-duplicate-code",
        code = "LX017",
        category = Complexity,
        level = Dir,
        scope = Workspace,
        requires_all = [],
        requires_any = [],
        declarations = Exclude,
        fixable = No,
        recommended = Strict,
        stability = Experimental
    )]
    pub NoDuplicateCode,
    "Warn on duplicate and near duplicate code blocks"
}

impl LintRule for NoDuplicateCode {
    fn meta(&self) -> &'static LintMeta {
        NoDuplicateCode::meta()
    }

    fn check_workspace(&self, ctx: &mut LintWorkspaceContext) {
        // lint metadata
        let meta = self.meta();
        let severity = ctx.get_severity(meta);
        if !severity.is_enabled() {
            return;
        }

        // rule options
        let rule_options = DuplicateCodeOptions::from_linter_options(ctx.options());

        // duplicate groups
        let detection = detect_duplicate_groups(ctx, &rule_options);
        if detection.occurrences.len() < 2 {
            return;
        }

        // emit exact groups first
        for group in detection.exact_groups {
            report_group_diagnostics(
                ctx,
                severity,
                &detection.occurrences,
                &group,
                DuplicateKind::Exact,
            );
        }

        // emit near groups next
        if !rule_options.near_enabled() {
            return;
        }

        for group in detection.near_groups {
            report_group_diagnostics(
                ctx,
                severity,
                &detection.occurrences,
                &group,
                DuplicateKind::Near,
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DuplicateCodeOptions {
    min_lines: usize,
    min_tokens: usize,
    near_similarity_threshold: u8,
}

impl DuplicateCodeOptions {
    /// Build rule options from linter options.
    fn from_linter_options(options: &destack_workspace::LinterOptions) -> Self {
        Self {
            min_lines: options.complexity.min_duplicate_code_lines,
            min_tokens: options.complexity.min_duplicate_code_tokens,
            near_similarity_threshold: options
                .complexity
                .min_duplicate_code_near_similarity
                .clamp(0, 100),
        }
    }

    /// Return true when near duplicate matching is enabled.
    fn near_enabled(self) -> bool {
        self.near_similarity_threshold > 0
    }
}

#[derive(Debug, Default)]
struct DuplicateDetectionResult {
    occurrences: Vec<CodeOccurrence>,
    exact_groups: Vec<Vec<usize>>,
    near_groups: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SignatureKey {
    primary_hash: u64,
    secondary_hash: u64,
    token_count: usize,
}

#[derive(Debug, Clone)]
struct CodeOccurrence {
    file_id: destack_source::FileId,
    span: Span,
    block_kind: &'static str,
    line_count: usize,
    token_count: usize,
    exact_signature: SignatureKey,
    exact_token_hashes: Vec<u64>,
    near_signature: Option<SignatureKey>,
    near_token_hashes: Option<Vec<u64>>,
    prefilter_key: BlockPrefilterKey,
}

#[derive(Debug, Clone, Copy)]
struct TokenCount {
    token_hash: u64,
    count: u32,
}

#[derive(Debug, Clone)]
struct NearSimilarityRecord {
    occurrence_index: usize,
    token_length: usize,
    bigram_length: usize,
    token_counts: Vec<TokenCount>,
    bigram_counts: Vec<TokenCount>,
    prefix_tokens: Vec<u64>,
}

#[derive(Debug, Clone)]
struct BlockCandidate {
    module_id: ModuleId,
    file_id: destack_source::FileId,
    span: Span,
    block_id: dir::LocalNodeId<dir::Block>,
    block_kind: &'static str,
    line_count: usize,
    prefilter_key: BlockPrefilterKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BlockPrefilterKey {
    top_level_expression_count: usize,
    top_level_shape_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DuplicateKind {
    Exact,
    Near,
}

impl DuplicateKind {
    /// Return diagnostic title for this duplicate kind.
    fn title(self) -> &'static str {
        match self {
            Self::Exact => "duplicate code block",
            Self::Near => "near duplicate code block",
        }
    }

    /// Return label prefix for this duplicate kind.
    fn label_prefix(self) -> &'static str {
        match self {
            Self::Exact => "this block duplicates",
            Self::Near => "this block is very similar to",
        }
    }
}

/// Detect exact and near duplicate groups for one run.
fn detect_duplicate_groups(
    ctx: &LintWorkspaceContext,
    options: &DuplicateCodeOptions,
) -> DuplicateDetectionResult {
    let include_near = options.near_enabled();
    let include_near_token_hashes = include_near;
    let occurrences = collect_occurrences(
        ctx,
        options.min_lines,
        options.min_tokens,
        include_near,
        include_near_token_hashes,
    );
    if occurrences.len() < 2 {
        return DuplicateDetectionResult::default();
    }

    let exact_groups = collect_exact_signature_groups(&occurrences);
    if exact_groups.is_empty() && !include_near {
        return DuplicateDetectionResult {
            occurrences,
            exact_groups,
            near_groups: Vec::new(),
        };
    }

    let mut exact_duplicate_indices = HashSet::new();
    for group in &exact_groups {
        for index in group {
            exact_duplicate_indices.insert(*index);
        }
    }

    let near_groups = if !include_near {
        Vec::new()
    } else if options.near_similarity_threshold >= 100 {
        collect_near_signature_groups(&occurrences)
            .into_iter()
            .map(|group| {
                group
                    .into_iter()
                    .filter(|index| !exact_duplicate_indices.contains(index))
                    .collect::<Vec<_>>()
            })
            .filter(|group| group.len() > 1)
            .collect()
    } else {
        collect_near_similarity_groups(
            &occurrences,
            &exact_duplicate_indices,
            options.near_similarity_threshold,
        )
    };

    DuplicateDetectionResult {
        occurrences,
        exact_groups,
        near_groups,
    }
}

/// Collect block occurrences eligible for duplicate detection.
fn collect_occurrences(
    ctx: &LintWorkspaceContext,
    min_lines: usize,
    min_tokens: usize,
    include_near: bool,
    include_near_token_hashes: bool,
) -> Vec<CodeOccurrence> {
    let mut candidates = Vec::new();

    for module_id in ctx.workspace_module_ids() {
        let Some(module) = ctx.repository_module(module_id) else {
            continue;
        };
        let module = module.as_ref();
        let Some(file) = ctx.repository_file(module.file_id) else {
            continue;
        };
        if !file.ty.is_code() || is_declaration_file(file.ty, ctx) {
            continue;
        }
        let Some(bound_dir) = ctx.bound_dir(module.id) else {
            continue;
        };

        let view = dir::View::new(&bound_dir.tree);
        for block_id in view.iter_nodes::<dir::Block>() {
            let span = view.get_span(block_id);
            let line_count = span_line_count(&file, span);
            if line_count < min_lines {
                continue;
            }

            candidates.push(BlockCandidate {
                module_id: module.id,
                file_id: module.file_id,
                span,
                block_id,
                block_kind: classify_block_kind(view, block_id),
                line_count,
                prefilter_key: build_block_prefilter_key(&bound_dir.tree, block_id),
            });
        }
    }

    if candidates.len() < 2 {
        return Vec::new();
    }

    // prefilter: only build full signatures for blocks sharing the same coarse shape
    let candidate_groups = collect_prefilter_groups(&candidates);
    if candidate_groups.is_empty() {
        return Vec::new();
    }

    let mut candidate_indices = candidate_groups.into_iter().flatten().collect::<Vec<_>>();
    candidate_indices.sort_unstable();
    candidate_indices.dedup();

    let mut occurrences = Vec::new();
    for candidate_index in candidate_indices {
        let candidate = &candidates[candidate_index];
        let Some(module) = ctx.repository_module(candidate.module_id) else {
            continue;
        };
        let module = module.as_ref();
        let Some(bound_dir) = ctx.bound_dir(module.id) else {
            continue;
        };
        let strings = ctx.repository.string_pool().clone();

        let signatures = build_block_signatures(
            strings.as_ref(),
            &bound_dir.tree,
            candidate.block_id,
            include_near,
            include_near_token_hashes,
        );
        if signatures.token_count < min_tokens {
            continue;
        }

        occurrences.push(CodeOccurrence {
            file_id: candidate.file_id,
            span: candidate.span,
            block_kind: candidate.block_kind,
            line_count: candidate.line_count,
            token_count: signatures.token_count,
            exact_signature: signatures.exact_signature,
            exact_token_hashes: signatures.exact_token_hashes,
            near_signature: signatures.near_signature,
            near_token_hashes: signatures.near_token_hashes,
            prefilter_key: candidate.prefilter_key,
        });
    }

    occurrences
}

/// Return grouped candidate indices for the prefilter key.
fn collect_prefilter_groups(candidates: &[BlockCandidate]) -> Vec<Vec<usize>> {
    let mut groups_by_key: HashMap<BlockPrefilterKey, Vec<usize>> = HashMap::new();

    for (index, candidate) in candidates.iter().enumerate() {
        groups_by_key
            .entry(candidate.prefilter_key)
            .or_default()
            .push(index);
    }

    let mut groups = groups_by_key
        .into_values()
        .filter(|group| group.len() > 1)
        .collect::<Vec<_>>();
    groups.sort_by_key(|group| group[0]);
    groups
}

/// Build a coarse prefilter key for a block.
fn build_block_prefilter_key(
    tree: &dir::Tree,
    block_id: dir::LocalNodeId<dir::Block>,
) -> BlockPrefilterKey {
    let block = tree.get(block_id);
    let mut hasher = StableHasher::new();
    for expression_id in block.iter_expressions() {
        let expression = tree.get(expression_id);
        std::mem::discriminant(expression).hash(&mut hasher);
    }

    BlockPrefilterKey {
        top_level_expression_count: block.len(),
        top_level_shape_hash: hasher.finish(),
    }
}

/// Return grouped occurrence indices for exact signatures.
fn collect_exact_signature_groups(occurrences: &[CodeOccurrence]) -> Vec<Vec<usize>> {
    collect_signature_groups_with_tokens(
        occurrences,
        |occurrence| Some(occurrence.exact_signature),
        |occurrence| Some(occurrence.exact_token_hashes.as_slice()),
    )
}

/// Return grouped occurrence indices for near signatures.
fn collect_near_signature_groups(occurrences: &[CodeOccurrence]) -> Vec<Vec<usize>> {
    collect_signature_groups_with_tokens(
        occurrences,
        |occurrence| occurrence.near_signature,
        |occurrence| occurrence.near_token_hashes.as_deref(),
    )
}

/// Return grouped occurrence indices for the selected signature.
fn collect_signature_groups_with_tokens(
    occurrences: &[CodeOccurrence],
    signature_selector: impl Fn(&CodeOccurrence) -> Option<SignatureKey>,
    token_selector: impl Fn(&CodeOccurrence) -> Option<&[u64]>,
) -> Vec<Vec<usize>> {
    let mut groups_by_signature: HashMap<SignatureKey, Vec<usize>> = HashMap::new();

    for (index, occurrence) in occurrences.iter().enumerate() {
        let Some(signature) = signature_selector(occurrence) else {
            continue;
        };

        groups_by_signature
            .entry(signature)
            .or_default()
            .push(index);
    }

    let mut groups = Vec::new();
    for group in groups_by_signature.into_values() {
        if group.len() < 2 {
            continue;
        }

        // verify hash groups by exact token stream to avoid hash collision false positives
        let mut groups_by_tokens = Vec::<Vec<usize>>::new();
        for index in group {
            let Some(tokens) = token_selector(&occurrences[index]) else {
                continue;
            };

            let mut matched = false;
            for token_group in &mut groups_by_tokens {
                let representative_index = token_group[0];
                let Some(representative_tokens) =
                    token_selector(&occurrences[representative_index])
                else {
                    continue;
                };
                if representative_tokens == tokens {
                    token_group.push(index);
                    matched = true;
                    break;
                }
            }

            if !matched {
                groups_by_tokens.push(vec![index]);
            }
        }

        for token_group in groups_by_tokens {
            if token_group.len() > 1 {
                groups.push(token_group);
            }
        }
    }

    groups.sort_by_key(|group| group[0]);
    groups
}

/// Return grouped occurrence indices for near similarity matching.
fn collect_near_similarity_groups(
    occurrences: &[CodeOccurrence],
    exact_duplicate_indices: &HashSet<usize>,
    threshold_percent: u8,
) -> Vec<Vec<usize>> {
    let mut groups_by_prefilter: HashMap<BlockPrefilterKey, Vec<usize>> = HashMap::new();
    for (index, occurrence) in occurrences.iter().enumerate() {
        if exact_duplicate_indices.contains(&index) {
            continue;
        }
        if occurrence.near_token_hashes.is_none() {
            continue;
        }

        groups_by_prefilter
            .entry(occurrence.prefilter_key)
            .or_default()
            .push(index);
    }

    let mut near_groups = Vec::new();
    for mut indices in groups_by_prefilter.into_values() {
        if indices.len() < 2 {
            continue;
        }
        indices.sort_unstable();

        let mut records = Vec::new();
        for index in indices {
            let Some(token_hashes) = occurrences[index].near_token_hashes.as_deref() else {
                continue;
            };

            records.push(NearSimilarityRecord {
                occurrence_index: index,
                token_length: token_hashes.len(),
                bigram_length: token_hashes.len().saturating_sub(1),
                token_counts: token_frequency_table(token_hashes),
                bigram_counts: token_bigram_frequency_table(token_hashes),
                prefix_tokens: Vec::new(),
            });
        }
        if records.len() < 2 {
            continue;
        }

        // prefix candidate generation: compare only pairs sharing at least one prefix token
        let min_token_length = records
            .iter()
            .map(|record| record.token_length)
            .min()
            .unwrap_or(0);
        for record in &mut records {
            let required_shared =
                required_shared_tokens(record.token_length, min_token_length, threshold_percent)
                    as usize;
            let prefix_length = record
                .token_length
                .saturating_sub(required_shared)
                .saturating_add(1);
            let token_hashes = occurrences[record.occurrence_index]
                .near_token_hashes
                .as_deref()
                .unwrap_or(&[]);
            record.prefix_tokens = prefix_token_set(token_hashes, prefix_length);
        }

        let mut disjoint = DisjointSet::new(records.len());
        let mut postings = HashMap::<u64, Vec<usize>>::new();
        for right_position in 0..records.len() {
            let right = &records[right_position];

            let mut candidate_positions = Vec::new();
            for token_hash in &right.prefix_tokens {
                if let Some(positions) = postings.get(token_hash) {
                    candidate_positions.extend(positions.iter().copied());
                }
            }
            candidate_positions.sort_unstable();
            candidate_positions.dedup();

            for left_position in candidate_positions {
                let left = &records[left_position];

                if !can_reach_similarity_threshold(
                    left.token_length,
                    right.token_length,
                    threshold_percent,
                ) {
                    continue;
                }

                let has_sufficient_shared_tokens = meets_shared_token_threshold(
                    &left.token_counts,
                    &right.token_counts,
                    left.token_length,
                    right.token_length,
                    threshold_percent,
                );
                if !has_sufficient_shared_tokens {
                    continue;
                }

                if left.bigram_length == 0 || right.bigram_length == 0 {
                    disjoint.union(left_position, right_position);
                    continue;
                }

                let bigram_threshold = threshold_percent.saturating_sub(10);
                if !can_reach_similarity_threshold(
                    left.bigram_length,
                    right.bigram_length,
                    bigram_threshold,
                ) {
                    continue;
                }

                let has_sufficient_shared_bigrams = meets_shared_token_threshold(
                    &left.bigram_counts,
                    &right.bigram_counts,
                    left.bigram_length,
                    right.bigram_length,
                    bigram_threshold,
                );
                if has_sufficient_shared_bigrams {
                    disjoint.union(left_position, right_position);
                }
            }

            for token_hash in &right.prefix_tokens {
                postings
                    .entry(*token_hash)
                    .or_default()
                    .push(right_position);
            }
        }

        let mut component_map: HashMap<usize, Vec<usize>> = HashMap::new();
        for (position, record) in records.iter().enumerate() {
            let root = disjoint.find(position);
            component_map
                .entry(root)
                .or_default()
                .push(record.occurrence_index);
        }

        for mut group in component_map.into_values() {
            if group.len() < 2 {
                continue;
            }
            group.sort_unstable();
            near_groups.push(group);
        }
    }

    near_groups.sort_by_key(|group| group[0]);
    near_groups
}

/// Build a unique sorted prefix token set.
fn prefix_token_set(token_hashes: &[u64], prefix_length: usize) -> Vec<u64> {
    if token_hashes.is_empty() || prefix_length == 0 {
        return Vec::new();
    }

    let mut sorted_tokens = token_hashes.to_vec();
    sorted_tokens.sort_unstable();

    let mut prefix_tokens = Vec::new();
    for token_hash in sorted_tokens.into_iter().take(prefix_length) {
        if prefix_tokens.last().is_some_and(|last| *last == token_hash) {
            continue;
        }
        prefix_tokens.push(token_hash);
    }

    prefix_tokens
}

/// Return true when two token sequences can satisfy the threshold.
fn can_reach_similarity_threshold(
    len_left: usize,
    len_right: usize,
    threshold_percent: u8,
) -> bool {
    let max_shared = len_left.min(len_right) as u128;
    max_shared >= required_shared_tokens(len_left, len_right, threshold_percent)
}

/// Return true when shared token frequency satisfies the threshold.
fn meets_shared_token_threshold(
    left_counts: &[TokenCount],
    right_counts: &[TokenCount],
    len_left: usize,
    len_right: usize,
    threshold_percent: u8,
) -> bool {
    let required_shared = required_shared_tokens(len_left, len_right, threshold_percent);
    if required_shared == 0 {
        return true;
    }

    let mut left_remaining = left_counts
        .iter()
        .map(|count| count.count as usize)
        .sum::<usize>();
    let mut right_remaining = right_counts
        .iter()
        .map(|count| count.count as usize)
        .sum::<usize>();
    let mut shared = 0usize;
    let mut left_index = 0usize;
    let mut right_index = 0usize;

    while left_index < left_counts.len() && right_index < right_counts.len() {
        let left = left_counts[left_index];
        let right = right_counts[right_index];

        if left.token_hash < right.token_hash {
            left_remaining = left_remaining.saturating_sub(left.count as usize);
            left_index += 1;
        } else if left.token_hash > right.token_hash {
            right_remaining = right_remaining.saturating_sub(right.count as usize);
            right_index += 1;
        } else {
            let matched = left.count.min(right.count) as usize;
            shared += matched;
            if shared as u128 >= required_shared {
                return true;
            }

            left_remaining = left_remaining.saturating_sub(left.count as usize);
            right_remaining = right_remaining.saturating_sub(right.count as usize);
            left_index += 1;
            right_index += 1;
        }

        let max_additional = left_remaining.min(right_remaining) as u128;
        if shared as u128 + max_additional < required_shared {
            return false;
        }
    }

    shared as u128 >= required_shared
}

/// Return minimum shared token count required to satisfy the threshold.
fn required_shared_tokens(len_left: usize, len_right: usize, threshold_percent: u8) -> u128 {
    let numerator = threshold_percent as u128 * (len_left + len_right) as u128;
    numerator.div_ceil(200)
}

/// Build a token frequency table from token hashes.
fn token_frequency_table(token_hashes: &[u64]) -> Vec<TokenCount> {
    token_frequency_table_from_vec(token_hashes.to_vec())
}

/// Build a token frequency table from an owned token vector.
fn token_frequency_table_from_vec(mut token_hashes: Vec<u64>) -> Vec<TokenCount> {
    if token_hashes.is_empty() {
        return Vec::new();
    }

    token_hashes.sort_unstable();
    let mut counts = Vec::new();

    let mut current_hash = token_hashes[0];
    let mut current_count = 1u32;
    for token_hash in token_hashes.into_iter().skip(1) {
        if token_hash == current_hash {
            current_count += 1;
            continue;
        }

        counts.push(TokenCount {
            token_hash: current_hash,
            count: current_count,
        });
        current_hash = token_hash;
        current_count = 1;
    }

    counts.push(TokenCount {
        token_hash: current_hash,
        count: current_count,
    });

    counts
}

/// Build a token bigram frequency table.
fn token_bigram_frequency_table(token_hashes: &[u64]) -> Vec<TokenCount> {
    if token_hashes.len() < 2 {
        return Vec::new();
    }

    let mut bigram_hashes = Vec::with_capacity(token_hashes.len().saturating_sub(1));
    for window in token_hashes.windows(2) {
        bigram_hashes.push(hash_bigram(window[0], window[1]));
    }

    token_frequency_table_from_vec(bigram_hashes)
}

/// Hash one token bigram.
fn hash_bigram(first: u64, second: u64) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    hash ^= first;
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    hash ^= second;
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    hash
}

#[derive(Debug)]
struct DisjointSet {
    parents: Vec<usize>,
}

impl DisjointSet {
    /// Create a disjoint set with one component per index.
    fn new(size: usize) -> Self {
        let mut parents = Vec::with_capacity(size);
        for index in 0..size {
            parents.push(index);
        }

        Self { parents }
    }

    /// Find the root component for an index.
    fn find(&mut self, index: usize) -> usize {
        let parent = self.parents[index];
        if parent == index {
            return index;
        }

        let root = self.find(parent);
        self.parents[index] = root;
        root
    }

    /// Merge two components.
    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root != right_root {
            self.parents[right_root] = left_root;
        }
    }
}

/// Emit diagnostics for one duplicate group.
fn report_group_diagnostics(
    ctx: &mut LintWorkspaceContext,
    severity: destack_workspace::LintSeverity,
    occurrences: &[CodeOccurrence],
    group: &[usize],
    duplicate_kind: DuplicateKind,
) {
    if group.len() < 2 {
        return;
    }

    for (position, index) in group.iter().enumerate() {
        let occurrence = &occurrences[*index];
        let reference_index = if position == 0 { group[1] } else { group[0] };
        let reference = &occurrences[reference_index];

        let mut label = String::from(duplicate_kind.label_prefix());
        label.push(' ');
        if occurrence.file_id == reference.file_id {
            label.push_str("another block in this file");
        } else {
            let Some(reference_file) = ctx.repository_file(reference.file_id) else {
                continue;
            };
            label.push_str("a ");
            label.push_str(reference.block_kind);
            label.push_str(" in ");
            label.push_str(&reference_file.name);
        }

        let diagnostic = LintReport::new(
            NO_DUPLICATE_CODE.id,
            NO_DUPLICATE_CODE.code,
            NO_DUPLICATE_CODE.category,
            severity,
            duplicate_kind.title(),
            occurrence.span,
        )
        .label(label)
        .note(format!(
            "{}: {} lines, {} tokens",
            occurrence.block_kind, occurrence.line_count, occurrence.token_count
        ));
        ctx.report(diagnostic);
    }
}

/// Return coarse description for the given block.
fn classify_block_kind(
    view: dir::View<'_>,
    block_id: dir::LocalNodeId<dir::Block>,
) -> &'static str {
    let Some(parent_id) = view.get_parent_id(block_id.id) else {
        return "code block";
    };

    let parent_type = view.get_node_type(parent_id);
    if parent_type == dir::NodeType::MatchCase {
        return "match arm";
    }
    if parent_type != dir::NodeType::Expression {
        return "code block";
    }

    let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
    let parent_expression = view.get(parent_expression_id);
    match parent_expression {
        dir::Expression::While { .. }
        | dir::Expression::ForEach { .. }
        | dir::Expression::For { .. }
        | dir::Expression::Loop { .. } => "loop body",
        dir::Expression::Block(inner_block_id) => {
            if *inner_block_id != block_id {
                return "code block";
            }
            classify_block_expression_owner(view, parent_expression_id)
        }
        _ => "code block",
    }
}

/// Return coarse description for a block expression owner.
fn classify_block_expression_owner(
    view: dir::View<'_>,
    block_expression_id: dir::LocalNodeId<dir::Expression>,
) -> &'static str {
    let Some(owner_id) = view.get_parent_id(block_expression_id.id) else {
        return "code block";
    };

    match view.get_node_type(owner_id) {
        dir::NodeType::Declaration => {
            let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(owner_id);
            let declaration = view.get(declaration_id);
            match declaration {
                dir::Declaration::Function(declaration)
                    if declaration.body == Some(block_expression_id) =>
                {
                    "function body"
                }
                _ => "code block",
            }
        }
        dir::NodeType::Member => {
            let member_id = dir::LocalNodeId::<dir::Member>::new(owner_id);
            let member = view.get(member_id);
            match member {
                dir::Member::Method { body, .. } if body == &Some(block_expression_id) => {
                    "method body"
                }
                dir::Member::StaticBlock { body, .. } if *body == block_expression_id => {
                    "static block"
                }
                dir::Member::ComptimeBlock { body, .. } if *body == block_expression_id => {
                    "comptime block"
                }
                _ => "code block",
            }
        }
        dir::NodeType::Expression => {
            let owner_expression_id = dir::LocalNodeId::<dir::Expression>::new(owner_id);
            let owner_expression = view.get(owner_expression_id);
            match owner_expression {
                dir::Expression::If {
                    then_expression,
                    else_expression,
                    ..
                } if *then_expression == block_expression_id
                    || else_expression == &Some(block_expression_id) =>
                {
                    "branch block"
                }
                dir::Expression::Try {
                    try_expression,
                    catch_expression,
                    finally_expression,
                    ..
                } if *try_expression == block_expression_id
                    || catch_expression == &Some(block_expression_id)
                    || finally_expression == &Some(block_expression_id) =>
                {
                    "try block"
                }
                dir::Expression::Match { .. } => "match arm",
                _ => "code block",
            }
        }
        _ => "code block",
    }
}

/// Return true when declaration files should be skipped.
fn is_declaration_file(file_type: FileType, ctx: &LintWorkspaceContext) -> bool {
    if ctx.options().include_declaration_files {
        return false;
    }

    matches!(
        file_type,
        FileType::TypeScriptDeclaration | FileType::DestackDeclaration
    )
}

/// Return line count for a span.
fn span_line_count(file: &destack_source::File, span: Span) -> usize {
    if span.end <= span.start {
        return 0;
    }

    let Some((start_line, _)) = file.get_position(span.start) else {
        return 0;
    };
    let end_index = span.end.saturating_sub(1);
    let Some((end_line, _)) = file.get_position(end_index) else {
        return 0;
    };

    end_line.saturating_sub(start_line) as usize + 1
}

#[derive(Debug)]
struct BlockSignatures {
    exact_signature: SignatureKey,
    exact_token_hashes: Vec<u64>,
    near_signature: Option<SignatureKey>,
    near_token_hashes: Option<Vec<u64>>,
    token_count: usize,
}

/// Build exact and near signatures for a block.
fn build_block_signatures(
    strings: &StringPool,
    tree: &dir::Tree,
    block_id: dir::LocalNodeId<dir::Block>,
    include_near: bool,
    include_near_token_hashes: bool,
) -> BlockSignatures {
    let block = tree.get(block_id);
    let mut collector =
        DuplicateSignatureCollector::new(strings, include_near, include_near_token_hashes);
    dir::walk_block(&mut collector, tree, block_id, block);
    collector.finish()
}

#[derive(Debug)]
struct DuplicateSignatureCollector<'a> {
    strings: &'a StringPool,
    visitor_options: dir::NodeVisitorOptions,
    exact_signature: SignatureHasher,
    exact_token_hashes: Vec<u64>,
    near_signature: Option<SignatureHasher>,
    near_token_hashes: Option<Vec<u64>>,
    token_count: usize,
}

impl<'a> DuplicateSignatureCollector<'a> {
    /// Create a new signature collector.
    fn new(strings: &'a StringPool, include_near: bool, include_near_token_hashes: bool) -> Self {
        Self {
            strings,
            visitor_options: dir::NodeVisitorOptions::default(),
            exact_signature: SignatureHasher::new(),
            exact_token_hashes: Vec::new(),
            near_signature: include_near.then(SignatureHasher::new),
            near_token_hashes: include_near_token_hashes.then(Vec::new),
            token_count: 0,
        }
    }

    /// Finish and return collected signatures.
    fn finish(self) -> BlockSignatures {
        BlockSignatures {
            exact_signature: self.exact_signature.finish(self.token_count),
            exact_token_hashes: self.exact_token_hashes,
            near_signature: self
                .near_signature
                .map(|signature| signature.finish(self.token_count)),
            near_token_hashes: self.near_token_hashes,
            token_count: self.token_count,
        }
    }

    /// Push one token to exact and near signatures.
    fn push_token(
        &mut self,
        key: &'static str,
        exact_value: &'static str,
        near_value: &'static str,
    ) {
        let exact_value_hash = stable_hash_bytes(exact_value.as_bytes());
        let near_value_hash = stable_hash_bytes(near_value.as_bytes());
        self.push_hashed_token(key, exact_value_hash, near_value_hash);
    }

    /// Push one prehashed token to exact and near signatures.
    fn push_hashed_token(
        &mut self,
        key: &'static str,
        exact_value_hash: u64,
        near_value_hash: u64,
    ) {
        let exact_token_hash = hash_token_hashed_value(key, exact_value_hash);
        self.exact_signature.push(exact_token_hash);
        self.exact_token_hashes.push(exact_token_hash);

        let near_token_hash = hash_token_hashed_value(key, near_value_hash);
        if let Some(signature) = self.near_signature.as_mut() {
            signature.push(near_token_hash);
        }
        if let Some(token_hashes) = self.near_token_hashes.as_mut() {
            token_hashes.push(near_token_hash);
        }

        self.token_count += 1;
    }

    /// Push the same token to exact and near signatures.
    fn push_same(&mut self, key: &'static str, value: &'static str) {
        self.push_token(key, value, value);
    }

    /// Push one boolean token.
    fn push_bool(&mut self, key: &'static str, value: bool) {
        self.push_hashed_token(key, stable_hash_bool(value), stable_hash_bool(value));
    }

    /// Push one length token.
    fn push_length(&mut self, key: &'static str, value: usize) {
        let value_hash = stable_hash_usize(value);
        self.push_hashed_token(key, value_hash, value_hash);
    }

    /// Push a prehashed literal token.
    fn push_literal_hashed(
        &mut self,
        key: &'static str,
        exact_hash: u64,
        placeholder: &'static str,
    ) {
        let near_hash = stable_hash_bytes(placeholder.as_bytes());
        self.push_hashed_token(key, exact_hash, near_hash);
    }

    /// Push debug value as a shared token.
    fn push_debug<T: std::fmt::Debug>(&mut self, key: &'static str, value: T) {
        let value_hash = stable_hash_debug(&value);
        self.push_hashed_token(key, value_hash, value_hash);
    }

    /// Push optional debug value as a shared token.
    fn push_debug_optional<T: std::fmt::Debug>(&mut self, key: &'static str, value: Option<T>) {
        if let Some(value) = value {
            self.push_debug(key, value);
        } else {
            self.push_hashed_token(key, stable_hash_none(), stable_hash_none());
        }
    }

    /// Push identifier string id.
    fn push_identifier_id(&mut self, key: &'static str, id: dir::StringId) {
        let text = self.strings.get(id);
        let exact_hash = stable_hash_bytes(text.as_bytes());
        let near_hash = stable_hash_bytes(b"$id");
        self.push_hashed_token(key, exact_hash, near_hash);
    }

    /// Push literal string id.
    fn push_literal_id(&mut self, key: &'static str, id: dir::StringId, placeholder: &'static str) {
        let text = self.strings.get(id);
        let exact_hash = stable_hash_bytes(text.as_bytes());
        let near_hash = stable_hash_bytes(placeholder.as_bytes());
        self.push_hashed_token(key, exact_hash, near_hash);
    }

    /// Push a name token.
    fn push_name(&mut self, key: &'static str, name: dir::Name) {
        match name {
            dir::Name::Identifier(id) => self.push_identifier_id(key, id),
            dir::Name::String(id) => self.push_literal_id(key, id, "$str"),
            dir::Name::Number(id) => self.push_literal_id(key, id, "$num"),
        }
    }

    /// Push declaration name and export metadata.
    fn push_declaration_name(&mut self, name: Option<dir::Name>) {
        if let Some(name) = name {
            self.push_name("declaration_name", name);
        } else {
            self.push_same("declaration_name", "None");
        }
    }

    /// Push key metadata.
    fn push_key(&mut self, key: dir::Key) {
        match key {
            dir::Key::Name(name) => {
                self.push_same("key_kind", "name");
                self.push_name("key_name", name);
            }
            dir::Key::Private(name) => {
                self.push_same("key_kind", "private");
                self.push_identifier_id("key_name", name);
            }
            dir::Key::Expression(_) => {
                self.push_same("key_kind", "expr");
            }
        }
    }

    /// Push function signature metadata.
    fn push_function_signature(&mut self, signature: &dir::FunctionSignature) {
        self.push_debug("function_is_abstract", signature.is_abstract);
        self.push_debug("function_is_override", signature.is_override);
        self.push_debug("function_asynchrony", signature.asynchrony);
        self.push_debug("function_is_generator", signature.is_generator);
        self.push_debug_optional("function_mode", signature.role);
        self.push_debug("function_kind", signature.form);
    }

    /// Push scalar literal metadata.
    fn push_scalar_literal(&mut self, literal: &dir::ScalarLiteral) {
        self.push_debug("scalar_kind", std::mem::discriminant(literal));
        match literal {
            dir::ScalarLiteral::Null => {
                self.push_same("scalar_value", "null");
            }
            dir::ScalarLiteral::Boolean(value) => {
                self.push_literal_hashed("scalar_value", stable_hash_bool(*value), "$bool");
            }
            dir::ScalarLiteral::Integer(value) => {
                self.push_literal_hashed("scalar_value", stable_hash_i64(*value), "$num");
            }
            dir::ScalarLiteral::Bigint(value) => {
                self.push_literal_hashed("scalar_value", stable_hash_i64(*value), "$num");
            }
            dir::ScalarLiteral::Float(value) => {
                self.push_literal_hashed("scalar_value", stable_hash_f64(*value), "$num");
            }
            dir::ScalarLiteral::Character(value) => {
                self.push_literal_hashed("scalar_value", stable_hash_char(*value), "$char");
            }
            dir::ScalarLiteral::String(value) => {
                self.push_literal_id("scalar_value", *value, "$str");
            }
            dir::ScalarLiteral::RegexString { content, flags } => {
                self.push_literal_id("scalar_regex_content", *content, "$regex");
                if let Some(flags) = flags {
                    self.push_literal_id("scalar_regex_flags", *flags, "$flags");
                } else {
                    self.push_same("scalar_regex_flags", "None");
                }
            }
        }
    }

    /// Push template literal metadata.
    fn push_template_literal(&mut self, value: &dir::TemplateLiteral) {
        self.push_debug("template_kind", std::mem::discriminant(value));
        match value {
            dir::TemplateLiteral::String { string } => {
                self.push_literal_id("template_string", *string, "$str");
            }
            dir::TemplateLiteral::InterpolatedString { strings, .. } => {
                self.push_length("template_parts", strings.len());
                for string in strings {
                    self.push_literal_id("template_part", *string, "$str");
                }
            }
        }
    }

    /// Push path metadata.
    fn push_path(&mut self, path: &dir::Path) {
        self.push_length("path_len", path.segments.len());
        for segment in &path.segments {
            self.push_identifier_id("path_segment", *segment);
        }
    }
}

impl dir::NodeVisitor for DuplicateSignatureCollector<'_> {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.visitor_options
    }

    fn visit_any(&mut self, _tree: &dir::Tree, ty: dir::NodeType, _id: u32) {
        self.push_debug("node_type", ty);
    }

    fn visit_block(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Block>,
        block: &dir::Block,
    ) {
        self.push_same("visit", "block");
        dir::walk_block(self, tree, id, block);
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        self.push_debug("expr_kind", std::mem::discriminant(expression));
        match expression {
            dir::Expression::Label { label, .. } => {
                self.push_identifier_id("expr_label", *label);
            }
            dir::Expression::Import { space, target, .. } => {
                self.push_debug("expr_import_space", *space);
                self.push_literal_id("expr_import_target", *target, "$str");
            }
            dir::Expression::Export { space, target, .. } => {
                self.push_debug("expr_export_space", *space);
                if let Some(target) = target {
                    self.push_literal_id("expr_export_target", *target, "$str");
                } else {
                    self.push_same("expr_export_target", "None");
                }
            }
            dir::Expression::Let {
                kind, mutability, ..
            } => {
                self.push_debug("expr_let_kind", *kind);
                self.push_debug("expr_let_mutability", *mutability);
            }
            dir::Expression::Using { asynchrony, .. } => {
                self.push_debug("expr_using_asynchrony", *asynchrony);
            }
            dir::Expression::If {
                form, condition, ..
            } => {
                self.push_debug("expr_if_form", *form);
                match condition {
                    dir::IfCondition::Expression { .. } => {
                        self.push_same("expr_if_condition_kind", "expr");
                    }
                    dir::IfCondition::Let {
                        kind, mutability, ..
                    } => {
                        self.push_same("expr_if_condition_kind", "let");
                        self.push_debug("expr_if_let_kind", *kind);
                        self.push_debug("expr_if_let_mutability", *mutability);
                    }
                }
            }
            dir::Expression::While { form, .. } => {
                self.push_debug("expr_while_form", *form);
            }
            dir::Expression::ForEach {
                asynchrony,
                operator,
                binding,
                ..
            } => {
                self.push_debug("expr_foreach_asynchrony", *asynchrony);
                self.push_debug("expr_foreach_operator", *operator);
                match binding {
                    dir::ForEachBinding::Pattern { keyword, .. } => {
                        self.push_same("expr_foreach_binding", "pattern");
                        self.push_debug_optional("expr_foreach_keyword", *keyword);
                    }
                    dir::ForEachBinding::Using { asynchrony, .. } => {
                        self.push_same("expr_foreach_binding", "using");
                        self.push_debug("expr_foreach_using_asynchrony", *asynchrony);
                    }
                }
            }
            dir::Expression::Match { form, .. } => {
                self.push_debug("expr_match_form", *form);
            }
            dir::Expression::Break { label, value } => {
                if let Some(label) = label {
                    self.push_identifier_id("expr_break_label", *label);
                } else {
                    self.push_same("expr_break_label", "None");
                }
                self.push_bool("expr_break_has_value", value.is_some());
            }
            dir::Expression::Continue { label } => {
                if let Some(label) = label {
                    self.push_identifier_id("expr_continue_label", *label);
                } else {
                    self.push_same("expr_continue_label", "None");
                }
            }
            dir::Expression::Yield { cardinality, .. } => {
                self.push_debug("expr_yield_cardinality", *cardinality);
            }
            dir::Expression::Identifier { name } => {
                self.push_identifier_id("expr_identifier", *name);
            }
            dir::Expression::QualifiedReference {
                path,
                generic_arguments,
            } => {
                self.push_path(path);
                self.push_bool("expr_path_generic_arguments", !generic_arguments.is_empty());
            }
            dir::Expression::PrivateIdentifier { name } => {
                self.push_identifier_id("expr_private_identifier", *name);
            }
            dir::Expression::ScalarLiteral(literal) => {
                self.push_scalar_literal(literal);
            }
            dir::Expression::Type { .. } => {
                self.push_same("expr_type", "type");
            }
            dir::Expression::TemplateExpression { value } => {
                self.push_template_literal(value);
            }
            dir::Expression::TaggedTemplateExpression { value, .. } => {
                self.push_template_literal(value);
            }
            dir::Expression::Unary { operator, .. } => {
                self.push_debug("expr_unary_operator", *operator);
            }
            dir::Expression::MoveOf {
                mutability,
                variance,
                ..
            } => {
                self.push_debug_optional("expr_value_of_mutability", *mutability);
                self.push_debug_optional("expr_value_of_variance", *variance);
            }
            dir::Expression::BorrowOf {
                mutability,
                variance,
                ..
            } => {
                self.push_debug_optional("expr_reference_of_mutability", *mutability);
                self.push_debug_optional("expr_reference_of_variance", *variance);
            }
            dir::Expression::Member { name, .. } => {
                if let Some(name) = *name {
                    self.push_identifier_id("expr_member_name", name);
                } else {
                    self.push_same("expr_member_name", "None");
                }
            }
            dir::Expression::PrivateMember { name, .. } => {
                if let Some(name) = *name {
                    self.push_identifier_id("expr_private_member_name", name);
                } else {
                    self.push_same("expr_private_member_name", "None");
                }
            }
            dir::Expression::Index { position, .. } => {
                self.push_debug("expr_index_position", *position);
            }
            dir::Expression::Call { position, .. } => {
                self.push_debug("expr_call_position", *position);
            }
            dir::Expression::Maybe { position, .. } => {
                self.push_debug("expr_maybe_position", *position);
            }
            dir::Expression::Must { position, .. } => {
                self.push_debug("expr_must_position", *position);
            }
            dir::Expression::Binary { operator, .. } => {
                self.push_debug("expr_binary_operator", *operator);
            }
            dir::Expression::Assign { operator, .. } => {
                self.push_debug("expr_assign_operator", *operator);
            }
            _ => {}
        }

        dir::walk_expression(self, tree, id, expression);
    }

    fn visit_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        self.push_debug("decl_kind", std::mem::discriminant(declaration));
        self.push_declaration_name(declaration.name());
        match declaration {
            dir::Declaration::Type(declaration) => {
                self.push_debug("decl_type_is_nominal", declaration.is_nominal);
                self.push_debug_optional("decl_type_mutability", declaration.mutability);
            }
            dir::Declaration::Function(declaration) => {
                self.push_function_signature(&declaration.signature);
            }
            _ => {}
        }

        dir::walk_declaration(self, tree, id, declaration);
    }

    fn visit_property(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        self.push_debug("property_kind", std::mem::discriminant(property));
        match property {
            dir::Property::Field { key, .. } => {
                self.push_key(*key);
            }
            dir::Property::Method { key, signature, .. } => {
                if let Some(key) = key {
                    self.push_key(*key);
                } else {
                    self.push_same("property_key", "None");
                }
                self.push_function_signature(signature);
            }
            dir::Property::Spread { .. } => {}
            dir::Property::Error => {}
        }

        dir::walk_property(self, tree, id, property);
    }

    fn visit_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) {
        self.push_debug("member_kind", std::mem::discriminant(member));
        match member {
            dir::Member::AssociatedType { name, .. } => {
                self.push_identifier_id("member_name", *name);
            }
            dir::Member::AssociatedConst { name, .. } => {
                self.push_identifier_id("member_name", *name);
            }
            dir::Member::Field { key, .. } => {
                self.push_key(*key);
            }
            dir::Member::Method { key, signature, .. } => {
                if let Some(key) = key {
                    self.push_key(*key);
                } else {
                    self.push_same("member_key", "None");
                }
                self.push_function_signature(signature);
            }
            dir::Member::StaticBlock { .. } | dir::Member::ComptimeBlock { .. } => {}
            dir::Member::Error => {}
        }

        dir::walk_member(self, tree, id, member);
    }

    fn visit_where_clause(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::WhereClause>,
        where_clause: &dir::WhereClause,
    ) {
        self.push_identifier_id("where_left", where_clause.left);
        dir::walk_where_clause(self, tree, id, where_clause);
    }

    fn visit_dependency_item(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::DependencyItem>,
        dependency_item: &dir::DependencyItem,
    ) {
        match dependency_item {
            dir::DependencyItem::Item {
                binding,
                space,
                name,
                alias,
                ..
            } => {
                self.push_debug("dependency_binding", *binding);
                self.push_debug_optional("dependency_space", *space);
                if let Some(name) = *name {
                    self.push_name("dependency_name", name);
                } else {
                    self.push_same("dependency_name", "None");
                }
                if let Some(alias) = *alias {
                    self.push_identifier_id("dependency_alias", alias);
                } else {
                    self.push_same("dependency_alias", "None");
                }
            }
            dir::DependencyItem::Error => {
                self.push_same("dependency_mode", "Error");
                self.push_same("dependency_kind", "Error");
                self.push_same("dependency_name", "Error");
                self.push_same("dependency_alias", "Error");
            }
        }

        dir::walk_dependency_item(self, tree, id, dependency_item);
    }

    fn visit_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        self.push_debug("parameter_kind", std::mem::discriminant(parameter));
        match parameter {
            dir::Parameter::Named {
                name,
                visibility,
                is_readonly,
                is_optional,
                ..
            } => {
                self.push_identifier_id("parameter_name", *name);
                self.push_debug_optional("parameter_visibility", *visibility);
                self.push_bool("parameter_is_readonly", *is_readonly);
                self.push_bool("parameter_is_optional", *is_optional);
            }
            dir::Parameter::VariadicNamed {
                name,
                visibility,
                is_readonly,
                ..
            } => {
                self.push_identifier_id("parameter_name", *name);
                self.push_debug_optional("parameter_visibility", *visibility);
                self.push_bool("parameter_is_readonly", *is_readonly);
                self.push_bool("parameter_is_optional", false);
            }
            dir::Parameter::Pattern { is_optional, .. } => {
                self.push_bool("parameter_is_optional", *is_optional);
            }
            dir::Parameter::VariadicPattern { .. } => {}
            dir::Parameter::Error => {}
        }

        dir::walk_parameter(self, tree, id, parameter);
    }

    fn visit_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Argument>,
        argument: &dir::Argument,
    ) {
        self.push_debug("argument_kind", std::mem::discriminant(argument));
        match argument {
            dir::Argument::Named { name, .. } => {
                self.push_name("argument_name", *name);
            }
            dir::Argument::Labeled { label, .. } => {
                self.push_identifier_id("argument_label", *label);
            }
            dir::Argument::Positional { .. } => {}
            dir::Argument::Spread { label, .. } => {
                if let Some(label) = label {
                    self.push_identifier_id("argument_label", *label);
                } else {
                    self.push_same("argument_label", "None");
                }
            }
            dir::Argument::Error => {}
        }

        dir::walk_argument(self, tree, id, argument);
    }

    fn visit_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        self.push_debug("pattern_kind", std::mem::discriminant(pattern));
        match pattern {
            dir::Pattern::Binding { name, .. } => {
                self.push_identifier_id("pattern_binding_name", *name);
            }
            dir::Pattern::TaggedTuple { .. } | dir::Pattern::TaggedObject { .. } => {
                self.push_same("pattern_tagged", "true");
            }
            _ => {}
        }

        dir::walk_pattern(self, tree, id, pattern);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) {
        self.push_debug("pattern_field_kind", std::mem::discriminant(pattern_field));
        match pattern_field {
            dir::PatternField::Named { name, .. } => {
                self.push_name("pattern_field_name", *name);
            }
            dir::PatternField::Computed { .. } => {}
            dir::PatternField::Positional { .. } | dir::PatternField::Elision => {}
            dir::PatternField::Spread { .. } => {}
        }

        dir::walk_pattern_field(self, tree, id, pattern_field);
    }

    fn visit_match_case(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
    ) {
        self.push_debug("match_case_kind", std::mem::discriminant(match_case));

        let selector = match match_case {
            dir::MatchCase::Expression { selector, .. } => selector,
            dir::MatchCase::Block { selector, .. } => selector,
        };
        match selector {
            dir::MatchSelector::Pattern { guard, .. } => {
                self.push_same("match_selector_kind", "pattern");
                self.push_bool("match_selector_guard", guard.is_some());
            }
            dir::MatchSelector::Default => {
                self.push_same("match_selector_kind", "default");
            }
        }

        dir::walk_match_case(self, tree, id, match_case);
    }
}

/// Build a stable token hash for a key and prehashed value payload.
fn hash_token_hashed_value(key: &str, value_hash: u64) -> u64 {
    stable_hash_token_hashed_value(key, value_hash)
}

#[derive(Debug, Clone, Copy)]
struct SignatureHasher {
    primary_hash: u64,
    secondary_hash: u64,
}

impl SignatureHasher {
    /// Build an empty sequence hasher.
    fn new() -> Self {
        Self {
            primary_hash: 0x6f8b_84d5_3172_0143,
            secondary_hash: 0xb7e1_5162_8aed_2a6b,
        }
    }

    /// Push one token hash into the sequence.
    fn push(&mut self, token_hash: u64) {
        self.primary_hash ^= token_hash;
        self.primary_hash = self.primary_hash.wrapping_mul(0x1000_0000_01b3);
        self.primary_hash ^= self.primary_hash >> 29;

        self.secondary_hash = self.secondary_hash.rotate_left(13) ^ token_hash;
        self.secondary_hash = self.secondary_hash.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        self.secondary_hash ^= self.secondary_hash >> 31;
    }

    /// Finalize this sequence hash into a signature key.
    fn finish(self, token_count: usize) -> SignatureKey {
        SignatureKey {
            primary_hash: self.primary_hash
                ^ (token_count as u64).wrapping_mul(0xa24b_aed4_963e_ee40),
            secondary_hash: self.secondary_hash
                ^ (token_count as u64).wrapping_mul(0x9fb2_1c65_1e98_df25),
            token_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    const METAMORPHIC_CASE_COUNT: u32 = 12;

    /// Build a function source used by duplicate code tests.
    fn function_source(
        function_name: &str,
        first_name: &str,
        second_name: &str,
        multiplier: i32,
        middle_operator: &str,
        offset: i32,
        tail: i32,
    ) -> String {
        format!(
            r#"
function {function_name}(input: int32): int32 {{
    let {first_name} = input * {multiplier};
    let {second_name} = {first_name} {middle_operator} {offset};
    return {second_name} - {tail};
}}
"#
        )
    }

    /// Build a stable identifier for metamorphic test cases.
    fn identifier(seed: u32, role: &str) -> String {
        format!("v{seed}_{role}")
    }

    /// Count duplicate code diagnostics.
    fn duplicate_lint_count(diagnostics: &[LintReport]) -> usize {
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == "no-duplicate-code")
            .count()
    }

    /// Run the duplicate code rule against two module sources.
    fn lint_pair(
        first_source: &str,
        second_source: &str,
        configure: impl FnOnce(&mut destack_workspace::LinterOptions),
    ) -> Vec<LintReport> {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
                configure(options);
            });

        let first_module = test.add_module("no_duplicate_code/property_first.ds", first_source);
        let second_module = test.add_module("no_duplicate_code/property_second.ds", second_source);
        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        test.lint_workspace_dir()
    }

    /// Report exact duplicate blocks across modules.
    #[test]
    fn test_reports_exact_duplicate_blocks_across_modules() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
            });

        let first_module = test.add_module(
            "no_duplicate_code/exact_first.ds",
            r#"
function shared(x: int32): int32 {
    let value = x * 2;
    let adjusted = value + 7;
    return adjusted - 1;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/exact_second.ds",
            r#"
function sharedAgain(x: int32): int32 {
    let value = x * 2;
    let adjusted = value + 7;
    return adjusted - 1;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result)
            .assert_lint("no-duplicate-code")
            .assert_lint_count("no-duplicate-code", 2);
    }

    /// Report near duplicate blocks when identifiers differ.
    #[test]
    fn test_reports_near_duplicate_blocks_for_renamed_identifiers() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
            });

        let first_module = test.add_module(
            "no_duplicate_code/near_first.ds",
            r#"
function first(input: int32): int32 {
    let running = input * 3;
    let total = running + 9;
    return total - 2;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/near_second.ds",
            r#"
function second(value: int32): int32 {
    let factor = value * 3;
    let result = factor + 9;
    return result - 2;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result)
            .assert_lint("no-duplicate-code")
            .assert_lint_count("no-duplicate-code", 2);
    }

    /// Report near duplicates for literal mutations.
    #[test]
    fn test_reports_near_duplicates_for_literal_mutations() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
            });

        let first_module = test.add_module(
            "no_duplicate_code/literal_first.ds",
            r#"
function first(x: int32): int32 {
    let value = x * 2;
    let adjusted = value + 7;
    return adjusted - 1;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/literal_second.ds",
            r#"
function second(x: int32): int32 {
    let value = x * 2;
    let adjusted = value + 9;
    return adjusted - 3;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result)
            .assert_lint("no-duplicate-code")
            .assert_lint_count("no-duplicate-code", 2);
    }

    /// Respect the near duplicate similarity threshold.
    #[test]
    fn test_respects_near_duplicate_similarity_threshold() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
                options.complexity.min_duplicate_code_near_similarity = 95;
            });

        let first_module = test.add_module(
            "no_duplicate_code/similarity_high_threshold_first.ds",
            r#"
function first(input: int32): int32 {
    let total = input * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/similarity_high_threshold_second.ds",
            r#"
function second(input: int32): int32 {
    let total = abs(input);
    let adjusted = total / 3;
    return adjusted + 11;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result).assert_no_lint("no-duplicate-code");
    }

    /// Detect near duplicates once the threshold is relaxed.
    #[test]
    fn test_detects_near_duplicates_with_relaxed_similarity_threshold() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
                options.complexity.min_duplicate_code_near_similarity = 60;
            });

        let first_module = test.add_module(
            "no_duplicate_code/similarity_low_threshold_first.ds",
            r#"
function first(input: int32): int32 {
    let total = input * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/similarity_low_threshold_second.ds",
            r#"
function second(input: int32): int32 {
    let total = abs(input);
    let adjusted = total / 3;
    return adjusted + 11;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result)
            .assert_lint("no-duplicate-code")
            .assert_lint_count("no-duplicate-code", 2);
    }

    /// Allow disabling near duplicate checks by setting threshold to zero.
    #[test]
    fn test_allows_disabling_near_duplicate_detection() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
                options.complexity.min_duplicate_code_near_similarity = 0;
            });

        let first_module = test.add_module(
            "no_duplicate_code/disabled_near_first.ds",
            r#"
function first(input: int32): int32 {
    let running = input * 3;
    let total = running + 9;
    return total - 2;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/disabled_near_second.ds",
            r#"
function second(value: int32): int32 {
    let factor = value * 3;
    let result = factor + 9;
    return result - 2;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result).assert_no_lint("no-duplicate-code");
    }

    /// Respect duplicate thresholds.
    #[test]
    fn test_respects_duplicate_thresholds() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 8;
                options.complexity.min_duplicate_code_tokens = 64;
            });

        let first_module = test.add_module(
            "no_duplicate_code/threshold_first.ds",
            r#"
function shared(x: int32): int32 {
    let value = x + 1;
    return value;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/threshold_second.ds",
            r#"
function sharedAgain(x: int32): int32 {
    let value = x + 1;
    return value;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result).assert_no_lint("no-duplicate-code");
    }

    /// Report all duplicate code diagnostics in one group.
    #[test]
    fn test_reports_all_duplicate_code_diagnostics() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
            });

        let first_module = test.add_module(
            "no_duplicate_code/cap_first.ds",
            r#"
function one(x: int32): int32 {
    let value = x * 2;
    let adjusted = value + 7;
    return adjusted - 1;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/cap_second.ds",
            r#"
function two(x: int32): int32 {
    let value = x * 2;
    let adjusted = value + 7;
    return adjusted - 1;
}
"#,
        );
        let third_module = test.add_module(
            "no_duplicate_code/cap_third.ds",
            r#"
function three(x: int32): int32 {
    let value = x * 2;
    let adjusted = value + 7;
    return adjusted - 1;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.import_module(third_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result)
            .assert_lint("no-duplicate-code")
            .assert_lint_count("no-duplicate-code", 3);
    }

    /// Report near duplicates when literals differ.
    #[test]
    fn test_reports_near_duplicates_when_literals_differ() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
            });

        let first_module = test.add_module(
            "no_duplicate_code/literal_norm_first.ds",
            r#"
function first(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 3;
    return adjusted - 1;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/literal_norm_second.ds",
            r#"
function second(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result)
            .assert_lint("no-duplicate-code")
            .assert_lint_count("no-duplicate-code", 2);
    }

    /// Report exact duplicates when near duplicate checks are disabled.
    #[test]
    fn test_reports_exact_duplicates_when_near_duplicate_detection_is_disabled() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
                options.complexity.min_duplicate_code_near_similarity = 0;
            });

        let first_module = test.add_module(
            "no_duplicate_code/exact_without_near_first.ds",
            r#"
function first(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/exact_without_near_second.ds",
            r#"
function second(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result)
            .assert_lint("no-duplicate-code")
            .assert_lint_count("no-duplicate-code", 2);
    }

    /// Skip declaration files by default.
    #[test]
    fn test_skips_declaration_files_by_default() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
            });

        let first_module = test.add_module(
            "no_duplicate_code/decl_first.d.ds",
            r#"
function first(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/decl_second.d.ds",
            r#"
function second(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result).assert_no_lint("no-duplicate-code");
    }

    /// Include declaration files when configured.
    #[test]
    fn test_includes_declaration_files_when_enabled() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
                options.include_declaration_files = true;
            });

        let first_module = test.add_module(
            "no_duplicate_code/decl_enabled_first.d.ds",
            r#"
function first(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/decl_enabled_second.d.ds",
            r#"
function second(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        test.result(result)
            .assert_lint("no-duplicate-code")
            .assert_lint_count("no-duplicate-code", 2);
    }

    /// Label duplicates in the same file as local duplicates.
    #[test]
    fn test_labels_same_file_duplicates_as_local() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
            });

        let module = test.add_module(
            "no_duplicate_code/same_file_duplicates.ds",
            r#"
function first(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}

function second(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );

        test.import_module(module);
        test.compile();

        let result = test.lint_workspace_dir();
        let lint_result = test.result(result);
        lint_result
            .assert_lint("no-duplicate-code")
            .assert_lint_count("no-duplicate-code", 2);

        let diagnostics = lint_result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == "no-duplicate-code")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic
                .label_message()
                .is_some_and(|label| label.contains("another block in this file"))
        }));
    }

    /// Emit stable diagnostic metadata for duplicate code reports.
    #[test]
    fn test_emits_expected_diagnostic_metadata() {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoDuplicateCode)])
            .with_options(|options| {
                options.complexity.min_duplicate_code_lines = 3;
                options.complexity.min_duplicate_code_tokens = 12;
            });

        let first_module = test.add_module(
            "no_duplicate_code/metadata_first.ds",
            r#"
function first(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );
        let second_module = test.add_module(
            "no_duplicate_code/metadata_second.ds",
            r#"
function second(x: int32): int32 {
    let total = x * 2;
    let adjusted = total + 9;
    return adjusted - 5;
}
"#,
        );

        test.import_module(first_module);
        test.import_module(second_module);
        test.compile();

        let result = test.lint_workspace_dir();
        let lint_result = test.result(result);
        lint_result.assert_lint_count("no-duplicate-code", 2);

        let diagnostics = lint_result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == "no-duplicate-code")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code() == NO_DUPLICATE_CODE.code)
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message() == "duplicate code block")
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.notes().any(|note| note.contains("lines")))
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.notes().any(|note| note.contains("tokens")))
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.has_no_fixes())
        );
    }

    /// Detect near duplicates for identifier-only mutations across generated cases.
    #[test]
    fn test_metamorphic_identifier_mutations_detect_near_duplicates() {
        for seed in 0..METAMORPHIC_CASE_COUNT {
            let multiplier = 2 + (seed % 3) as i32;
            let offset = 5 + seed as i32;
            let tail = 1 + (seed % 4) as i32;

            let first_source = function_source(
                "first",
                &identifier(seed, "left"),
                &identifier(seed, "middle"),
                multiplier,
                "+",
                offset,
                tail,
            );
            let second_source = function_source(
                "second",
                &identifier(seed + 100, "left"),
                &identifier(seed + 100, "middle"),
                multiplier,
                "+",
                offset,
                tail,
            );

            let diagnostics = lint_pair(&first_source, &second_source, |_| {});
            let count = duplicate_lint_count(&diagnostics);
            assert_eq!(
                count, 2,
                "seed {seed} should produce two near duplicate diagnostics"
            );
        }
    }

    /// Detect literal mutations as near duplicates in generated cases.
    #[test]
    fn test_metamorphic_literal_mutations_match() {
        for seed in 0..METAMORPHIC_CASE_COUNT {
            let multiplier = 2 + (seed % 3) as i32;
            let first_offset = 10 + seed as i32;
            let second_offset = first_offset + 3;
            let first_tail = 2 + (seed % 5) as i32;
            let second_tail = first_tail + 2;

            let first_source = function_source(
                "first",
                &identifier(seed, "left"),
                &identifier(seed, "middle"),
                multiplier,
                "+",
                first_offset,
                first_tail,
            );
            let second_source = function_source(
                "second",
                &identifier(seed + 100, "left"),
                &identifier(seed + 100, "middle"),
                multiplier,
                "+",
                second_offset,
                second_tail,
            );

            let diagnostics = lint_pair(&first_source, &second_source, |_| {});
            let with_count = duplicate_lint_count(&diagnostics);
            assert_eq!(
                with_count, 2,
                "seed {seed} should match with literal mutations"
            );
        }
    }

    /// Avoid duplicate reports for structural mutations in generated cases.
    #[test]
    fn test_metamorphic_structural_mutations_do_not_match() {
        for seed in 0..METAMORPHIC_CASE_COUNT {
            let multiplier = 2 + (seed % 3) as i32;
            let offset = 7 + seed as i32;
            let tail = 1 + (seed % 4) as i32;

            let first_source = function_source(
                "first",
                &identifier(seed, "left"),
                &identifier(seed, "middle"),
                multiplier,
                "+",
                offset,
                tail,
            );
            let second_source = function_source(
                "second",
                &identifier(seed + 100, "left"),
                &identifier(seed + 100, "middle"),
                multiplier,
                "-",
                offset,
                tail,
            );

            let diagnostics = lint_pair(&first_source, &second_source, |_| {});
            let count = duplicate_lint_count(&diagnostics);
            assert_eq!(
                count, 0,
                "seed {seed} should not match after structural mutation"
            );
        }
    }

    /// Avoid emitting near duplicate titles for exact duplicate groups.
    #[test]
    fn test_exact_duplicates_do_not_emit_near_duplicate_title() {
        let first_source = function_source("first", "lhs", "rhs", 3, "+", 9, 2);
        let second_source = function_source("second", "lhs", "rhs", 3, "+", 9, 2);

        let diagnostics = lint_pair(&first_source, &second_source, |_| {});
        assert_eq!(duplicate_lint_count(&diagnostics), 2);
        assert!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.rule_id == "no-duplicate-code")
                .all(|diagnostic| diagnostic.message() == "duplicate code block")
        );
    }
}
