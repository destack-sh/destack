use crate::{DestackFormatOptions, assert_format_program_roundtrip_with_file_type};
use destack_source::FileType;

/// Ternary branch separator comments should stay on the consequent line.
#[test]
fn test_format_ternary_branch_separator_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = cond ? left /* left-note */ : right /* right-note */\n",
        "const value = cond\n    ? left /* left-note */\n    : right; /* right-note */\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(40),
    );
}

/// Ternary line comments before alternates should stay with the consequent line.
#[test]
fn test_format_ternary_alternate_line_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = cond ? left : // alt-line\nright\n",
        "const value = cond\n    ? left // alt-line\n    : right;\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(30),
    );
}

/// Separator comments around `new` branches should stay inside the consequent branch.
#[test]
fn test_format_ternary_new_branch_separator_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = cond ? new Left() /* left-new */ : new Right()\n",
        "const value = cond\n    ? new Left() /* left-new */\n    : new Right();\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(36),
    );
}

/// JSX-chain block comments after `null` branches should stay before `:`.
#[test]
fn test_format_jsx_chain_null_branch_separator_block_comment() {
    assert_format_program_roundtrip_with_file_type(
        "const value = <>{condition ? null /* branch-note */ : other ? <A /> : <B />}</>\n",
        "const value = <>{condition ? null /* branch-note */ : other ? <A /> : <B />}</>;\n",
        FileType::TypeScriptXml,
        DestackFormatOptions::default_with_line_width(100),
    );
}
