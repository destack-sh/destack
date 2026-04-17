use crate::assert_format_program_reference_widths;
use destack_source::FileType;

/// Type assertions in default exports should preserve angle assertions.
#[test]
fn test_format_type_assertion_parentheses() {
    assert_format_program_reference_widths(
        r#"export default <Array>[];
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"export default <Array>[];
"#,
            ),
            (
                100,
                r#"export default <Array>[];
"#,
            ),
        ],
    );
}
