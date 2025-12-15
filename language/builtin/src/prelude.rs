/// An item that should be implicitly available in every module.
///
/// Prelude items are injected into module scopes during binding,
/// allowing code to reference them without explicit imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PreludeItem {
    /// The name as it appears in scope.
    pub name: &'static str,
    /// The module path (relative to core/).
    pub module: &'static str,
    /// The export name in that module.
    pub export: &'static str,
}

impl PreludeItem {
    const fn new(name: &'static str, module: &'static str, export: &'static str) -> Self {
        Self {
            name,
            module,
            export,
        }
    }
}

/// All prelude items.
pub const PRELUDE: &[PreludeItem] = &[
    // reflection
    PreludeItem::new("Type", "reflection/type", "Type"),
    PreludeItem::new("typeOf", "reflection/type", "typeOf"),
    PreludeItem::new("deprecated", "reflection/decorator", "deprecated"),
    PreludeItem::new("intrinsic", "reflection/decorator", "intrinsic"),
    PreludeItem::new("inline", "reflection/decorator", "inline"),
    PreludeItem::new("unroll", "reflection/decorator", "unroll"),

    // operator
    // arithmetic
    PreludeItem::new("Add", "operator/arithmetic", "Add"),
    PreludeItem::new("Subtract", "operator/arithmetic", "Subtract"),
    PreludeItem::new("Multiply", "operator/arithmetic", "Multiply"),
    PreludeItem::new("Divide", "operator/arithmetic", "Divide"),
    PreludeItem::new("Remainder", "operator/arithmetic", "Remainder"),
    PreludeItem::new("Power", "operator/arithmetic", "Power"),
    PreludeItem::new("Negate", "operator/arithmetic", "Negate"),
    PreludeItem::new("And", "operator/arithmetic", "And"),
    PreludeItem::new("Or", "operator/arithmetic", "Or"),
    PreludeItem::new("Xor", "operator/arithmetic", "Xor"),
    PreludeItem::new("Not", "operator/arithmetic", "Not"),
    PreludeItem::new("ShiftLeft", "operator/arithmetic", "ShiftLeft"),
    PreludeItem::new("ShiftRight", "operator/arithmetic", "ShiftRight"),
    PreludeItem::new("ShiftRightUnsigned", "operator/arithmetic", "ShiftRightUnsigned"),
    PreludeItem::new("Concatenate", "operator/arithmetic", "Concatenate"),
    // comparison
    PreludeItem::new("Equal", "operator/comparison", "Equal"),
    PreludeItem::new("Compare", "operator/comparison", "Compare"),
    PreludeItem::new("Ordering", "operator/comparison", "Ordering"),
    // subscript
    PreludeItem::new("Index", "operator/subscript", "Index"),
    PreludeItem::new("IndexSet", "operator/subscript", "IndexSet"),
    PreludeItem::new("Deref", "operator/subscript", "Deref"),
    PreludeItem::new("DerefSet", "operator/subscript", "DerefSet"),
    // control
    PreludeItem::new("Try", "operator/control", "Try"),
    PreludeItem::new("ControlFlow", "operator/control", "ControlFlow"),
    PreludeItem::new("Bound", "operator/control", "Bound"),
    PreludeItem::new("RangeBounds", "operator/control", "RangeBounds"),
    PreludeItem::new("Range", "operator/control", "Range"),
    PreludeItem::new("RangeInclusive", "operator/control", "RangeInclusive"),
    PreludeItem::new("RangeFrom", "operator/control", "RangeFrom"),
    PreludeItem::new("RangeTo", "operator/control", "RangeTo"),
    PreludeItem::new("RangeToInclusive", "operator/control", "RangeToInclusive"),
    PreludeItem::new("RangeFull", "operator/control", "RangeFull"),
];

/// Check if a name is in the prelude.
pub fn is_prelude_name(name: &str) -> bool {
    PRELUDE.iter().any(|item| item.name == name)
}

/// Find a prelude item by name.
pub fn find_prelude_item(name: &str) -> Option<&'static PreludeItem> {
    PRELUDE.iter().find(|item| item.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_not_empty() {
        assert!(!PRELUDE.is_empty());
    }

    #[test]
    fn test_is_prelude_name() {
        assert!(is_prelude_name("Add"));
        assert!(is_prelude_name("Type"));
        assert!(!is_prelude_name("NotInPrelude"));
    }

    #[test]
    fn test_find_prelude_item() {
        let item = find_prelude_item("Add").unwrap();
        assert_eq!(item.name, "Add");
        assert_eq!(item.module, "operator/arithmetic");
        assert_eq!(item.export, "Add");
    }
}
