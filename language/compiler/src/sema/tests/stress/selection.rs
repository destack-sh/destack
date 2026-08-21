use super::stats::assert_check_stats;

const ITEMS: usize = 1_000;

/// Repeated identical generic calls each type their own site on a cold pass.
#[test]
fn test_repeated_generic_calls_type_each_site() {
    let body = (0..ITEMS)
        .map(|index| format!("    let value{index}: int32 = pick(1, 2);"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "function pick<T>(a: T, b: T): T {{\n    a\n}}\n\nfunction heavy(): void {{\n{body}\n}}"
    );

    assert_check_stats(
        &source,
        r#"
check.stats.solve.variables=3000
check.stats.solve.constraints=5000
check.stats.solve.obligations=1000
check.stats.solve.solutions=3000
check.stats.solve.bounds=4000
check.stats.solve.decisions=2000
check.stats.judges.decided=2
check.stats.judges.replayed=1998
check.stats.bindings.built=0
check.stats.bindings.replayed=0
check.stats.members.derived=0
check.stats.members.replayed=0
check.stats.members.refused=0
check.stats.probes.total=4003
check.stats.probes.selections=0
check.stats.probes.extensions=0
check.stats.instantiations=1000
check.stats.interns=18012
check.stats.reduces=45008
"#,
    );
}

/// Distinct literal arguments instantiate one generic callee per site.
#[test]
fn test_distinct_literal_calls_instantiate_per_site() {
    let body = (0..ITEMS)
        .map(|index| format!("    let value{index}: int32 = pick({index}, {index});"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "function pick<T>(a: T, b: T): T {{\n    a\n}}\n\nfunction heavy(): void {{\n{body}\n}}"
    );

    assert_check_stats(
        &source,
        r#"
check.stats.solve.variables=3000
check.stats.solve.constraints=5000
check.stats.solve.obligations=1000
check.stats.solve.solutions=3000
check.stats.solve.bounds=4000
check.stats.solve.decisions=2000
check.stats.judges.decided=1000
check.stats.judges.replayed=1000
check.stats.bindings.built=0
check.stats.bindings.replayed=0
check.stats.members.derived=0
check.stats.members.replayed=0
check.stats.members.refused=0
check.stats.probes.total=4003
check.stats.probes.selections=0
check.stats.probes.extensions=0
check.stats.instantiations=1000
check.stats.interns=18012
check.stats.reduces=45008
"#,
    );
}
