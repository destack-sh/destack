use destack_dir::{Symbol, SymbolBinding, SymbolKind, SymbolType};
use destack_source::LanguageType;

/// Summary of a declaration for merge decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SymbolDescriptor {
    /// The declared symbol type.
    pub(crate) symbol_type: SymbolType,
    /// How the symbol was introduced.
    pub(crate) binding: SymbolBinding,
    /// The scope kind for the symbol.
    pub(crate) kind: SymbolKind,
}

impl From<&Symbol> for SymbolDescriptor {
    /// Create a symbol descriptor from a symbol.
    fn from(symbol: &Symbol) -> Self {
        Self {
            symbol_type: symbol.ty,
            binding: symbol.binding,
            kind: symbol.kind,
        }
    }
}

/// Check whether two declarations can merge.
pub(crate) fn can_merge_declarations(
    language_type: LanguageType,
    left: SymbolDescriptor,
    right: SymbolDescriptor,
) -> bool {
    // namespace merges
    if left.kind == SymbolKind::Namespace || right.kind == SymbolKind::Namespace {
        let (namespace, other, namespace_is_left) = if left.kind == SymbolKind::Namespace {
            (left, right, true)
        } else {
            (right, left, false)
        };

        // namespace declarations always merge with each other
        if other.kind == SymbolKind::Namespace {
            return true;
        }

        // class and function declarations merge with ambient namespaces in any order
        if matches!(other.symbol_type, SymbolType::Class | SymbolType::Function) {
            if !(language_type.supports_declaration_merging()
                || language_type.is_destack() && other.symbol_type == SymbolType::Function)
            {
                return false;
            }

            if namespace.binding == SymbolBinding::Ambient
                || other.binding == SymbolBinding::Ambient
            {
                return true;
            }

            // runtime namespace declarations must follow class and function declarations
            return !namespace_is_left;
        }

        // enum merges are order independent
        if other.symbol_type == SymbolType::Enum {
            return true;
        }

        // namespace and var style value declarations merge only for ambient namespaces
        if other.symbol_type == SymbolType::Void {
            return namespace.binding == SymbolBinding::Ambient;
        }

        return false;
    }

    // allow function overloads in destack modules
    if !language_type.supports_declaration_merging() {
        let is_function_overload =
            left.symbol_type == SymbolType::Function && right.symbol_type == SymbolType::Function;
        return language_type.is_destack() && is_function_overload;
    }

    // reject type aliases in merge candidates
    if left.symbol_type == SymbolType::TypeAlias || right.symbol_type == SymbolType::TypeAlias {
        return false;
    }

    // allow ambient function signatures to merge with runtime implementations
    if left.symbol_type == SymbolType::Function
        && right.symbol_type == SymbolType::Function
        && (left.binding == SymbolBinding::Ambient || right.binding == SymbolBinding::Ambient)
    {
        return true;
    }

    // allow function overloads
    if left.symbol_type == SymbolType::Function && right.symbol_type == SymbolType::Function {
        return true;
    }

    // allow enum redeclarations
    if left.symbol_type == SymbolType::Enum && right.symbol_type == SymbolType::Enum {
        return true;
    }

    // allow interface redeclarations
    if left.symbol_type == SymbolType::Interface && right.symbol_type == SymbolType::Interface {
        return true;
    }

    // allow interface merges with class or function declarations
    if (left.symbol_type == SymbolType::Interface && right.symbol_type == SymbolType::Class)
        || (left.symbol_type == SymbolType::Class && right.symbol_type == SymbolType::Interface)
        || (left.symbol_type == SymbolType::Interface && right.symbol_type == SymbolType::Function)
        || (left.symbol_type == SymbolType::Function && right.symbol_type == SymbolType::Interface)
    {
        return true;
    }

    false
}
