use crate::{TsppFormatOptions, assert_format, assert_format_roundtrip, parse_first_expression};
use tspp_source::FileType;

/// Simple imports should stay stable.
#[test]
fn test_format_import() {
    assert_format!(
        r#"import "foo""#,
        r#"import "foo""#,
        parse_first_expression,
        TsppFormatOptions::default()
    );
}

/// Named import lists should normalize spacing.
#[test]
fn test_format_import_with_items_from() {
    assert_format!(
        r#"import {foo,bar,baz} from "foo""#,
        r#"import { bar, baz, foo } from "foo""#,
        parse_first_expression,
        TsppFormatOptions::default_with_line_width(60)
    );
}

/// Export attributes should stay stable.
#[test]
fn test_format_export_with_attributes() {
    assert_format!(
        r#"export { foo } from "bar" with { mode: "strict" }"#,
        r#"export { foo } from "bar" with { mode: "strict" }"#,
        parse_first_expression,
        TsppFormatOptions::default()
    );
}

/// Default-plus-namespace imports should roundtrip cleanly.
#[test]
fn test_format_import_default_and_namespace_roundtrip() {
    assert_format_roundtrip!(
        r#"import a, * as b from "a""#,
        r#"import a, * as b from "a""#,
        FileType::Tspp,
        parse_first_expression,
    );
}
