use crate::generate::core::upper_camel;

const KEYWORDS: &[&str] = &[
    "abstract",
    "any",
    "arguments",
    "as",
    "asserts",
    "async",
    "await",
    "bigint",
    "boolean",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "constructor",
    "continue",
    "debugger",
    "declare",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "get",
    "if",
    "implements",
    "import",
    "in",
    "infer",
    "instanceof",
    "interface",
    "is",
    "keyof",
    "let",
    "module",
    "namespace",
    "never",
    "new",
    "null",
    "number",
    "object",
    "of",
    "package",
    "private",
    "protected",
    "public",
    "readonly",
    "require",
    "return",
    "set",
    "static",
    "string",
    "super",
    "switch",
    "symbol",
    "this",
    "throw",
    "true",
    "try",
    "type",
    "typeof",
    "undefined",
    "unique",
    "unknown",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

/// Return one safe TypeScript binding identifier.
pub(super) fn identifier(name: &str) -> String {
    if name.chars().all(|character| character.is_ascii_digit()) {
        format!("field{name}")
    } else if !is_identifier(name) {
        format!("field{}", upper_camel(name))
    } else if KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

/// Return one safe TypeScript object member name.
pub(super) fn member(name: &str) -> String {
    if KEYWORDS.contains(&name) {
        format!("{name:?}")
    } else {
        name.to_string()
    }
}

/// Return whether one string is a TypeScript identifier.
pub(super) fn is_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    if !(first == '_' || first == '$' || first.is_ascii_alphabetic()) {
        return false;
    }

    characters
        .all(|character| character == '_' || character == '$' || character.is_ascii_alphanumeric())
}
