use super::counters::assert_check_counters;

const ITEMS: usize = 1_000;

/// Repeated extension method calls reuse one canonical member decision.
#[test]
fn test_repeated_extension_method_calls_reuse_one_member_decision() {
    let body = (0..ITEMS)
        .map(|index| format!("    let value{index}: int32 = 0.double();"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "extension of int32 {{\n    double(this): int32 {{\n        this + this\n    }}\n}}\n\n\
         function heavy(): void {{\n{body}\n}}"
    );

    assert_check_counters(
        &source,
        r#"
check.solve.variables=14
check.solve.constraints=0
check.solve.obligations=1000
check.solve.solutions=14
check.solve.bounds=0
check.solve.decisions=3003
check.relations.decided=5
check.relations.reused=1003
check.bindings.built=1
check.bindings.reused=1003
check.members.derived=4
check.members.reused=3996
check.members.refused=0
check.probes.total=2036
check.probes.selections=0
check.probes.extensions=0
check.instantiations=22
check.interns=21139
check.reduces=34116
"#,
    );
}
