use crate::{
    DestackFormatOptions, TestFormatter, assert_format,
    assert_format_program_idempotent_with_file_type,
};
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

#[test]
fn test_format_parameter_comment_between_name_and_type() {
    assert_format!(
        "x /* a */ : number",
        "x /* a */ : number",
        |p| p.eat_parameter(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_optional_parameter_comment_between_name_and_type() {
    assert_format!(
        "x? /* a */ : number",
        "x? /* a */ : number",
        |p| p.eat_parameter(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_parameter_comment_before_name() {
    assert_format!(
        "/* a */ x: number",
        "/* a */ x: number",
        |p| p.eat_parameter(),
        DestackFormatOptions::default()
    );
}

/// Trailing separator line comments in parameter lists should stay idempotent.
#[test]
fn test_format_signature_trailing_separator_line_comment_is_idempotent() {
    let source = "f2 = (
  currentRequest: {a: number},
  // TODO this is a very very very very long comment that makes it go > 80 columns
): number => {};
";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Own-line block separator comments in parameter lists should stay idempotent.
#[test]
fn test_format_signature_trailing_separator_block_comment_is_idempotent() {
    let source = r#"var x = {
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
"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}
