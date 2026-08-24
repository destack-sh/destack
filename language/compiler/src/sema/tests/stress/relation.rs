use super::counters::assert_check_counters;

const ITEMS: usize = 1_000;

/// Repeated union conversions replay one canonical relation answer.
#[test]
fn test_repeated_union_conversions_reuse_one_relation_answer() {
    let body = (0..ITEMS)
        .map(|index| format!("    let value{index}: int32 | undefined = 0;"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("function heavy(): void {{\n{body}\n}}");

    assert_check_counters(
        &source,
        r#"
check.solve.variables=0
check.solve.constraints=0
check.solve.obligations=1000
check.solve.solutions=0
check.solve.bounds=0
check.solve.decisions=1000
check.relations.decided=4
check.relations.reused=1000
check.bindings.built=0
check.bindings.reused=0
check.members.derived=0
check.members.reused=0
check.members.refused=0
check.probes.total=3007
check.probes.selections=0
check.probes.extensions=0
check.instantiations=0
check.interns=8009
check.reduces=16018
"#,
    );
}
