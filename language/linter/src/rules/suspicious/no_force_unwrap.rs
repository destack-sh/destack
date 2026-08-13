use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow force unwraps.
    pub NO_FORCE_UNWRAP {
        id: "no-force-unwrap",
        summary: "Disallow force unwraps",
        explanation: r#"
Force unwrap turns an expected absence or failure into a panic. Propagate a typed failure or handle
both cases explicitly; suppress the lint with a reason when an external invariant makes failure
impossible.
"#,
        example: {
            reported: r#"
function load(config?: Config): Config {
    return config!;
}
"#,
            accepted: r#"
function load(config?: Config): Result<Config, string> {
    if (config === undefined) {
        return Result.err("configuration is missing");
    }

    return Result.ok(config);
}
"#,
        },
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
