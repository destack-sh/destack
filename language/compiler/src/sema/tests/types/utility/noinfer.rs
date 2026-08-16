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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;

const ok: "red" | "blue" = choose<"red" | "blue">(
    ["red" as "red" | "blue", "blue" as "red" | "blue"],
    "red" as "red" | "blue" | undefined,
);
ok satisfies "red" | "blue";

=== dir ===
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;
/// @generic.template symbol=choose parameters=(C: string)
/// @type.symbol symbol=choose source="declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C" type=<C: string>(Array<C>, NoInfer<C> | undefined?) => C
/// @type.symbol symbol=choose.C source="C: string" type=C
/// @type.symbol symbol=choose.values source="values: C[]" type=Array<C>
/// @resolution.name source=C target=choose.C
/// @type.symbol symbol=choose.fallback source="fallback?: NoInfer<C>" type=NoInfer<C> | undefined
/// @resolution.name source=NoInfer target=types.object.NoInfer
/// @resolution.name source=C target=choose.C
/// @resolution.name source=C target=choose.C

const ok = choose(["red", "blue"], "red");
/// @type.symbol symbol=ok source=ok type="red" | "blue"
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=choose target=choose
/// @resolution.call source="choose([\"red\", \"blue\"], \"red\")" parameters=(Array<"red" | "blue">, "red" | "blue" | undefined) arguments=(provided(["red", "blue"]) as Array<"red" | "blue">, provided("red") as "red" | "blue" | undefined) return="red" | "blue" kind=symbol target=choose instance="choose<\"red\" | \"blue\">"
/// @generic.instantiation id="choose<\"red\" | \"blue\">" template=choose arguments=("red" | "blue")
/// @generic.instance id="Array<\"red\" | \"blue\">" template=collections.array.Array arguments=("red" | "blue")
/// @generic.instance id="NoInfer<\"red\" | \"blue\">" template=types.object.NoInfer arguments=("red" | "blue")
/// @generic.instance id="choose<\"red\" | \"blue\">" template=choose arguments=("red" | "blue")
/// @generic.instance id="memory.init.MaybeUninit<\"red\" | \"blue\">" template=memory.init.MaybeUninit arguments=("red" | "blue")
/// @generic.instance id="memory.unique.Unique<Slice<memory.init.MaybeUninit<\"red\" | \"blue\">>>" template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<"red" | "blue">>)
/// @generic.instance id="memory.unique.empty<memory.init.MaybeUninit<\"red\" | \"blue\">>" template=memory.unique.empty arguments=(memory.init.MaybeUninit<"red" | "blue">)

ok satisfies "red" | "blue";
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="local" lifetime="static" access="readonly"
/// @resolution.access source=ok root=ok
"#,
    );
}

#[test]
fn test_noinfer_never_binds_open_outer_inference() {
    let session = TestSession::single(
        r#"
declare function choose<C: string>(values: C[], fallback: NoInfer<C>): C;
declare function make<T>(): T[];

const values = make();
const picked = choose(values, "green");
const reds: "red"[] = values;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function choose<C: string>(values: C[], fallback: NoInfer<C>): C;
declare function make<T>(): T[];

const values: "red"[] = make<"red">();
const picked: "red" = choose<"red">(values, "green");
const reds: "red"[] = values;

=== dir ===
declare function choose<C: string>(values: C[], fallback: NoInfer<C>): C;
/// @generic.template symbol=choose parameters=(C: string)
/// @type.symbol symbol=choose source="declare function choose<C: string>(values: C[], fallback: NoInfer<C>): C" type=<C: string>(Array<C>, NoInfer<C>) => C
/// @type.symbol symbol=choose.C source="C: string" type=C
/// @type.symbol symbol=choose.values source="values: C[]" type=Array<C>
/// @resolution.name source=C target=choose.C
/// @type.symbol symbol=choose.fallback source="fallback: NoInfer<C>" type=NoInfer<C>
/// @resolution.name source=NoInfer target=types.object.NoInfer
/// @resolution.name source=C target=choose.C
/// @resolution.name source=C target=choose.C

declare function make<T>(): T[];
/// @generic.template symbol=make parameters=(T)
/// @type.symbol symbol=make source="declare function make<T>(): T[]" type=<T>() => Array<T>
/// @type.symbol symbol=make.T source=T type=T
/// @resolution.name source=T target=make.T

const values = make();
/// @type.symbol symbol=values source=values type=Array<"red">
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=make target=make
/// @resolution.call source=make() parameters=() return=Array<"red"> kind=symbol target=make instance="make<\"red\">"
/// @generic.instantiation id="make<\"red\">" template=make arguments=("red")

const picked = choose(values, "green");
/// @type.symbol symbol=picked source=picked type="red"
/// @resolution.pattern source=picked kind=binding target=picked
/// @resolution.name source=choose target=choose
/// @resolution.call source="choose(values, \"green\")" parameters=(Array<"red">, "red") arguments=(provided(values) as Array<"red">, provided("green") as "red") return="red" kind=symbol target=choose instance="choose<\"red\">"
/// @generic.instantiation id="choose<\"red\">" template=choose arguments=("red")
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values

const reds: "red"[] = values;
/// @type.symbol symbol=reds source=reds type=Array<"red">
/// @resolution.pattern source=reds kind=binding target=reds
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"green\"' is not assignable to parameter of type '\"red\"'"
/// @diagnostic.label line=6 column=31 span="\"green\"" line_source="const picked = choose(values, \"green\");"
/// @diagnostic.related line=6 column=16 span="choose(values, \"green\")" line_source="const picked = choose(values, \"green\");" message="in this call"
"#,
    );
}

#[test]
fn test_noinfer_composite_targets_verify_once_closed() {
    let session = TestSession::single(
        r#"
declare function keep<C: string>(values: C[], extras: NoInfer<C[]>): C;
declare function make<T>(): T[];

const values = make();
const kept = keep(values, ["green"]);
const reds: "red"[] = values;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function keep<C: string>(values: C[], extras: NoInfer<C[]>): C;
declare function make<T>(): T[];

const values: "red"[] = make<"red">();
const kept: "red" = keep<"red">(values, ["green"]);
const reds: "red"[] = values;

=== dir ===
declare function keep<C: string>(values: C[], extras: NoInfer<C[]>): C;
/// @generic.template symbol=keep parameters=(C: string)
/// @type.symbol symbol=keep source="declare function keep<C: string>(values: C[], extras: NoInfer<C[]>): C" type=<C: string>(Array<C>, NoInfer<Array<C>>) => C
/// @type.symbol symbol=keep.C source="C: string" type=C
/// @type.symbol symbol=keep.values source="values: C[]" type=Array<C>
/// @resolution.name source=C target=keep.C
/// @type.symbol symbol=keep.extras source="extras: NoInfer<C[]>" type=NoInfer<Array<C>>
/// @resolution.name source=NoInfer target=types.object.NoInfer
/// @resolution.name source=C target=keep.C
/// @resolution.name source=C target=keep.C

declare function make<T>(): T[];
/// @generic.template symbol=make parameters=(T)
/// @type.symbol symbol=make source="declare function make<T>(): T[]" type=<T>() => Array<T>
/// @type.symbol symbol=make.T source=T type=T
/// @resolution.name source=T target=make.T

const values = make();
/// @type.symbol symbol=values source=values type=Array<"red">
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=make target=make
/// @resolution.call source=make() parameters=() return=Array<"red"> kind=symbol target=make instance="make<\"red\">"
/// @generic.instantiation id="make<\"red\">" template=make arguments=("red")

const kept = keep(values, ["green"]);
/// @type.symbol symbol=kept source=kept type="red"
/// @resolution.pattern source=kept kind=binding target=kept
/// @resolution.name source=keep target=keep
/// @resolution.call source="keep(values, [\"green\"])" parameters=(Array<"red">, Array<"red">) arguments=(provided(values) as Array<"red">, provided(["green"]) as Array<"red">) return="red" kind=symbol target=keep instance="keep<\"red\">"
/// @generic.instantiation id="keep<\"red\">" template=keep arguments=("red")
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values

const reds: "red"[] = values;
/// @type.symbol symbol=reds source=reds type=Array<"red">
/// @resolution.pattern source=reds kind=binding target=reds
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"green\"' is not assignable to parameter of type '\"red\"'"
/// @diagnostic.label line=6 column=28 span="\"green\"" line_source="const kept = keep(values, [\"green\"]);"
/// @diagnostic.related line=6 column=14 span="keep(values, [\"green\"])" line_source="const kept = keep(values, [\"green\"]);" message="in this call"
/// @diagnostic.note message="the mismatch is in element 0"
"#,
    );
}

#[test]
fn test_noinfer_preserves_sibling_inference() {
    let session = TestSession::single(
        r#"
declare function first<T, U = string>(value: (T, NoInfer<U>)): T;

const value = first((1, "text"));
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function first<T, U = string>(value: (T, NoInfer<U>)): T;

const value: float64 = first<float64, string>((1, "text"));

=== dir ===
declare function first<T, U = string>(value: (T, NoInfer<U>)): T;
/// @generic.template symbol=first parameters=(T, U = string)
/// @type.symbol symbol=first source="declare function first<T, U = string>(value: (T, NoInfer<U>)): T" type=<T, U = string>((T, NoInfer<U>)) => T
/// @type.symbol symbol=first.T source=T type=T
/// @type.symbol symbol=first.U source="U = string" type=U
/// @type.symbol symbol=first.value source="value: (T, NoInfer<U>)" type=(T, NoInfer<U>)
/// @resolution.name source=T target=first.T
/// @resolution.name source=NoInfer target=types.object.NoInfer
/// @resolution.name source=U target=first.U
/// @resolution.name source=T target=first.T

const value = first((1, "text"));
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=first target=first
/// @resolution.call source="first((1, \"text\"))" parameters=((float64, string)) arguments=(provided((1, "text")) as (float64, string)) return=float64 kind=symbol target=first instance="first<float64, string>"
/// @generic.instantiation id="first<float64, string>" template=first arguments=(float64, string)
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'NoInfer<string>' is not assignable to parameter of type 'string'"
/// @diagnostic.label line=4 column=25 span="\"text\"" line_source="const value = first((1, \"text\"));"
/// @diagnostic.related line=4 column=15 span="first((1, \"text\"))" line_source="const value = first((1, \"text\"));" message="in this call"
/// @diagnostic.note message="the mismatch is in element 1"
"#,
    );
}

#[test]
fn test_noinfer_still_contextually_types_lambda_arguments() {
    let session = TestSession::single(
        r#"
declare function on<T>(seeds: T[], callback: NoInfer<(value: T) => void>): void;
declare function make<T>(): T[];

const seeds = make();
on(seeds, (value) => {});
const reds: "red"[] = seeds;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function on<T>(seeds: T[], callback: NoInfer<(arg0: T) => void>): void;
declare function make<T>(): T[];

const seeds: "red"[] = make<"red">();
on<"red">(seeds, (value: "red"): void => {});
const reds: "red"[] = seeds;

=== dir ===
declare function on<T>(seeds: T[], callback: NoInfer<(value: T) => void>): void;
/// @generic.template symbol=on parameters=(T#1)
/// @type.symbol symbol=on source="declare function on<T>(seeds: T[], callback: NoInfer<(value: T) => void>): void" type=<T#1>(Array<T#1>, NoInfer<Function<(T#1,), void>>) => void
/// @type.symbol symbol=on.T source=T type=T#1
/// @type.symbol symbol=on.seeds source="seeds: T[]" type=Array<T#1>
/// @resolution.name source=T target=on.T
/// @type.symbol symbol=on.callback source="callback: NoInfer<(value: T) => void>" type=NoInfer<Function<(T#1,), void>>
/// @resolution.name source=NoInfer target=types.object.NoInfer
/// @type.symbol symbol=on.value source="value: T" type=T#1
/// @resolution.name source=T target=on.T

declare function make<T>(): T[];
/// @generic.template symbol=make parameters=(T#2)
/// @type.symbol symbol=make source="declare function make<T>(): T[]" type=<T#2>() => Array<T#2>
/// @type.symbol symbol=make.T source=T type=T#2
/// @resolution.name source=T target=make.T

const seeds = make();
/// @type.symbol symbol=seeds source=seeds type=Array<"red">
/// @resolution.pattern source=seeds kind=binding target=seeds
/// @resolution.name source=make target=make
/// @resolution.call source=make() parameters=() return=Array<"red"> kind=symbol target=make instance="make<\"red\">"
/// @generic.instantiation id="make<\"red\">" template=make arguments=("red")

on(seeds, (value) => {});
/// @resolution.name source=on target=on
/// @resolution.call source="on(seeds, (value) => {})" parameters=(Array<"red">, Function<("red",), void>) arguments=(provided(seeds) as Array<"red">, provided((value) => {}) as Function<("red",), void>) return=void kind=symbol target=on instance="on<\"red\">"
/// @generic.instantiation id="on<\"red\">" template=on arguments=("red")
/// @resolution.name source=seeds target=seeds
/// @resolution.place source=seeds placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=seeds root=seeds
/// @type.symbol symbol=symbol9 source="(value) => {}" type=Function<("red",), void>
/// @type.symbol symbol=symbol9.value source=value type="red"

const reds: "red"[] = seeds;
/// @type.symbol symbol=reds source=reds type=Array<"red">
/// @resolution.pattern source=reds kind=binding target=reds
/// @resolution.name source=seeds target=seeds
/// @resolution.place source=seeds placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=seeds root=seeds
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'NoInfer<(value: \"red\") => void>' is not assignable to parameter of type '(value: \"red\") => void'"
/// @diagnostic.label line=6 column=11 span="(value) => {}" line_source="on(seeds, (value) => {});"
/// @diagnostic.related line=6 column=1 span="on(seeds, (value) => {})" line_source="on(seeds, (value) => {});" message="in this call"
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;

choose<"red" | "blue">(["red" as "red" | "blue", "blue" as "red" | "blue"], "green");

=== dir ===
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;
/// @generic.template symbol=choose parameters=(C: string)
/// @type.symbol symbol=choose source="declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C" type=<C: string>(Array<C>, NoInfer<C> | undefined?) => C
/// @type.symbol symbol=choose.C source="C: string" type=C
/// @type.symbol symbol=choose.values source="values: C[]" type=Array<C>
/// @resolution.name source=C target=choose.C
/// @type.symbol symbol=choose.fallback source="fallback?: NoInfer<C>" type=NoInfer<C> | undefined
/// @resolution.name source=NoInfer target=types.object.NoInfer
/// @resolution.name source=C target=choose.C
/// @resolution.name source=C target=choose.C

choose(["red", "blue"], "green");
/// @resolution.name source=choose target=choose
/// @resolution.call source="choose([\"red\", \"blue\"], \"green\")" parameters=(Array<"red" | "blue">, "red" | "blue" | undefined) arguments=(provided(["red", "blue"]) as Array<"red" | "blue">, provided("green") as "red" | "blue" | undefined) return="red" | "blue" kind=symbol target=choose instance="choose<\"red\" | \"blue\">"
/// @generic.instantiation id="choose<\"red\" | \"blue\">" template=choose arguments=("red" | "blue")
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"green\"' is not assignable to parameter of type '\"red\" | \"blue\" | undefined'"
/// @diagnostic.label line=4 column=25 span="\"green\"" line_source="choose([\"red\", \"blue\"], \"green\");"
/// @diagnostic.related line=4 column=1 span="choose([\"red\", \"blue\"], \"green\")" line_source="choose([\"red\", \"blue\"], \"green\");" message="in this call"
"#,
    );
}
