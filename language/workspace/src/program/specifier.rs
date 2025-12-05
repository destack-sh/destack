use std::borrow::Cow;

/// Module dependency specifier (like `./foo.js` or `../bar.js` or `#baz`).
#[derive(Debug)]
pub struct ModuleSpecifier {
    /// Path (like `./foo.js` or `../bar.js`), without query or fragment.
    pub path: String,
    /// Query `?query`, contains `?` (like `?foo` in `foo.js?foo`).
    pub query: Option<String>,
    /// Fragment `#query`, contains `#` (like `#foo` in `foo.js#foo`).
    pub fragment: Option<String>,
}

impl ModuleSpecifier {
    /// Returns the module path, without query or fragment.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Parses a module dependency specifier from a string.
    pub fn parse(specifier: &str) -> Self {
        if specifier.is_empty() {
            return Self {
                path: String::new(),
                query: None,
                fragment: None,
            };
        }

        // determine offset for certain leading characters
        let offset = match specifier.as_bytes()[0] {
            b'/' | b'.' | b'#' => 1,
            _ => 0,
        };

        let mut query_start: Option<usize> = None;
        let mut fragment_start: Option<usize> = None;
        let mut prev = specifier.chars().next().unwrap();

        // optimize for the common case (no escaped chars)
        let mut escaped_indexes: Option<Vec<usize>> = None;
        for (i, c) in specifier.char_indices().skip(offset) {
            if c == '?' && query_start.is_none() {
                query_start = Some(i);
            }
            if c == '#' {
                if prev == '\0' {
                    // escape for # with \0
                    escaped_indexes.get_or_insert_with(Vec::new).push(i - 1);
                } else {
                    fragment_start = Some(i);
                    break;
                }
            }
            prev = c;
        }

        // parse main path, query, and fragment ranges
        let (path_raw, query, fragment) = match (query_start, fragment_start) {
            (Some(i), Some(j)) => {
                debug_assert!(i < j);
                (
                    &specifier[..i],
                    Some(&specifier[i..j]),
                    Some(&specifier[j..]),
                )
            }
            (Some(i), None) => (&specifier[..i], Some(&specifier[i..]), None),
            (None, Some(j)) => (&specifier[..j], None, Some(&specifier[j..])),
            _ => (specifier, None, None),
        };

        // remove any escaped indexes from path
        let path = escaped_indexes.map_or(Cow::Borrowed(path_raw), |escaped_indexes| {
            Cow::Owned(
                path_raw
                    .chars()
                    .enumerate()
                    .filter_map(|(i, c)| (!escaped_indexes.contains(&i)).then_some(c))
                    .collect::<String>(),
            )
        });

        Self {
            path: path.to_string(),
            query: query.map(|q| q.to_string()),
            fragment: fragment.map(|f| f.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ModuleSpecifier;

    /// Parse an absolute specifier.
    #[test]
    fn test_parse_absolute_specifier() {
        let specifier = "/test?#";
        let parsed = ModuleSpecifier::parse(specifier);
        assert_eq!(parsed.path, "/test");
        assert_eq!(parsed.query, Some("?".to_string()));
        assert_eq!(parsed.fragment, Some("#".to_string()));
    }

    /// Parse a relative specifier.
    #[test]
    fn test_parse_relative_specifier() {
        let specifiers = ["./test", "../test", "../../test"];
        for specifier in specifiers {
            let mut r = specifier.to_string();
            r.push_str("?#");
            let parsed = ModuleSpecifier::parse(&r);
            assert_eq!(parsed.path, specifier);
            assert_eq!(parsed.query, Some("?".to_string()));
            assert_eq!(parsed.fragment, Some("#".to_string()));
        }
    }

    /// Parse a hash specifier.
    #[test]
    fn test_parse_hash_specifier() {
        let specifiers = ["#", "#path"];
        for specifier in specifiers {
            let mut r = specifier.to_string();
            r.push_str("?#");
            let parsed = ModuleSpecifier::parse(&r);
            assert_eq!(parsed.path, specifier);
            assert_eq!(parsed.query, Some("?".to_string()));
            assert_eq!(parsed.fragment, Some("#".to_string()));
        }
    }

    /// Parse a module specifier.
    #[test]
    fn test_parse_module_specifier() {
        let specifiers = ["module"];
        for specifier in specifiers {
            let mut r = specifier.to_string();
            r.push_str("?#");
            let parsed = ModuleSpecifier::parse(&r);
            assert_eq!(parsed.path, specifier);
            assert_eq!(parsed.query, Some("?".to_string()));
            assert_eq!(parsed.fragment, Some("#".to_string()));
        }
    }

    /// Parse a query and fragment specifier.
    #[test]
    fn test_parse_query_fragment_specifier() {
        let data = [
            ("a?", Some("?"), None),
            ("a?query", Some("?query"), None),
            ("a?query1?query2", Some("?query1?query2"), None),
            (
                "a?query1?query2?query3",
                Some("?query1?query2?query3"),
                None,
            ),
            ("a#", None, Some("#")),
            ("a#b#c", None, Some("#b#c")),
            ("a#fragment", None, Some("#fragment")),
            ("a?#", Some("?"), Some("#")),
            ("a?#fragment", Some("?"), Some("#fragment")),
            ("a?query#", Some("?query"), Some("#")),
            ("a?query#fragment", Some("?query"), Some("#fragment")),
            ("a#fragment?", None, Some("#fragment?")),
            ("a#fragment?query", None, Some("#fragment?query")),
        ];

        for (specifier_str, query, fragment) in data {
            let specifier = ModuleSpecifier::parse(specifier_str);
            assert_eq!(specifier.path, "a", "{specifier_str}");
            assert_eq!(
                specifier.query,
                query.map(|q| q.to_string()),
                "{specifier_str}"
            );
            assert_eq!(
                specifier.fragment,
                fragment.map(|f| f.to_string()),
                "{specifier_str}"
            );
        }
    }

    /// Test parsing enhanced-resolve edge cases.
    #[test]
    // https://github.com/webpack/enhanced-resolve/blob/main/test/identifier.test.js
    fn test_parse_enhanced_resolve_edge_cases() {
        let data = [
            ("path/#", "path/", "", "#"),
            ("path/as/?", "path/as/", "?", ""),
            ("path/#/?", "path/", "", "#/?"),
            ("path/#repo#hash", "path/", "", "#repo#hash"),
            ("path/#r#hash", "path/", "", "#r#hash"),
            ("path/#repo/#repo2#hash", "path/", "", "#repo/#repo2#hash"),
            ("path/#r/#r#hash", "path/", "", "#r/#r#hash"),
            (
                "path/#/not/a/hash?not-a-query",
                "path/",
                "",
                "#/not/a/hash?not-a-query",
            ),
        ];

        for (specifier_str, path, query, fragment) in data {
            let specifier = ModuleSpecifier::parse(specifier_str);
            assert_eq!(specifier.path, path, "{specifier_str}");
            assert_eq!(
                specifier.query.unwrap_or("".to_string()),
                query,
                "{specifier_str}"
            );
            assert_eq!(
                specifier.fragment.unwrap_or("".to_string()),
                fragment,
                "{specifier_str}"
            );
        }
    }

    /// Test parsing enhanced-resolve windows-like specifiers.
    /// https://github.com/webpack/enhanced-resolve/blob/main/test/identifier.test.js
    #[test]
    fn test_parse_enhanced_resolve_windows_like() {
        let data = [
            ("path\\#", "path\\", "", "#"),
            ("path\\as\\?", "path\\as\\", "?", ""),
            ("path\\#\\?", "path\\", "", "#\\?"),
            ("path\\#repo#hash", "path\\", "", "#repo#hash"),
            ("path\\#r#hash", "path\\", "", "#r#hash"),
            (
                "path\\#repo\\#repo2#hash",
                "path\\",
                "",
                "#repo\\#repo2#hash",
            ),
            ("path\\#r\\#r#hash", "path\\", "", "#r\\#r#hash"),
            (
                "path\\#/not/a/hash?not-a-query",
                "path\\",
                "",
                "#/not/a/hash?not-a-query",
            ),
        ];

        for (specifier_str, path, query, fragment) in data {
            let specifier = ModuleSpecifier::parse(specifier_str);
            assert_eq!(specifier.path, path, "{specifier_str}");
            assert_eq!(
                specifier.query.unwrap_or("".to_string()),
                query,
                "{specifier_str}"
            );
            assert_eq!(
                specifier.fragment.unwrap_or("".to_string()),
                fragment,
                "{specifier_str}"
            );
        }
    }
}
