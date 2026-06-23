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

const ok: "red" | "blue" = choose<"red" | "blue">(["red", "blue"], "red");
ok satisfies "red" | "blue";

=== checked ===
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;
/// @generic.template symbol=choose parameters=(C: string)
/// @type.symbol symbol=choose type=<C: string>(Array<C>, NoInfer<C> | undefined) => C
/// @type.symbol symbol=values type=Array<C>
/// @type.symbol symbol=fallback type=NoInfer<C> | undefined
/// @resolution.name source=NoInfer target=types.object.NoInfer

const ok = choose(["red", "blue"], "red");
/// @type.symbol symbol=ok type="red" | "blue"
/// @resolution.name source=choose target=choose
/// @resolution.call source="choose([\"red\", \"blue\"], \"red\")" parameters=(Array<"red" | "blue">, NoInfer<"red" | "blue">) return="red" | "blue" kind=symbol target=choose instance="choose<\"red\" | \"blue\">"
/// @generic.instance source="choose([\"red\", \"blue\"], \"red\")" id="choose<\"red\" | \"blue\">"

ok satisfies "red" | "blue";
/// @resolution.name source=ok target=ok
/// @generic.instance id="choose<\"red\" | \"blue\">" template=choose arguments=("red" | "blue")
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

choose<"red" | "blue">(["red", "blue"], "green");

=== checked ===
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;
/// @generic.template symbol=choose parameters=(C: string)
/// @type.symbol symbol=choose type=<C: string>(Array<C>, NoInfer<C> | undefined) => C
/// @type.symbol symbol=values type=Array<C>
/// @type.symbol symbol=fallback type=NoInfer<C> | undefined
/// @resolution.name source=NoInfer target=types.object.NoInfer

choose(["red", "blue"], "green");
/// @resolution.name source=choose target=choose
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"green\"' is not assignable to type 'NoInfer<\"red\" | \"blue\">'"
/// @diagnostic.label line=4 column=1 source="choose([\"red\", \"blue\"], \"green\");"
"#,
    );
}
