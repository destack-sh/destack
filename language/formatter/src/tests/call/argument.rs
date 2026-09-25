use crate::{
    TsppFormatOptions, assert_format_program, assert_format_roundtrip, parse_first_expression,
};
use tspp_source::FileType;

/// Malformed call argument slots should preserve their authored source.
#[test]
fn test_format_recovered_call_argument() {
    assert_format_roundtrip!(
        "consume(1, , 3)",
        "consume(1, , 3)",
        FileType::Tspp,
        parse_first_expression,
    );
}

/// Call argument decorators should be preserved.
#[test]
fn test_format_decorated_call_argument() {
    assert_format_program!(
        r#"call(@if(true) value)
"#,
        r#"call(@if(true) value);
"#,
        FileType::Tspp,
    );
}

/// Multiline tree arguments should force expanded multi-argument call layout.
#[test]
fn test_format_multiline_tree_argument_forces_expanded_call_layout() {
    assert_format_program!(
        r#"const view = fn(bar, <div>
  <span />
</div>)
"#,
        r#"const view = fn(
  bar,
  <div>
    <span />
  </div>,
);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Generic tree tags should keep their type arguments in the opening tag.
#[test]
fn test_format_tree_preserves_generic_tag_arguments() {
    assert_format_program!(
        r#"const view = fn(<Foo<Bar> />)
"#,
        r#"const view = fn(<Foo<Bar> />);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}
