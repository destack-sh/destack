use destack_dir as dir;

use crate::check::{Dump, DumpContext, TypeLiteralTerm};

use super::format::dump_list;

impl Dump for TypeLiteralTerm {
    /// Render one type literal term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Error => "error".to_string(),
            Self::Never => "never".to_string(),
            Self::Any => "any".to_string(),
            Self::Unknown => "unknown".to_string(),
            Self::Void => "void".to_string(),
            Self::Null => "null".to_string(),
            Self::Undefined => "undefined".to_string(),
            Self::Object => "object".to_string(),
            Self::Primitive(primitive) => primitive.dump(context),
            Self::Scalar(literal) => literal.dump(context),
        }
    }
}

impl Dump for dir::ScalarLiteral {
    /// Render one scalar literal.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Undefined => "undefined".to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::Bigint(value) => format!("{value}n"),
            Self::Float(value) => value.to_string(),
            Self::Character(value) => value.to_string(),
            Self::String(value) => context.string(*value),
            Self::RegexString { content, flags } => {
                let content = context.string(*content);
                let flags = flags.map(|flags| context.string(flags)).unwrap_or_default();

                format!("/{content}/{flags}")
            }
        }
    }
}

impl Dump for dir::TypeLiteral {
    /// Render one DIR type literal.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Never => "never".to_string(),
            Self::Any => "any".to_string(),
            Self::Undefined => "undefined".to_string(),
            Self::Unknown => "unknown".to_string(),
            Self::Object => "object".to_string(),
            Self::Void => "void".to_string(),
            Self::Null => "null".to_string(),
            Self::Boolean => "boolean".to_string(),
            Self::Character => "char".to_string(),
            Self::String => "string".to_string(),
            Self::Bigint => "bigint".to_string(),
            Self::Number => "number".to_string(),
            Self::Integer(integer) => integer.dump(context),
            Self::Float(float) => float.dump(context),
            Self::Symbol => "symbol".to_string(),
            Self::UniqueSymbol => "unique symbol".to_string(),
        }
    }
}

impl Dump for dir::PrimitiveType {
    /// Render one primitive type.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Boolean => "boolean".to_string(),
            Self::Character => "char".to_string(),
            Self::String => "string".to_string(),
            Self::Bigint => "bigint".to_string(),
            Self::Integer(integer) => integer.dump(context),
            Self::Float(float) => float.dump(context),
            Self::Symbol => "symbol".to_string(),
            Self::UniqueSymbol => "unique symbol".to_string(),
        }
    }
}

impl Dump for dir::IntegerType {
    /// Render one integer type.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        self.as_str()
    }
}

impl Dump for dir::FloatType {
    /// Render one float type.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        self.as_str().to_string()
    }
}

impl Dump for dir::StringId {
    /// Render one interned string id.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        context.string(*self)
    }
}

/// Render one string id list.
pub(super) fn dump_strings(strings: &[dir::StringId], context: &DumpContext<'_, '_>) -> String {
    let strings = strings
        .iter()
        .map(|string| string.dump(context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(strings)
}
