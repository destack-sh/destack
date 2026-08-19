use super::stats::assert_check_stats;

const ITEMS: usize = 1_000;

/// Repeated union conversions replay one canonical relation answer.
#[test]
fn test_repeated_union_conversions_replay_one_relation_answer() {
    let body = (0..ITEMS)
        .map(|index| format!("    let value{index}: int32 | undefined = 0;"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("function heavy(): void {{\n{body}\n}}");

    assert_check_stats(
        &source,
        r#"
check.stats.solve.variables=0
check.stats.solve.constraints=0
check.stats.solve.obligations=1000
check.stats.solve.solutions=0
check.stats.solve.bounds=0
check.stats.solve.decisions=1000
check.stats.judges.decided=4
check.stats.judges.replayed=1000
check.stats.bindings.built=0
check.stats.bindings.replayed=0
check.stats.members.derived=0
check.stats.members.replayed=0
check.stats.members.refused=0
check.stats.probes.total=3007
check.stats.probes.selections=0
check.stats.probes.extensions=0
check.stats.instantiations=0
check.stats.interns=8009
check.stats.reduces=16018
"#,
    );
}
