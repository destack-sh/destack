use crate::tests::TestSession;

#[test]
fn test_lower_borrowed_signature_alias_fields_without_arguments() {
    let session = TestSession::single(
        r#"
type Predicate = (value: &readonly int32) => boolean;

struct Rule {
    accept: Predicate;
}

function test(rule: &readonly Rule): int32 {
    return 3;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Predicate = function<<'a>(ref<int32, borrowed, 'a, readonly>) => boolean, repeatable, managed, mutable>;

@copy
type Rule {
    accept: Predicate;
}

function test.main.test<'a>(v0: ref<Rule, borrowed, 'a, readonly>): int32 {
entry(v0: ref<Rule, borrowed, 'a, readonly>):
    v1: int32 = 3
    return v1
}

/// @layout.struct name=Rule size=16 align=8
/// @layout.field owner=Rule index=0 name=accept offset=0 size=16 align=8
"#,
    );
}
