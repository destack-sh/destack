use crate::{DestackFormatOptions, assert_format, assert_format_roundtrip};
use destack_source::FileType;

/// Simple imports should stay stable.
#[test]
fn test_format_import() {
    assert_format!(
        r#"import "foo""#,
        r#"import "foo""#,
        crate::parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Named import lists should normalize spacing.
#[test]
fn test_format_import_with_items_from() {
    assert_format!(
        r#"import {foo,bar,baz} from "foo""#,
        r#"import { bar, baz, foo } from "foo""#,
        crate::parse_first_expression,
        DestackFormatOptions::default_with_line_width(60)
    );
}

/// Type namespace imports should keep their explicit form.
#[test]
fn test_format_import_type_namespace() {
    assert_format!(
        r#"import type * as React from "react""#,
        r#"import type * as React from "react""#,
        crate::parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Export attributes should stay stable.
#[test]
fn test_format_export_with_attributes() {
    assert_format!(
        r#"export { foo } from "bar" with { mode: "strict" }"#,
        r#"export { foo } from "bar" with { mode: "strict" }"#,
        crate::parse_first_expression,
        DestackFormatOptions::default()
    );
}

/// Default-plus-namespace imports should roundtrip cleanly.
#[test]
fn test_format_import_default_and_namespace_roundtrip() {
    assert_format_roundtrip!(
        r#"import a, * as b from "a""#,
        r#"import a, * as b from "a""#,
        FileType::JavaScript,
        crate::parse_first_expression,
    );
}
