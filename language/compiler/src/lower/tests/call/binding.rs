use crate::tests::TestSession;

#[test]
fn test_lower_binding_call_to_a_dotted_host_extern() {
    let session = TestSession::single(
        r#"
@binding("destack.clock.now", {
    provider: "host",
    effect: "external",
    requires: [],
})
declare function now(): float64;

function sample(): float64 {
    return now();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.sample(): float64 {
entry:
    v0: float64 = call destack.clock.now()
    return v0
}

@binding("destack.clock.now")
external function destack.clock.now(): float64
"#,
    );
}
