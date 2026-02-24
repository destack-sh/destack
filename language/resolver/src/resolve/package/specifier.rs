use crate::Resolver;
use std::cmp::Ordering;

impl Resolver {
    pub(crate) fn parse_package_specifier(specifier: &str) -> (&str, &str) {
        // find first slash
        let mut separator_index = specifier.as_bytes().iter().position(|b| *b == b'/');

        // scoped packages have format `@scope/package-name/subpath`
        if specifier.starts_with('@') {
            if separator_index.is_none() || specifier.is_empty() {
                // fall through with no separator
            } else if let Some(first_slash) = separator_index {
                separator_index = specifier.as_bytes()[first_slash + 1..]
                    .iter()
                    .position(|b| *b == b'/')
                    .map(|offset| offset + first_slash + 1);
            }
        }

        // split at separator
        let package_name = separator_index.map_or(specifier, |index| &specifier[..index]);
        let package_subpath = separator_index.map_or("", |index| &specifier[index..]);
        (package_name, package_subpath)
    }

    /// Compare two pattern keys for specificity ordering.
    pub(super) fn pattern_key_compare(key_a: &str, key_b: &str) -> Ordering {
        if key_a.is_empty() {
            return Ordering::Greater;
        }

        // ensure pattern keys are actual pattern keys
        debug_assert!(
            key_a.ends_with('/') || key_a.match_indices('*').count() == 1,
            "{key_a}"
        );
        debug_assert!(
            key_b.ends_with('/') || key_b.match_indices('*').count() == 1,
            "{key_b}"
        );

        // compare pattern keys
        let a_pos = key_a.bytes().position(|c| c == b'*');
        let base_length_a = a_pos.map_or(key_a.len(), |p| p + 1);
        let b_pos = key_b.bytes().position(|c| c == b'*');
        let base_length_b = b_pos.map_or(key_b.len(), |p| p + 1);
        if base_length_a > base_length_b {
            Ordering::Less
        } else if base_length_b > base_length_a || a_pos.is_none() {
            Ordering::Greater
        } else if b_pos.is_none() || key_a.len() > key_b.len() {
            Ordering::Less
        } else if key_b.len() > key_a.len() {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}
