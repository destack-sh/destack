use crate::{
    DestackFormatOptions, assert_format, assert_format_program_idempotent,
    assert_format_program_reference_widths,
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

/// Method comment seams should stay attached to the final method shell.
#[test]
fn test_format_method_comment_seams() {
    assert_format_program_reference_widths(
        r#"class A {
  m1(element: Element, key: string, undefined: undefined) /* block comment */ {
    // method body
  }
  m2(element: Element, key: string, undefined: undefined): void /* block comment */ {
    // method body
  }
  m3(tagName: string, rect: number[]): void // line comment
  {
    // method body
  }
  m4(tagName: string, rect: number[]) // line comment
  {
    // method body
  }
}
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"class A {
  m1(element: Element, key: string, undefined: undefined) /* block comment */ {
    // method body
  }
  m2(
    element: Element,
    key: string,
    undefined: undefined,
  ): void /* block comment */ {
    // method body
  }
  m3(tagName: string, rect: number[]): void {
    // line comment
    // method body
  }
  m4(tagName: string, rect: number[]) {
    // line comment
    // method body
  }
}
"#,
            ),
            (
                100,
                r#"class A {
  m1(element: Element, key: string, undefined: undefined) /* block comment */ {
    // method body
  }
  m2(element: Element, key: string, undefined: undefined): void /* block comment */ {
    // method body
  }
  m3(tagName: string, rect: number[]): void {
    // line comment
    // method body
  }
  m4(tagName: string, rect: number[]) {
    // line comment
    // method body
  }
}
"#,
            ),
        ],
    );
}
