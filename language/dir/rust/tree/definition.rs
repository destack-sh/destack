/// Definition introduces a type or function into its scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    Struct,
    Enum,
    Union,
    Trait,
    Function,
    Implement,
    Let,
}

// nocheckin: Definitions, DIR, ...
