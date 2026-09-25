use serde::{Deserialize, Serialize};
use tspp_core::FNV_PRIME_128;
use tspp_serde::Reflect;

use crate::Postings;

/// Code fingerprints and adjacent pairs in one module.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CodeIndex {
    /// Name-insensitive structural fingerprints keyed densely by visible node id.
    fingerprints: Vec<CodeFingerprint>,
    /// Visible node ids ordered by name-insensitive structural fingerprint.
    order: Vec<u32>,
    /// Adjacent node pairs ordered by name-insensitive structural fingerprint.
    pairs: Vec<CodePair>,
}

/// Code region postings by module ordinal.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct CodePostings {
    /// Module postings for visible node fingerprints.
    pub nodes: Postings<CodeFingerprint>,
    /// Module postings for adjacent pair fingerprints.
    pub pairs: Postings<CodeFingerprint>,
}

impl CodeIndex {
    /// Create an index from dense fingerprints and visible regions.
    pub fn new(fingerprints: Vec<CodeFingerprint>, order: Vec<u32>, pairs: Vec<CodePair>) -> Self {
        let mut index = Self {
            fingerprints,
            order,
            pairs,
        };
        index.finish();

        index
    }

    /// Sort this index for lookup.
    pub fn finish(&mut self) {
        // order visible nodes by fingerprint and identity
        self.order
            .sort_unstable_by_key(|node| (self.fingerprints[*node as usize], *node));
        self.order.dedup();

        // order adjacent pairs by fingerprint and identity
        self.pairs
            .sort_unstable_by_key(|pair| (pair.fingerprint, pair.first, pair.second));
        self.pairs.dedup();
    }

    /// Return one visible node's name-insensitive structural fingerprint.
    pub fn fingerprint(&self, node: u32) -> Option<CodeFingerprint> {
        let fingerprint = *self.fingerprints.get(node as usize)?;

        (fingerprint.node_count != 0).then_some(fingerprint)
    }

    /// Iterate visible node ids carrying one name-insensitive structural fingerprint.
    pub fn nodes(&self, fingerprint: CodeFingerprint) -> impl Iterator<Item = u32> + '_ {
        let start = self
            .order
            .partition_point(|node| self.fingerprints[*node as usize] < fingerprint);
        let end = self.order[start..]
            .partition_point(|node| self.fingerprints[*node as usize] == fingerprint)
            + start;

        self.order[start..end].iter().copied()
    }

    /// Iterate adjacent pairs carrying one name-insensitive structural fingerprint.
    pub fn pairs(&self, fingerprint: CodeFingerprint) -> impl Iterator<Item = CodePair> + '_ {
        let start = self
            .pairs
            .partition_point(|pair| pair.fingerprint < fingerprint);
        let end =
            self.pairs[start..].partition_point(|pair| pair.fingerprint == fingerprint) + start;

        self.pairs[start..end].iter().copied()
    }

    /// Iterate the indexed node fingerprints.
    pub fn fingerprints(&self) -> impl Iterator<Item = CodeFingerprint> + '_ {
        self.order
            .iter()
            .map(|node| self.fingerprints[*node as usize])
    }

    /// Iterate the indexed adjacent-pair fingerprints.
    pub fn pair_fingerprints(&self) -> impl Iterator<Item = CodeFingerprint> + '_ {
        self.pairs.iter().map(|pair| pair.fingerprint)
    }
}

impl CodePostings {
    /// Build code postings from module indexes.
    pub fn build(indexes: &[&CodeIndex]) -> Self {
        let nodes = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .fingerprints()
                .map(move |fingerprint| (fingerprint, module))
        }));
        let pairs = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .pair_fingerprints()
                .map(move |fingerprint| (fingerprint, module))
        }));

        Self { nodes, pairs }
    }

    /// Replace postings for one module code index.
    pub fn update(&mut self, module: u32, index: &CodeIndex) {
        self.nodes.replace(module, index.fingerprints());
        self.pairs.replace(module, index.pair_fingerprints());
    }
}

/// One stable name-insensitive structural fingerprint for a visible DIR region.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
)]
pub struct CodeFingerprint {
    /// The stable hash value.
    value: u128,
    /// The number of visible nodes represented.
    node_count: u32,
    /// The number of concatenated code regions represented.
    region_count: u32,
}

impl CodeFingerprint {
    /// Create a name-insensitive structural fingerprint for one visible code region.
    pub const fn new(value: u128, node_count: u32) -> Self {
        Self {
            value,
            node_count,
            region_count: 1,
        }
    }

    /// Return the stable hash value.
    pub const fn value(self) -> u128 {
        self.value
    }

    /// Return the represented visible node count.
    pub const fn node_count(self) -> u32 {
        self.node_count
    }

    /// Return the represented code region count.
    pub const fn region_count(self) -> u32 {
        self.region_count
    }

    /// Concatenate fingerprints using the 128-bit FNV prime as the rolling multiplier.
    pub fn concatenate(fingerprints: impl IntoIterator<Item = Self>) -> Option<Self> {
        let mut fingerprints = fingerprints.into_iter();
        let mut combined = fingerprints
            .next()
            .filter(|fingerprint| fingerprint.node_count != 0 && fingerprint.region_count != 0)?;

        // concatenate each remaining region in source order
        for fingerprint in fingerprints {
            if fingerprint.node_count == 0 || fingerprint.region_count == 0 {
                return None;
            }

            let node_count = combined.node_count.checked_add(fingerprint.node_count)?;
            let region_count = combined
                .region_count
                .checked_add(fingerprint.region_count)?;
            let shift = FNV_PRIME_128.wrapping_pow(fingerprint.region_count);
            let value = combined
                .value
                .wrapping_mul(shift)
                .wrapping_add(fingerprint.value);
            combined = Self {
                value,
                node_count,
                region_count,
            };
        }

        Some(combined)
    }
}

/// One adjacent visible node pair indexed as a code region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct CodePair {
    /// The combined name-insensitive structural fingerprint.
    pub fingerprint: CodeFingerprint,
    /// The first visible node id.
    pub first: u32,
    /// The second visible node id.
    pub second: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserve region identity and grouping during fingerprint concatenation.
    #[test]
    fn test_concatenate_fingerprints() {
        let first = CodeFingerprint::new(3, 5);
        let second = CodeFingerprint::new(7, 11);
        let third = CodeFingerprint::new(13, 17);

        // compare both groupings with one direct concatenation
        let direct = CodeFingerprint::concatenate([first, second, third]);
        let left = CodeFingerprint::concatenate([first, second])
            .and_then(|pair| CodeFingerprint::concatenate([pair, third]));
        let right = CodeFingerprint::concatenate([second, third])
            .and_then(|pair| CodeFingerprint::concatenate([first, pair]));

        assert_eq!(direct, left);
        assert_eq!(direct, right);
        assert_eq!(CodeFingerprint::concatenate([first]), Some(first));
    }
}
