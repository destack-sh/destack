use crate::assert_format_program_idempotent;
use destack_source::FileType;

/// Type alias comments after `=` should stay stable across passes.
#[test]
fn test_format_typescript_union_head_comment_after_equals_is_idempotent() {
    assert_format_program_idempotent!(
        r#"type Aa1 = /*1*/ | /*2*/ C | D;
"#,
        FileType::TypeScript
    );
}
