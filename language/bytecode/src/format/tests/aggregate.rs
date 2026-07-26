use super::format_fixture;

/// Format physical aggregate byte placement and variant layouts canonically.
#[test]
fn test_format_aggregate_operations() {
    let input = r#"
function f0 {
    aggregate r2,([r0,0,4],[r1,8,8])
extract r4, r2,0,4
insert r5, r2,8,8,r1
variant.new r7, l1,1,r0
variant.tag r9, r7,l1
return r4
}
"#;
    let expected = r#"
function f0 {
    aggregate r2, ([r0, 0, 4], [r1, 8, 8])
    extract r4, r2, 0, 4
    insert r5, r2, 8, 8, r1
    variant.new r7, l1, 1, r0
    variant.tag r9, r7, l1
    return r4
}
"#;

    let formatted = format_fixture(input);
    assert_eq!(formatted, expected.trim());
}
