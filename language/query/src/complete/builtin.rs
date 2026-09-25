use rustc_hash::FxHashSet;
use tspp_dir as dir;

use crate::{CompletionCandidate, CompletionItemKind, CompletionOrigin};

/// Primitive type completions.
pub(super) fn primitive_type_completions() -> Vec<CompletionCandidate> {
    let mut names = vec![
        "unknown".to_string(),
        "never".to_string(),
        "void".to_string(),
        "null".to_string(),
        "undefined".to_string(),
        "boolean".to_string(),
        "char".to_string(),
        "string".to_string(),
        "bigint".to_string(),
        "number".to_string(),
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

    let mut seen = FxHashSet::default();
    let mut completions = Vec::new();
    for name in names {
        if !seen.insert(name.clone()) {
            continue;
        }

        completions.push(
            CompletionCandidate::new(
                &name,
                CompletionItemKind::BuiltinType,
                CompletionOrigin::Builtin,
            )
            .with_ordering_text(length_ordering_text(&name)),
        );
    }

    completions
}

/// Keyword completions.
pub(super) fn keyword_completions() -> Vec<CompletionCandidate> {
    let mut seen = FxHashSet::default();
    let mut completions = Vec::new();

    for keyword in keywords() {
        let label = keyword.as_str();
        if !seen.insert(label) {
            continue;
        }

        let mut completion = CompletionCandidate::new(
            label,
            CompletionItemKind::Keyword,
            CompletionOrigin::Keyword,
        )
        .with_ordering_text(length_ordering_text(label));
        if let Some(snippet) = keyword_snippet(keyword) {
            completion = completion.with_snippet(snippet);
        }

        completions.push(completion);
    }

    for literal in ["true", "false"] {
        if !seen.insert(literal) {
            continue;
        }

        completions.push(
            CompletionCandidate::new(
                literal,
                CompletionItemKind::Keyword,
                CompletionOrigin::Keyword,
            )
            .with_ordering_text(length_ordering_text(literal)),
        );
    }

    completions
}

/// Build stable length-aware ordering text.
pub(super) fn length_ordering_text(label: &str) -> String {
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

/// Return keywords that can begin a declaration or statement.
fn keywords() -> [dir::Keyword; 39] {
    [
        dir::Keyword::Import,
        dir::Keyword::Export,
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
        dir::Keyword::Using,
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
        dir::Keyword::Finally,
        dir::Keyword::Async,
        dir::Keyword::Await,
        dir::Keyword::With,
        dir::Keyword::New,
        dir::Keyword::This,
        dir::Keyword::Super,
        dir::Keyword::Null,
        dir::Keyword::Undefined,
    ]
}

/// Resolve a snippet template for control-flow keywords.
fn keyword_snippet(keyword: dir::Keyword) -> Option<&'static str> {
    match keyword {
        dir::Keyword::If => Some("if (${1:condition}) {\n    $0\n}"),
        dir::Keyword::For => Some("for (${1:item} of ${2:items}) {\n    $0\n}"),
        dir::Keyword::While => Some("while (${1:condition}) {\n    $0\n}"),
        dir::Keyword::Switch => {
            Some("switch (${1:value}) {\n    case ${2:pattern}:\n        $0\n    default:\n}")
        }
        dir::Keyword::Try => Some("try {\n    $1\n} catch (${2:error}) {\n    $0\n}"),
        _ => None,
    }
}
