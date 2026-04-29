use std::borrow::Cow;

/// Module dependency specifier like `./foo.js` or `#internal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSpecifier {
    /// The path part without query or fragment.
    pub path: String,
    /// The optional query, including `?`.
    pub query: Option<String>,
    /// The optional fragment, including `#`.
    pub fragment: Option<String>,
}

impl ModuleSpecifier {
    /// Return the path part without query or fragment.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Parse one module dependency specifier.
    pub fn parse(specifier: &str) -> Self {
        // empty specifier
        if specifier.is_empty() {
            return Self {
                path: String::new(),
                query: None,
                fragment: None,
            };
        }

        // skip leading path marker
        let offset = match specifier.as_bytes()[0] {
            b'/' | b'.' | b'#' => 1,
            _ => 0,
        };

        // scanner state
        let mut query_start = None;
        let mut fragment_start = None;
        let Some(mut previous) = specifier.chars().next() else {
            return Self {
                path: String::new(),
                query: None,
                fragment: None,
            };
        };
        let mut escaped_indexes = None;

        // query and fragment split points
        for (index, character) in specifier.char_indices().skip(offset) {
            if character == '?' && query_start.is_none() {
                query_start = Some(index);
            }

            // fragment or escaped hash
            if character == '#' {
                if previous == '\0' {
                    escaped_indexes.get_or_insert_with(Vec::new).push(index - 1);
                } else {
                    fragment_start = Some(index);
                    break;
                }
            }

            previous = character;
        }

        // raw component slices
        let (path, query, fragment) = match (query_start, fragment_start) {
            (Some(query_start), Some(fragment_start)) => (
                &specifier[..query_start],
                Some(&specifier[query_start..fragment_start]),
                Some(&specifier[fragment_start..]),
            ),
            (Some(query_start), None) => (
                &specifier[..query_start],
                Some(&specifier[query_start..]),
                None,
            ),
            (None, Some(fragment_start)) => (
                &specifier[..fragment_start],
                None,
                Some(&specifier[fragment_start..]),
            ),
            (None, None) => (specifier, None, None),
        };

        // escaped hash removal
        let path = escaped_indexes.map_or(Cow::Borrowed(path), |escaped_indexes| {
            Cow::Owned(
                path.chars()
                    .enumerate()
                    .filter_map(|(index, character)| {
                        (!escaped_indexes.contains(&index)).then_some(character)
                    })
                    .collect::<String>(),
            )
        });

        // owned specifier parts
        Self {
            path: path.to_string(),
            query: query.map(str::to_string),
            fragment: fragment.map(str::to_string),
        }
    }
}
