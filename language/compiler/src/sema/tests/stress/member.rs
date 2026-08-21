use super::stats::assert_check_stats;

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

    assert_check_stats(
        &source,
        r#"
check.stats.solve.variables=46
check.stats.solve.constraints=0
check.stats.solve.obligations=1000
check.stats.solve.solutions=46
check.stats.solve.bounds=0
check.stats.solve.decisions=3003
check.stats.judges.decided=90
check.stats.judges.replayed=1106
check.stats.bindings.built=1
check.stats.bindings.replayed=1003
check.stats.members.derived=4
check.stats.members.replayed=3996
check.stats.members.refused=0
check.stats.probes.total=6624
check.stats.probes.selections=0
check.stats.probes.extensions=342
check.stats.instantiations=173
check.stats.interns=19957
check.stats.reduces=29095
"#,
    );
}
