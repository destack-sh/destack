/// A metavariable marker recognized in Pattern source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternMarker<'source> {
    /// One structural node or name.
    Node {
        /// The name without its leading dollar sign.
        name: &'source str,
    },
    /// Zero or more structural nodes.
    Nodes {
        /// The name without its leading dollar signs.
        name: &'source str,
    },
}

impl<'source> PatternMarker<'source> {
    /// Parse one complete identifier-like source token.
    pub fn parse(text: &'source str) -> Option<Self> {
        let (name, marker) = if let Some(name) = text.strip_prefix("$$$") {
            (name, Self::Nodes { name })
        } else {
            let name = text.strip_prefix('$')?;

            (name, Self::Node { name })
        };
        let first = name.as_bytes().first().copied()?;
        if first != b'_' && !first.is_ascii_uppercase() {
            return None;
        }
        if !name
            .bytes()
            .all(|byte| byte == b'_' || byte.is_ascii_uppercase() || byte.is_ascii_digit())
        {
            return None;
        }

        Some(marker)
    }

    /// Return the name without leading dollar signs.
    pub fn name(self) -> &'source str {
        match self {
            Self::Node { name } | Self::Nodes { name } => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PatternMarker;

    /// Recognize only complete uppercase Pattern marker tokens.
    #[test]
    fn test_parse_pattern_marker() {
        let markers = [
            ("$NODE", Some(PatternMarker::Node { name: "NODE" })),
            ("$_", Some(PatternMarker::Node { name: "_" })),
            ("$NODE_2", Some(PatternMarker::Node { name: "NODE_2" })),
            ("$$$NODES", Some(PatternMarker::Nodes { name: "NODES" })),
            ("$$$_", Some(PatternMarker::Nodes { name: "_" })),
            ("$name", None),
            ("$Mixed", None),
            ("$9NODE", None),
            ("$NODE-name", None),
            ("$", None),
            ("$$NODES", None),
            ("$$$$NODES", None),
            ("NODE", None),
        ];

        for (source, expected) in markers {
            assert_eq!(PatternMarker::parse(source), expected, "{source}");
        }
    }
}
