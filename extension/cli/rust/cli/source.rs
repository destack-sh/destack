use std::fs;
use std::path::Path;

use dyst_ast::{BlockFormat, NodeVisitor, Parser, SemanticTokenIndex, SemanticType};
use dyst_session::Session;
use dyst_source::{Source, SourceFormat, SourceId, Uri};
use dyst_token::{TokenSpan, TokenType};

use crate::console::console;
use crate::console::parse::CommandArguments;

/// Read a source either from a file or inline string argument.
pub(crate) fn read_source(ctx: &CommandArguments) -> Result<Source, String> {
    if let Some(path) = ctx.option("file") {
        let path_ref = Path::new(path);
        fs::read_to_string(path_ref)
            .map_err(|error| format!("failed to read {path}: {error}"))
            .map(|content| {
                Source::from_string(
                    SourceId::new(0),
                    Uri::from_string(path),
                    SourceFormat::Dyst,
                    content,
                )
            })
    } else if let Some(string) = ctx.option("string") {
        Ok(Source::from_string(
            SourceId::new(0),
            Uri::from_string("<string>"),
            SourceFormat::Dyst,
            string.to_string(),
        ))
    } else {
        Err("provide --file <path> or --string <string>".to_string())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SemanticSpan {
    pub semantic_type: SemanticType,
    pub text: String,
}

/// Compute semantic spans for a given label and body.
pub(crate) fn semantic_spans_from_text(
    label: &str,
    text: &str,
) -> Result<Vec<SemanticSpan>, String> {
    let source = Source::from_string(
        SourceId::new(1),
        Uri::from_string(label),
        SourceFormat::Dyst,
        text.to_string(),
    );
    semantic_spans_from_source(&source)
}

/// Compute semantic spans directly from a Source value.
pub(crate) fn semantic_spans_from_source(source: &Source) -> Result<Vec<SemanticSpan>, String> {
    let mut session = Session::new();
    let mut parser = Parser::prepare(source, &mut session);
    let expressions = parser.with_recovery(
        parser.mark(),
        |parser| parser.eat_block_body(BlockFormat::Implicit),
        Vec::new(),
        TokenType::End,
    );
    parser.finalize();

    let mut all_tokens: Vec<TokenSpan> = parser.tokens.clone();
    all_tokens.extend_from_slice(&parser.side_tokens);
    all_tokens.sort_by(|lhs, rhs| {
        lhs.span
            .start
            .cmp(&rhs.span.start)
            .then_with(|| lhs.span.end.cmp(&rhs.span.end))
    });

    let mut index = SemanticTokenIndex::from_tokens(source, &all_tokens);
    for expression in expressions {
        index.visit_expression(&parser.tree, expression, parser.tree.get(expression));
    }

    let semantic_types = index.semantic_types;
    let mut spans = Vec::with_capacity(all_tokens.len());
    for (token, semantic_type) in all_tokens.iter().zip(semantic_types.into_iter()) {
        if token.token.r#type == TokenType::End {
            continue;
        }
        let slice = &source.content[token.span.start as usize..token.span.end as usize];
        spans.push(SemanticSpan {
            semantic_type,
            text: slice.to_string(),
        });
    }

    Ok(spans)
}

/// Render semantic spans with ANSI coloring.
pub(crate) fn render_semantic_spans(spans: &[SemanticSpan]) -> String {
    let mut out = String::new();
    for span in spans {
        match color_for_semantic(span.semantic_type) {
            Some(code) => out.push_str(&console::color(&span.text, code)),
            None => out.push_str(&span.text),
        }
    }
    out
}

fn color_for_semantic(semantic_type: SemanticType) -> Option<&'static str> {
    match semantic_type {
        SemanticType::Whitespace => None,
        SemanticType::Identifier => None,
        SemanticType::Keyword => Some("94"),
        SemanticType::LiteralNumbery => Some("93"),
        SemanticType::LiteralStringy => Some("92"),
        SemanticType::Parenthesis | SemanticType::Symbol | SemanticType::Operator => Some("96"),
        SemanticType::Doc => Some("32"),
        SemanticType::Comment => Some("2"),
        SemanticType::Modifier => Some("95"),
        SemanticType::Macro => Some("95"),
        SemanticType::Type => Some("36"),
        SemanticType::Function => Some("33"),
        SemanticType::Parameter | SemanticType::Argument => Some("91"),
        SemanticType::Variable => Some("37"),
    }
}
