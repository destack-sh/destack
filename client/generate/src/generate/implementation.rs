use crate::generate::schema::{Item, SchemaModule};

/// Handwritten client implementation owners.
const IMPLEMENTATION_OWNERS: &[ImplementationOwner] = &[
    ImplementationOwner {
        module: &["core", "bitset"],
        item: "BitSet",
    },
    ImplementationOwner {
        module: &["dir", "symbol", "key"],
        item: "StaticKey",
    },
    ImplementationOwner {
        module: &["dir", "source", "comment"],
        item: "CommentNewlines",
    },
    ImplementationOwner {
        module: &["dir", "tree", "dependency"],
        item: "DependencyItem",
    },
    ImplementationOwner {
        module: &["dir", "tree", "argument"],
        item: "GenericParameter",
    },
    ImplementationOwner {
        module: &["dir", "tree", "argument"],
        item: "Parameter",
    },
    ImplementationOwner {
        module: &["dir", "tree", "block"],
        item: "Block",
    },
    ImplementationOwner {
        module: &["dir", "tree", "declaration"],
        item: "Declaration",
    },
    ImplementationOwner {
        module: &["dir", "tree", "expression"],
        item: "Expression",
    },
    ImplementationOwner {
        module: &["dir", "tree", "key"],
        item: "Name",
    },
    ImplementationOwner {
        module: &["dir", "tree", "key"],
        item: "Key",
    },
    ImplementationOwner {
        module: &["dir", "tree", "path"],
        item: "Path",
    },
    ImplementationOwner {
        module: &["dir", "tree", "property"],
        item: "Property",
    },
    ImplementationOwner {
        module: &["dir", "tree", "property"],
        item: "Member",
    },
    ImplementationOwner {
        module: &["mir", "tree", "attribute"],
        item: "FloatValue",
    },
    ImplementationOwner {
        module: &["mir", "tree", "value"],
        item: "ValueSlice",
    },
    ImplementationOwner {
        module: &["mir", "tree", "value"],
        item: "Value",
    },
    ImplementationOwner {
        module: &["mir", "analyses", "link_graph"],
        item: "CallComponentGraph",
    },
    ImplementationOwner {
        module: &["mir", "analyses", "link_graph"],
        item: "LinkGraph",
    },
    ImplementationOwner {
        module: &["query", "assist", "semantic"],
        item: "SemanticTokenModifiers",
    },
];

/// One client owner with handwritten implementation.
pub(crate) struct ImplementationOwner {
    /// Generated module path segments.
    module: &'static [&'static str],
    /// Generated item name.
    item: &'static str,
}

impl ImplementationOwner {
    /// Return whether this owner matches one generated item.
    pub(crate) fn matches(&self, item: &Item) -> bool {
        self.item == item.name
    }

    /// Return this owner TypeScript implementation object.
    pub(crate) fn typescript_impl(&self) -> String {
        format!("{}Impl", self.item)
    }

    /// Return this owner Python implementation class.
    pub(crate) fn python_impl(&self) -> String {
        format!("{}Impl", self.item)
    }

    /// Return this owner public module segments.
    pub(crate) fn module(&self) -> &[&'static str] {
        self.module
    }
}

/// Return handwritten implementation owners for one generated module.
pub(crate) fn implementation_owners(module: &SchemaModule) -> Vec<&'static ImplementationOwner> {
    IMPLEMENTATION_OWNERS
        .iter()
        .filter(|owner| {
            module
                .path
                .segments()
                .iter()
                .map(String::as_str)
                .eq(owner.module.iter().copied())
        })
        .collect()
}
