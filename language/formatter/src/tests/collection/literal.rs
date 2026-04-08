use crate::{DestackFormatOptions, assert_format, assert_format_roundtrip};
use destack_fir::format;
use destack_fir::format::{BestFittingVariants, Document, FormatNode, FormatTagKind};
use destack_source::FileType;

/// Validate one document's tag nesting recursively.
fn validate_document_tags(
    nodes: &[FormatNode],
    stack: &mut Vec<FormatTagKind>,
    path: &mut Vec<String>,
) -> Result<(), String> {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index.to_string());

        match node {
            // nested content
            FormatNode::Interned(interned) => {
                validate_document_tags(interned, stack, path)?;
            }
            FormatNode::BestFitting { variants, .. } => {
                validate_best_fitting_variants(variants, stack, path)?;
            }

            // tag pairs
            FormatNode::Tag(tag) if tag.is_start() => {
                stack.push(tag.kind());
            }
            FormatNode::Tag(tag) => {
                let end_kind = tag.kind();
                let Some(start_kind) = stack.pop() else {
                    return Err(std::format!(
                        "missing start tag for {end_kind:?} at {}",
                        path.join(".")
                    ));
                };

                if start_kind != end_kind {
                    return Err(std::format!(
                        "mismatch at {}: start={start_kind:?}, end={end_kind:?}",
                        path.join(".")
                    ));
                }
            }

            // leaf content
            _ => {}
        }

        path.pop();
    }

    Ok(())
}

/// Validate all best-fitting variants independently.
fn validate_best_fitting_variants(
    variants: &BestFittingVariants,
    stack: &mut Vec<FormatTagKind>,
    path: &mut Vec<String>,
) -> Result<(), String> {
    for (index, variant) in variants.into_iter().enumerate() {
        let snapshot = stack.clone();

        path.push(std::format!("variant-{index}"));
        validate_document_tags(variant, stack, path)?;

        if *stack != snapshot {
            return Err(std::format!(
                "best-fitting variant leaked stack at {}: before={snapshot:?}, after={stack:?}",
                path.join(".")
            ));
        }

        path.pop();
        *stack = snapshot;
    }

    Ok(())
}

/// Assert one document has balanced tag nesting.
fn assert_document_tags_are_balanced(document: &Document) {
    let mut stack = Vec::new();
    let mut path = Vec::new();

    validate_document_tags(document, &mut stack, &mut path).unwrap();

    assert!(stack.is_empty(), "unclosed tags at end: {stack:?}");
}

/// Multi-char strings should normalize to double quotes in semantic mode.
#[test]
fn test_format_string_literal_multi_char() {
    assert_format!(r#"'hello'"#, r#""hello""#, |p| p
        .eat_expression(Default::default()));
}

/// Embedded target quotes should stay escaped once.
#[test]
fn test_format_string_literal_escapes_embedded_target_quote() {
    assert_format!(r#"'\"1\"'"#, r#""\"1\"""#, |p| p
        .eat_expression(Default::default()));
}

/// Template literals with interpolation should stay stable.
#[test]
fn test_format_template_literal_one_interpolation() {
    assert_format!(
        r#"tagged`hello ${name}`"#,
        r#"tagged`hello ${name}`"#,
        |p| p.eat_expression(Default::default())
    );
}

/// Ternary interpolations in one-line templates should remain inline and idempotent.
#[test]
fn test_format_template_literal_ternary_interpolation_stays_inline_roundtrip() {
    assert_format_roundtrip!(
        r#"`"${isSSR ? "------------------------------------------------------------------------------" : false}" TEST`"#,
        r#"`"${isSSR ? "------------------------------------------------------------------------------" : false}" TEST`"#,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
    );
}

/// Multiline call interpolations in templates should build a valid document.
#[test]
fn test_format_template_literal_multiline_call_interpolation_document() {
    let input = r#"const foo = () => {
  {
    {
      {
        return `
line 1
line 2
...
line n
${foo({
  many: keys,
  many: keys
})}
line n + 1
line n + 2
line n + n
`;
      }
    }
  }
};"#;

    let (test, roots) =
        crate::TestFormatter::parse_with_file_type(input, FileType::JavaScript, |p| Ok(p.parse()))
            .unwrap();
    let options = DestackFormatOptions {
        language_type: FileType::JavaScript.into(),
        ..DestackFormatOptions::default()
    };
    let context = test.context(options);
    let formatted = format!(context, [crate::statement_list(&roots)]).unwrap();

    assert_document_tags_are_balanced(formatted.document());

    formatted.print().unwrap();
}

/// Long strings should not be forcibly broken.
#[test]
fn test_format_long_string_not_broken() {
    assert_format!(
        r#""This is a very long string that exceeds the line width but should not be broken""#,
        r#""This is a very long string that exceeds the line width but should not be broken""#,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(40)
    );
}

/// Paths with multiple segments should stay stable.
#[test]
fn test_format_path_multiple_segments() {
    assert_format!(
        r#"destack.geometry.math"#,
        r#"destack.geometry.math"#,
        |p| p.eat_path(),
        DestackFormatOptions::default()
    );
}
