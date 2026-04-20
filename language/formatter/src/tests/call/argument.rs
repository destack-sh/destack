use crate::{DestackFormatOptions, assert_format, assert_format_program};
use destack_source::FileType;

/// Simple named tree arguments should stay stable.
#[test]
fn test_format_argument_named() {
    assert_format!(
        r#"x: 1"#,
        r#"x: 1"#,
        |p| p.eat_tree_argument(),
        DestackFormatOptions::default()
    );
}

/// Multiline JSX arguments should force expanded multi-argument call layout.
#[test]
fn test_format_multiline_jsx_argument_forces_expanded_call_layout() {
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
        FileType::JavaScriptXml,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Generic tree tags should keep their type arguments in the opening tag.
#[test]
fn test_format_tree_argument_preserves_generic_tag_arguments() {
    assert_format_program!(
        r#"const view = fn(<Foo<Bar> />)
"#,
        r#"const view = fn(<Foo<Bar> />);
"#,
        FileType::TypeScriptXml,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}
