use crate::{
    TsppFormatOptions, assert_format_program, assert_format_program_roundtrip_with_file_type,
};
use tspp_source::FileType;

/// Binary ternary conditions should stay beside their assignment when they fit.
#[test]
fn test_format_binary_ternary_condition_stays_with_assignment() {
    assert_format_program!(
        r#"let opposed = from.x.abs() > from.z.abs()
    ? Vector3 { x: -from.y, y: from.x, z: T.zero() }
    : Vector3 { x: T.zero(), y: -from.z, z: from.y }
"#,
        r#"let opposed = from.x.abs() > from.z.abs()
    ? Vector3 { x: -from.y, y: from.x, z: T.zero() }
    : Vector3 { x: T.zero(), y: -from.z, z: from.y };
"#,
        FileType::Tspp,
    );
}

/// Ternary branch separator comments should stay on the consequent line.
#[test]
fn test_format_ternary_branch_separator_comments() {
    assert_format_program_roundtrip_with_file_type(
        r#"const value = cond ? left /* left-note */ : right /* right-note */
"#,
        r#"const value = cond
    ? left /* left-note */
    : right; /* right-note */
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(40),
    );
}

/// Ternary line comments after `:` should stay with the alternate branch.
#[test]
fn test_format_ternary_alternate_line_comments() {
    assert_format_program_roundtrip_with_file_type(
        r#"const value = cond ? left : // alt-line
right
"#,
        r#"const value = cond
    ? left
    : // alt-line
      right;
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(30),
    );
}

/// Separator comments around `new` branches should stay inside the consequent branch.
#[test]
fn test_format_ternary_new_branch_separator_comments() {
    assert_format_program_roundtrip_with_file_type(
        r#"const value = cond ? new Left() /* left-new */ : new Right()
"#,
        r#"const value = cond
    ? new Left() /* left-new */
    : new Right();
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(36),
    );
}

/// tree-chain block comments after `null` branches should stay before `:`.
#[test]
fn test_format_tree_chain_null_branch_separator_block_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"const value = <>{condition ? null /* branch-note */ : other ? <A /> : <B />}</>
"#,
        r#"const value = <>{condition ? null /* branch-note */ : other ? <A /> : <B />}</>;
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(100),
    );
}

/// tree branch block comments before `:` should stay with the consequent branch.
#[test]
fn test_format_tree_chain_branch_separator_block_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"const node = <div>{isVideo ? <Video /> /* video-comment */ : <Image /> /* image-comment */}</div>
"#,
        r#"const node = <div>{isVideo ? <Video /> /* video-comment */ : <Image /> /* image-comment */}</div>;
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(100),
    );
}

/// Wrapped tree branch block comments should stay inside their branch groups.
#[test]
fn test_format_tree_chain_wrapped_branch_separator_comments() {
    assert_format_program_roundtrip_with_file_type(
        r#"const node = <div>{isVideo ? <Video /> /* keep-video */ : <Image /> /* keep-image */}</div>
"#,
        r#"const node = (
    <div>
        {isVideo ? (
            <Video /> /* keep-video */
        ) : (
            <Image /> /* keep-image */
        )}
    </div>
);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(40),
    );
}

/// Tree line comments after `:` should stay with the alternate branch.
#[test]
fn test_format_tree_chain_alternate_line_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"const node = <>{x ? <A /> : // alt-line
<B />}</>
"#,
        r#"const node = (
    <>
        {x ? (
            <A />
        ) : (
            // alt-line
            <B />
        )}
    </>
);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(40),
    );
}
