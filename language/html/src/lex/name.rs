use std::fmt;

use super::{HtmlString, LocalName, Namespace, Prefix};

/// One expanded HTML name.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub(crate) struct ExpandedName<'a> {
    /// The resolved namespace.
    pub(crate) ns: &'a Namespace,
    /// The local name.
    pub(crate) local: &'a LocalName,
}

impl fmt::Debug for ExpandedName<'_> {
    /// Format this expanded name for debugging.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ns.is_empty() {
            write!(formatter, "{}", self.local)
        } else {
            write!(formatter, "{{{}}}:{}", self.ns, self.local)
        }
    }
}

/// One fully qualified HTML name.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub(crate) struct QualifiedName {
    /// The unresolved prefix when one exists.
    pub(crate) prefix: Option<Prefix>,
    /// The resolved namespace.
    pub(crate) ns: Namespace,
    /// The local name.
    pub(crate) local: LocalName,
}

impl QualifiedName {
    /// Build one qualified name.
    pub(crate) fn new(prefix: Option<Prefix>, ns: Namespace, local: LocalName) -> Self {
        Self { prefix, ns, local }
    }

    /// Return one expanded view over this name.
    pub(crate) fn expanded(&self) -> ExpandedName<'_> {
        ExpandedName {
            ns: &self.ns,
            local: &self.local,
        }
    }
}

/// One lexer attribute.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub(crate) struct Attribute {
    /// The attribute name.
    pub(crate) name: QualifiedName,
    /// The attribute value.
    pub(crate) value: HtmlString,
}

/// Build one expanded name literal.
macro_rules! expanded_name {
    ("", $local:tt) => {
        $crate::lex::ExpandedName {
            ns: &$crate::lex::ns!(),
            local: &$crate::lex::local_name!($local),
        }
    };
    ($ns:ident $local:tt) => {
        $crate::lex::ExpandedName {
            ns: &$crate::lex::ns!($ns),
            local: &$crate::lex::local_name!($local),
        }
    };
}

/// Re-export the expanded-name literal macro.
pub(crate) use expanded_name;
