use crate::tests::{DirRows, TestSession};

/// A literal type read back through a name stays exact at generic calls.
#[test]
fn test_keep_a_literal_type_read_back_through_a_name_unwidened() {
    let session = TestSession::single(
        r#"
declare function id<T>(value: T): T;
declare function withLabel<T: string, U>(label: T, callback: (label: T) => U): U;

const tag: "users" = "users";
const again = id(tag);
const fresh = id("users");
const tags = [tag];
const literals = ["users"];
const echoed = withLabel("users", (label) => label);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;
declare function withLabel<T: string, U>(label: T, callback: (label: T) => U): U;

const tag: "users" = "users";
const again: "users" = id<"users">(tag);
const fresh: "users" = id<"users">("users");
const tags: "users"[] = [tag];
const literals: string[] = ["users"];
const echoed: "users" = withLabel<"users", "users">("users", (label: "users"): "users" => label);

=== dir ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=(T#1)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T#1>(T#1) => T#1
/// @type.symbol symbol=id.T source=T type=T#1
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

declare function withLabel<T: string, U>(label: T, callback: (label: T) => U): U;
/// @generic.template symbol=withLabel parameters=(T#2: string, U)
/// @type.symbol symbol=withLabel source="declare function withLabel<T: string, U>(label: T, callback: (label: T) => U): U" type=<T#2: string, U>(T#2, (T#2) => U) => U
/// @type.symbol symbol=withLabel.T source="T: string" type=T#2
/// @type.symbol symbol=withLabel.U source=U type=U
/// @resolution.name source=T target=withLabel.T
/// @type.symbol symbol=withLabel.label#2 source="label: T" type=T#2
/// @resolution.name source=T target=withLabel.T
/// @resolution.name source=U target=withLabel.U
/// @resolution.name source=U target=withLabel.U

const tag: "users" = "users";
/// @type.symbol symbol=tag source=tag type="users"
/// @resolution.pattern source=tag kind=binding target=tag

const again = id(tag);
/// @type.symbol symbol=again source=again type="users"
/// @resolution.pattern source=again kind=binding target=again
/// @resolution.name source=id target=id
/// @resolution.call source=id(tag) parameters=("users") arguments=(provided(tag) as "users") return="users" kind=symbol target=id instance="id<\"users\">"
/// @generic.instantiation id="id<\"users\">" template=id arguments=("users")
/// @resolution.name source=tag target=tag
/// @resolution.place source=tag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=tag root=tag

const fresh = id("users");
/// @type.symbol symbol=fresh source=fresh type="users"
/// @resolution.pattern source=fresh kind=binding target=fresh
/// @resolution.name source=id target=id
/// @resolution.call source="id(\"users\")" parameters=("users") arguments=(provided("users") as "users") return="users" kind=symbol target=id instance="id<\"users\">"

const tags = [tag];
/// @type.symbol symbol=tags source=tags type="users"[]
/// @resolution.pattern source=tags kind=binding target=tags
/// @resolution.call source=[tag] parameters=(^Slice<"users">) arguments=(rest(provided(tag) as "users") as "users") return="users"[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<\"users\">"
/// @generic.instantiation id="arrayFromOwnedSlice<\"users\">" template=arrayFromOwnedSlice arguments=("users")
/// @resolution.name source=tag target=tag
/// @resolution.place source=tag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=tag root=tag

const literals = ["users"];
/// @type.symbol symbol=literals source=literals type=string[]
/// @resolution.pattern source=literals kind=binding target=literals
/// @resolution.call source=["users"] parameters=(^Slice<string>) arguments=(rest(provided("users") as string) as string) return=string[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<string>
/// @generic.instantiation id=arrayFromOwnedSlice<string> template=arrayFromOwnedSlice arguments=(string)

const echoed = withLabel("users", (label) => label);
/// @type.symbol symbol=echoed source=echoed type="users"
/// @resolution.pattern source=echoed kind=binding target=echoed
/// @resolution.name source=withLabel target=withLabel
/// @resolution.call source="withLabel(\"users\", (label) => label)" parameters=("users", ("users") => "users") arguments=(provided("users") as "users", provided((label) => label) as ("users") => "users") return="users" kind=symbol target=withLabel instance="withLabel<\"users\", \"users\">"
/// @generic.instantiation id="withLabel<\"users\", \"users\">" template=withLabel arguments=("users", "users")
/// @type.symbol symbol=symbol15 source="(label) => label" type=Function<("users",), "users", "readonly">
/// @type.symbol symbol=symbol15.label source=label type="users"
/// @resolution.name source=label target=symbol15.label
/// @resolution.place source=label placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=label root=symbol15.label
"#,
        r#"
"#,
    );
}

/// Fresh literal forms widen while types read back through a name stay exact.
#[test]
fn test_widen_fresh_literal_forms_and_keep_read_back_types() {
    let session = TestSession::single(
        r#"
declare function id<T>(value: T): T;

const tag: "users" = "users";
let rebound = tag;
const negative = id(-1);
const text = id(`users`);
const field = id({ name: tag });
const fresh = id({ name: "users" });
const list = id([tag, "users"]);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;

const tag: "users" = "users";
let rebound: "users" = tag;
const negative: -1 = id<-1>(-1);
const text: "users" = id<"users">(`users`);
const field: { name: "users" } = id<{ name: "users" }>({ name: tag });
const fresh: { name: string } = id<{ name: string }>({ name: "users" });
const list: string[] = id<string[]>([tag, "users"]);

=== dir ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=(T)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=id.T source=T type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const tag: "users" = "users";
/// @type.symbol symbol=tag source=tag type="users"
/// @resolution.pattern source=tag kind=binding target=tag

let rebound = tag;
/// @type.symbol symbol=rebound source=rebound type="users"
/// @resolution.pattern source=rebound kind=binding target=rebound
/// @resolution.name source=tag target=tag
/// @resolution.access source=tag root=tag

const negative = id(-1);
/// @type.symbol symbol=negative source=negative type=-1
/// @resolution.pattern source=negative kind=binding target=negative
/// @resolution.name source=id target=id
/// @resolution.call source=id(-1) parameters=(-1) arguments=(provided(-1) as -1) return=-1 kind=symbol target=id instance=id<-1>
/// @generic.instantiation id=id<-1> template=id arguments=(-1)
/// @resolution.operator source=-1 type=-1 operator="-" kind=builtin operands=[1 as 1 families=(integer)]

const text = id(`users`);
/// @type.symbol symbol=text source=text type="users"
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=id target=id
/// @resolution.call source=id(`users`) parameters=("users") arguments=(provided(`users`) as "users") return="users" kind=symbol target=id instance="id<\"users\">"
/// @generic.instantiation id="id<\"users\">" template=id arguments=("users")

const field = id({ name: tag });
/// @type.symbol symbol=field source=field type={ name: "users" }
/// @resolution.pattern source=field kind=binding target=field
/// @resolution.name source=id target=id
/// @resolution.call source="id({ name: tag })" parameters=({ name: "users" }) arguments=(provided({ name: tag }) as { name: "users" }) return={ name: "users" } kind=symbol target=id instance="id<{ name: \"users\" }>"
/// @generic.instantiation id="id<{ name: \"users\" }>" template=id arguments=({ name: "users" })
/// @resolution.name source=tag target=tag
/// @resolution.place source=tag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=tag root=tag

const fresh = id({ name: "users" });
/// @type.symbol symbol=fresh source=fresh type={ name: string }
/// @resolution.pattern source=fresh kind=binding target=fresh
/// @resolution.name source=id target=id
/// @resolution.call source="id({ name: \"users\" })" parameters=({ name: string }) arguments=(provided({ name: "users" }) as { name: string }) return={ name: string } kind=symbol target=id instance="id<{ name: string }>"
/// @generic.instantiation id="id<{ name: string }>" template=id arguments=({ name: string })

const list = id([tag, "users"]);
/// @type.symbol symbol=list source=list type=string[]
/// @resolution.pattern source=list kind=binding target=list
/// @resolution.name source=id target=id
/// @resolution.call source="id([tag, \"users\"])" parameters=(string[]) arguments=(provided([tag, "users"]) as string[]) return=string[] kind=symbol target=id instance=id<string[]>
/// @generic.instantiation id=id<string[]> template=id arguments=(string[])
/// @resolution.call source=[tag, "users"] parameters=(^Slice<string>) arguments=(rest(provided(tag) as string, provided("users") as string) as string) return=string[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<string>
/// @generic.instantiation id=arrayFromOwnedSlice<string> template=arrayFromOwnedSlice arguments=(string)
/// @resolution.name source=tag target=tag
/// @resolution.place source=tag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=tag root=tag
"#,
        r#"
"#,
    );
}
