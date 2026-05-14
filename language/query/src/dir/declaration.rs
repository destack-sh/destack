use destack_core::StringPool;
use destack_dir as dir;
use destack_dir::{Declaration, Name};

/// Resolve the declared name when one exists.
pub(crate) fn declaration_name(declaration: &Declaration) -> Option<Name> {
    match declaration {
        Declaration::Global(_) => None,
        Declaration::Module(_) => None,
        Declaration::Namespace(declaration) => Some(declaration.name),
        Declaration::Type(declaration) => Some(declaration.name),
        Declaration::Struct(declaration) => Some(declaration.name),
        Declaration::Class(declaration) => declaration.name,
        Declaration::Enum(declaration) => declaration.name,
        Declaration::Interface(declaration) => declaration.name,
        Declaration::Extension(declaration) => declaration.name,
        Declaration::Function(declaration) => declaration.name,
    }
}

/// Resolve the export kind when one exists.
pub(crate) fn declaration_export(declaration: &Declaration) -> Option<dir::ExportKind> {
    match declaration {
        Declaration::Global(_) => None,
        Declaration::Module(_) => None,
        Declaration::Namespace(declaration) => declaration.export,
        Declaration::Type(declaration) => declaration.export,
        Declaration::Struct(declaration) => declaration.export,
        Declaration::Class(declaration) => declaration.export,
        Declaration::Enum(declaration) => declaration.export,
        Declaration::Interface(declaration) => declaration.export,
        Declaration::Extension(declaration) => declaration.export,
        Declaration::Function(declaration) => declaration.export,
    }
}

/// Return whether the declaration is ambient.
pub(crate) fn declaration_is_ambient(declaration: &Declaration) -> bool {
    match declaration {
        Declaration::Global(declaration) => declaration.is_ambient,
        Declaration::Module(_) => false,
        Declaration::Namespace(declaration) => declaration.is_ambient,
        Declaration::Type(declaration) => declaration.is_ambient,
        Declaration::Struct(declaration) => declaration.is_ambient,
        Declaration::Class(declaration) => declaration.is_ambient,
        Declaration::Enum(declaration) => declaration.is_ambient,
        Declaration::Interface(declaration) => declaration.is_ambient,
        Declaration::Extension(declaration) => declaration.is_ambient,
        Declaration::Function(declaration) => declaration.is_ambient,
    }
}

/// Return whether the declaration is abstract.
pub(crate) fn declaration_is_abstract(declaration: &Declaration) -> bool {
    match declaration {
        Declaration::Class(declaration) => declaration.is_abstract,
        Declaration::Function(declaration) => declaration.signature.is_abstract,
        _ => false,
    }
}

/// Resolve a display name for a declaration.
pub(crate) fn declaration_display_name(strings: &StringPool, declaration: &Declaration) -> String {
    // default block declarations to keyword labels
    if matches!(declaration, Declaration::Global(_)) {
        return "global".to_string();
    }
    if matches!(declaration, Declaration::Module(_)) {
        return "module".to_string();
    }

    // prefer the explicit declaration name
    declaration_name(declaration)
        .map(|name| strings.get(name.string()).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string())
}
