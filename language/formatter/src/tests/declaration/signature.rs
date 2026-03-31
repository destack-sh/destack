use crate::{DestackFormatOptions, assert_format, assert_format_program_idempotent};
use destack_source::FileType;

#[test]
fn test_format_parameter() {
    assert_format!(
        "x: int32",
        "x: int32",
        |p| p.eat_parameter(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_parameter_with_default() {
    assert_format!(
        "x: int32 = 1",
        "x: int32 = 1",
        |p| p.eat_parameter(),
        DestackFormatOptions::default()
    );
}

/// Trailing separator line comments in parameter lists should stay idempotent.
#[test]
fn test_format_signature_trailing_separator_line_comment_is_idempotent() {
    assert_format_program_idempotent!(
        r#"f2 = (
  currentRequest: {a: number},
  // this deliberately long comment keeps the separator seam in the broken layout
): number => {};
"#,
        FileType::TypeScript
    );
}

/// Own-line block separator comments in parameter lists should stay idempotent.
#[test]
fn test_format_signature_trailing_separator_block_comment_is_idempotent() {
    assert_format_program_idempotent!(
        r#"var x = {
  getSectionMode(
    pageMetaData: PageMetaData,
    sectionMetaData: SectionMetaData
    /* $FlowFixMe This error was exposed while converting keyMirror
     * to keyMirrorRecursive */
  ): $Enum<SectionMode> {
  }
}

class X2 {
  getSectionMode(
    pageMetaData: PageMetaData,
    sectionMetaData: SectionMetaData = ['unknown']
    /* $FlowFixMe This error was exposed while converting keyMirror
     * to keyMirrorRecursive */
  ): $Enum<SectionMode> {
  }
}
"#,
        FileType::TypeScript
    );
}
