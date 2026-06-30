use crate::tests::{DirRows, TestSession};

#[test]
fn test_noinfer_keeps_inference_from_earlier_arguments() {
    let session = TestSession::single(
        r#"
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;

const ok = choose(["red", "blue"], "red");
ok satisfies "red" | "blue";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;

const ok: "red" | "blue" = choose<"red" | "blue">(
    ["red", "blue"],
    "red" as "red" | "blue" | undefined,
);
ok satisfies "red" | "blue";

=== checked ===
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;
/// @generic.template symbol=choose parameters=(C: string)
/// @type.symbol symbol=choose source="declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C" type=<C: string>(Array<C>, types.object.NoInfer<C> | undefined) => C
/// @type.symbol symbol=choose.C source="C: string" type=C
/// @type.symbol symbol=choose.values source="values: C[]" type=Array<C>
/// @resolution.name source=C target=choose.C
/// @type.symbol symbol=choose.fallback source="fallback?: NoInfer<C>" type=types.object.NoInfer<C> | undefined
/// @resolution.name source=NoInfer target=types.object.NoInfer
/// @resolution.name source=C target=choose.C
/// @resolution.name source=C target=choose.C

const ok = choose(["red", "blue"], "red");
/// @type.symbol symbol=ok source=ok type="red" | "blue"
/// @resolution.name source=choose target=choose
/// @resolution.call source="choose([\"red\", \"blue\"], \"red\")" parameters=(Array<"red" | "blue">, "red" | "blue" | undefined) arguments=(provided(["red", "blue"]) as Array<"red" | "blue">, provided("red") as "red" | "blue" | undefined) return="red" | "blue" kind=symbol target=choose instance="choose<\"red\" | \"blue\">"
/// @generic.instance source="choose([\"red\", \"blue\"], \"red\")" id="choose<\"red\" | \"blue\">"

ok satisfies "red" | "blue";
/// @resolution.name source=ok target=ok

/// @generic.instance id="choose<\"red\" | \"blue\">" template=choose arguments=("red" | "blue")
/// @generic.instance id=types.object.NoInfer<C> template=types.object.NoInfer arguments=(C)
"#,
    );
}

#[test]
fn test_noinfer_rejects_unrelated_later_arguments() {
    let session = TestSession::single(
        r#"
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;

choose(["red", "blue"], "green");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;

choose(["red", "blue"], "green");

=== checked ===
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;
/// @generic.template symbol=choose parameters=(C: string)
/// @type.symbol symbol=choose source="declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C" type=<C: string>(Array<C>, types.object.NoInfer<C> | undefined) => C
/// @type.symbol symbol=choose.C source="C: string" type=C
/// @type.symbol symbol=choose.values source="values: C[]" type=Array<C>
/// @resolution.name source=C target=choose.C
/// @type.symbol symbol=choose.fallback source="fallback?: NoInfer<C>" type=types.object.NoInfer<C> | undefined
/// @resolution.name source=NoInfer target=types.object.NoInfer
/// @resolution.name source=C target=choose.C
/// @resolution.name source=C target=choose.C

choose(["red", "blue"], "green");
/// @resolution.name source=choose target=choose

/// @generic.instance id=types.object.NoInfer<C> template=types.object.NoInfer arguments=(C)
"#,
        r#"
/// @diagnostic.error code=EC209 message="argument of type '\"green\"' is not assignable to parameter of type '\"red\" | \"blue\" | undefined'"
/// @diagnostic.label line=4 column=25 span="\"green\"" line_source="choose([\"red\", \"blue\"], \"green\");"
"#,
    );
}
