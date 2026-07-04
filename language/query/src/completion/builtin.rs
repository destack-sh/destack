use std::collections::HashSet;

use destack_dir as dir;

use super::{Completion, CompletionKind, SORT_BUILTIN, SORT_KEYWORD};

/// Primitive type completions.
pub(super) fn primitive_type_completions() -> Vec<Completion> {
    let mut names = vec![
        "any".to_string(),
        "unknown".to_string(),
        "never".to_string(),
        "void".to_string(),
        "null".to_string(),
        "undefined".to_string(),
        "object".to_string(),
        "boolean".to_string(),
        "character".to_string(),
        "string".to_string(),
        "bigint".to_string(),
        "number".to_string(),
        "symbol".to_string(),
        "unique symbol".to_string(),
        "int".to_string(),
        "uint".to_string(),
        "float".to_string(),
    ];

    for integer_type in integer_types() {
        names.push(integer_type.as_str());
    }

    for float_type in [dir::FloatType::Float32, dir::FloatType::Float64] {
        names.push(float_type.as_str().to_string());
    }

    let mut seen = HashSet::new();
    let mut completions = Vec::new();
    for name in names {
        if !seen.insert(name.clone()) {
            continue;
        }

        completions.push(
            Completion::new(&name, CompletionKind::TypeParameter)
                .with_sort_order(SORT_BUILTIN)
                .with_sort_text(length_sort_text(&name))
                .as_builtin(),
        );
    }

    completions
}

/// Keyword completions.
pub(super) fn keyword_completions() -> Vec<Completion> {
    let mut seen = HashSet::new();
    let mut completions = Vec::new();

    for keyword in keywords() {
        let label = keyword.as_str();
        if !seen.insert(label) {
            continue;
        }

        let mut completion = Completion::new(label, CompletionKind::Keyword)
            .with_sort_order(SORT_KEYWORD)
            .with_sort_text(length_sort_text(label))
            .as_keyword();
        if let Some(snippet) = keyword_snippet(keyword) {
            completion = completion.with_insert_text(snippet).as_snippet();
        }

        completions.push(completion);
    }

    for literal in ["true", "false"] {
        if !seen.insert(literal) {
            continue;
        }

        completions.push(
            Completion::new(literal, CompletionKind::Keyword)
                .with_sort_order(SORT_KEYWORD)
                .with_sort_text(length_sort_text(literal))
                .as_keyword(),
        );
    }

    completions
}

/// Build one stable length-aware sort text.
pub(super) fn length_sort_text(label: &str) -> String {
    format!("{:02}:{}", label.chars().count(), label.to_lowercase())
}

/// Return primitive integer types shown as builtins.
fn integer_types() -> [dir::IntegerType; 14] {
    [
        dir::IntegerType::Fixed {
            width: 8,
            is_signed: true,
        },
        dir::IntegerType::Fixed {
            width: 16,
            is_signed: true,
        },
        dir::IntegerType::Fixed {
            width: 32,
            is_signed: true,
        },
        dir::IntegerType::Fixed {
            width: 64,
            is_signed: true,
        },
        dir::IntegerType::Fixed {
            width: 128,
            is_signed: true,
        },
        dir::IntegerType::Fixed {
            width: 256,
            is_signed: true,
        },
        dir::IntegerType::Pointer { is_signed: true },
        dir::IntegerType::Fixed {
            width: 8,
            is_signed: false,
        },
        dir::IntegerType::Fixed {
            width: 16,
            is_signed: false,
        },
        dir::IntegerType::Fixed {
            width: 32,
            is_signed: false,
        },
        dir::IntegerType::Fixed {
            width: 64,
            is_signed: false,
        },
        dir::IntegerType::Fixed {
            width: 128,
            is_signed: false,
        },
        dir::IntegerType::Fixed {
            width: 256,
            is_signed: false,
        },
        dir::IntegerType::Pointer { is_signed: false },
    ]
}

/// Return keywords shown in statement completion.
fn keywords() -> [dir::Keyword; 76] {
    [
        dir::Keyword::Public,
        dir::Keyword::Protected,
        dir::Keyword::Private,
        dir::Keyword::Readonly,
        dir::Keyword::Exclusive,
        dir::Keyword::Local,
        dir::Keyword::Shared,
        dir::Keyword::Static,
        dir::Keyword::Final,
        dir::Keyword::Virtual,
        dir::Keyword::Accessor,
        dir::Keyword::Default,
        dir::Keyword::This,
        dir::Keyword::Super,
        dir::Keyword::Package,
        dir::Keyword::Import,
        dir::Keyword::Export,
        dir::Keyword::From,
        dir::Keyword::Const,
        dir::Keyword::Let,
        dir::Keyword::Type,
        dir::Keyword::Newtype,
        dir::Keyword::Struct,
        dir::Keyword::Class,
        dir::Keyword::Enum,
        dir::Keyword::Interface,
        dir::Keyword::Function,
        dir::Keyword::Extension,
        dir::Keyword::Declare,
        dir::Keyword::New,
        dir::Keyword::Constructor,
        dir::Keyword::Extends,
        dir::Keyword::Implements,
        dir::Keyword::Satisfies,
        dir::Keyword::Abstract,
        dir::Keyword::Override,
        dir::Keyword::InstanceOf,
        dir::Keyword::Where,
        dir::Keyword::Typeof,
        dir::Keyword::Void,
        dir::Keyword::Null,
        dir::Keyword::Undefined,
        dir::Keyword::Keyof,
        dir::Keyword::Infer,
        dir::Keyword::Any,
        dir::Keyword::Never,
        dir::Keyword::As,
        dir::Keyword::Is,
        dir::Keyword::In,
        dir::Keyword::Of,
        dir::Keyword::Using,
        dir::Keyword::Comptime,
        dir::Keyword::If,
        dir::Keyword::Else,
        dir::Keyword::Match,
        dir::Keyword::Switch,
        dir::Keyword::Case,
        dir::Keyword::Do,
        dir::Keyword::While,
        dir::Keyword::For,
        dir::Keyword::Loop,
        dir::Keyword::Break,
        dir::Keyword::Continue,
        dir::Keyword::Debugger,
        dir::Keyword::Return,
        dir::Keyword::Yield,
        dir::Keyword::Try,
        dir::Keyword::Catch,
        dir::Keyword::Throw,
        dir::Keyword::Finally,
        dir::Keyword::Async,
        dir::Keyword::Await,
        dir::Keyword::Get,
        dir::Keyword::Set,
        dir::Keyword::Move,
        dir::Keyword::With,
    ]
}

/// Resolve a snippet template for control-flow keywords.
fn keyword_snippet(keyword: dir::Keyword) -> Option<&'static str> {
    match keyword {
        dir::Keyword::If => Some("if (${1:condition}) {\n    $0\n}"),
        dir::Keyword::For => Some("for (${1:item} in ${2:items}) {\n    $0\n}"),
        dir::Keyword::While => Some("while (${1:condition}) {\n    $0\n}"),
        dir::Keyword::Switch => {
            Some("switch (${1:value}) {\n    case ${2:pattern}:\n        $0\n    default:\n}")
        }
        dir::Keyword::Try => Some("try {\n    $1\n} catch (${2:error}) {\n    $0\n}"),
        _ => None,
    }
}
