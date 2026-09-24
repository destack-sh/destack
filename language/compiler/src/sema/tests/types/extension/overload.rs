use crate::tests::{DirRows, TestSession};

#[test]
fn test_unreachable_extension_overload_reports_warning() {
    let session = TestSession::single(
        r#"
struct User {}

extension of User {
    display(): ^string {
        return "first";
    }

    display(): ^string {
        return "second";
    }

    greet(name: string): string {
        return name;
    }

    greet(name: "admin"): string {
        return "root";
    }

    label(value: int32): string {
        return "number";
    }

    label(value: string): string {
        return value;
    }

    pair(left: int32): string {
        return "one";
    }

    pair(left: int32, right: string): string {
        return right;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct User {}

extension of User {
    display(): ^string {
        return "first" as ^string;
    }

    display(): ^string {
        return "second" as ^string;
    }

    greet(name: string): string {
        return name;
    }

    greet(name: "admin"): string {
        return "root";
    }

    label(value: int32): string {
        return "number";
    }

    label(value: string): string {
        return value;
    }

    pair(left: int32): string {
        return "one";
    }

    pair(left: int32, right: string): string {
        return right;
    }
}

=== dir ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=display#1 slot=display type=<display#1.'a>(this: &display#1.'a readonly User) => ^string
/// @definition.method symbol=display#2 slot=display type=<display#2.'a>(this: &display#2.'a readonly User) => ^string
/// @definition.method symbol=greet#1 slot=greet type=<greet#1.'a>(this: &greet#1.'a readonly User, string) => string
/// @definition.method symbol=greet#2 slot=greet type=<greet#2.'a>(this: &greet#2.'a readonly User, "admin") => string
/// @definition.method symbol=label#1 slot=label type=<label#1.'a>(this: &label#1.'a readonly User, int32) => string
/// @definition.method symbol=label#2 slot=label type=<label#2.'a>(this: &label#2.'a readonly User, string) => string
/// @definition.method symbol=pair#1 slot=pair type=<pair#1.'a>(this: &pair#1.'a readonly User, int32) => string
/// @definition.method symbol=pair#2 slot=pair type=<pair#2.'a>(this: &pair#2.'a readonly User, int32, string) => string
/// @resolution.name source=User target=User

    display(): ^string {
    /// @generic.template symbol=display#1 parameters=('a)
    /// @type.symbol symbol=display#1 type=<display#1.'a>(this: &display#1.'a readonly User) => ^string
    /// @type.symbol symbol=display.this#1 type=&display#1.'a readonly User

        return "first";
        /// @generic.instantiation id="clone<\"managed\" & \"local\">" template=clone arguments=("managed" & "local")

    }

    display(): ^string {
    /// @generic.template symbol=display#2 parameters=('a)
    /// @type.symbol symbol=display#2 type=<display#2.'a>(this: &display#2.'a readonly User) => ^string
    /// @type.symbol symbol=display.this#2 type=&display#2.'a readonly User

        return "second";
    }

    greet(name: string): string {
    /// @generic.template symbol=greet#1 parameters=('a)
    /// @type.symbol symbol=greet#1 type=<greet#1.'a>(this: &greet#1.'a readonly User, string) => string
    /// @type.symbol symbol=greet.this#1 type=&greet#1.'a readonly User
    /// @type.symbol symbol=greet.name#1 source="name: string" type=string

        return name;
        /// @resolution.name source=name target=greet.name#1
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=greet.name#1

    }

    greet(name: "admin"): string {
    /// @generic.template symbol=greet#2 parameters=('a)
    /// @type.symbol symbol=greet#2 type=<greet#2.'a>(this: &greet#2.'a readonly User, "admin") => string
    /// @type.symbol symbol=greet.this#2 type=&greet#2.'a readonly User
    /// @type.symbol symbol=greet.name#2 source="name: \"admin\"" type="admin"

        return "root";
    }

    label(value: int32): string {
    /// @generic.template symbol=label#1 parameters=('a)
    /// @type.symbol symbol=label#1 type=<label#1.'a>(this: &label#1.'a readonly User, int32) => string
    /// @type.symbol symbol=label.this#1 type=&label#1.'a readonly User
    /// @type.symbol symbol=label.value#1 source="value: int32" type=int32

        return "number";
    }

    label(value: string): string {
    /// @generic.template symbol=label#2 parameters=('a)
    /// @type.symbol symbol=label#2 type=<label#2.'a>(this: &label#2.'a readonly User, string) => string
    /// @type.symbol symbol=label.this#2 type=&label#2.'a readonly User
    /// @type.symbol symbol=label.value#2 source="value: string" type=string

        return value;
        /// @resolution.name source=value target=label.value#2
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=label.value#2

    }

    pair(left: int32): string {
    /// @generic.template symbol=pair#1 parameters=('a)
    /// @type.symbol symbol=pair#1 type=<pair#1.'a>(this: &pair#1.'a readonly User, int32) => string
    /// @type.symbol symbol=pair.this#1 type=&pair#1.'a readonly User
    /// @type.symbol symbol=pair.left#1 source="left: int32" type=int32

        return "one";
    }

    pair(left: int32, right: string): string {
    /// @generic.template symbol=pair#2 parameters=('a)
    /// @type.symbol symbol=pair#2 type=<pair#2.'a>(this: &pair#2.'a readonly User, int32, string) => string
    /// @type.symbol symbol=pair.this#2 type=&pair#2.'a readonly User
    /// @type.symbol symbol=pair.left#2 source="left: int32" type=int32
    /// @type.symbol symbol=pair.right source="right: string" type=string

        return right;
        /// @resolution.name source=right target=pair.right
        /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=right root=pair.right

    }
}
"#,
        r#"
/// @diagnostic.warning id=unreachable-overload message="overload 'display' can never be selected"
/// @diagnostic.label line=9 column=5 span="display" line_source="display(): ^string {"
/// @diagnostic.warning id=unreachable-overload message="overload 'greet' can never be selected"
/// @diagnostic.label line=17 column=5 span="greet" line_source="greet(name: \"admin\"): string {"
"#,
    );
}

#[test]
fn test_later_owned_parameter_overload_stays_reachable_beside_a_borrowed_one() {
    let session = TestSession::single(
        r#"
struct Slice {}
struct Builder {}
struct Path {}

extension of Path {
    static from(value: &readonly Slice): Path {
        return Path {};
    }

    static from(value: ^Builder): Path {
        return Path {};
    }
}

declare const builder: ^Builder;
declare const slice: &readonly Slice;

const fromBuilder = Path.from(builder);
const fromSlice = Path.from(slice);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Slice {}
struct Builder {}
struct Path {}

extension of Path {
    static from(value: &'a readonly Slice): Path {
        return Path {};
    }

    static from(value: Builder): Path {
        return Path {};
    }
}

declare const builder: Builder;
declare const slice: &'static readonly Slice;

const fromBuilder: Path = Path.from(builder);
const fromSlice: Path = Path.from<"static">(slice);

=== dir ===
struct Slice {}
/// @type.symbol symbol=Slice source="struct Slice {}" type=Slice
/// @definition.struct symbol=Slice source="struct Slice {}"

struct Builder {}
/// @type.symbol symbol=Builder source="struct Builder {}" type=Builder
/// @definition.struct symbol=Builder source="struct Builder {}"

struct Path {}
/// @type.symbol symbol=Path source="struct Path {}" type=Path
/// @definition.struct symbol=Path source="struct Path {}"

extension of Path {
/// @definition.extension symbol=<module>#2 form=local target=Path
/// @definition.method symbol=from#1 slot=from static=true type=<from#1.'a>(&from#1.'a readonly Slice) => Path
/// @definition.method symbol=from#2 slot=from static=true type=(Builder) => Path
/// @resolution.name source=Path target=Path

    static from(value: &readonly Slice): Path {
    /// @generic.template symbol=from#1 parameters=('a)
    /// @type.symbol symbol=from#1 type=<from#1.'a>(&from#1.'a readonly Slice) => Path
    /// @type.symbol symbol=from.value#1 source="value: &readonly Slice" type=&from#1.'a readonly Slice
    /// @resolution.name source=Slice target=Slice
    /// @resolution.name source=Path target=Path

        return Path {};
        /// @resolution.name source=Path target=Path

    }

    static from(value: ^Builder): Path {
    /// @type.symbol symbol=from#2 type=(Builder) => Path
    /// @type.symbol symbol=from.value#2 source="value: ^Builder" type=Builder
    /// @resolution.name source=Builder target=Builder
    /// @resolution.name source=Path target=Path

        return Path {};
        /// @resolution.name source=Path target=Path

    }
}

declare const builder: ^Builder;
/// @type.symbol symbol=builder source=builder type=Builder
/// @resolution.pattern source=builder kind=binding target=builder
/// @resolution.name source=Builder target=Builder

declare const slice: &readonly Slice;
/// @type.symbol symbol=slice source=slice type=&'static readonly Slice
/// @resolution.pattern source=slice kind=binding target=slice
/// @resolution.name source=Slice target=Slice

const fromBuilder = Path.from(builder);
/// @type.symbol symbol=fromBuilder source=fromBuilder type=Path
/// @resolution.pattern source=fromBuilder kind=binding target=fromBuilder
/// @resolution.name source=Path target=Path
/// @resolution.member source=Path.from receiver=Path type=<from#1.'a>(&from#1.'a readonly Slice) => Path & (Builder) => Path kind=overload-set targets=[from#1, from#2]
/// @resolution.call source=Path.from(builder) parameters=(Builder) arguments=(provided(builder) as Builder) return=Path kind=symbol target=from#2
/// @resolution.name source=builder target=builder
/// @resolution.place source=builder placement="local" lifetime="static" access="immutable"
/// @resolution.access source=builder root=builder

const fromSlice = Path.from(slice);
/// @type.symbol symbol=fromSlice source=fromSlice type=Path
/// @resolution.pattern source=fromSlice kind=binding target=fromSlice
/// @resolution.name source=Path target=Path
/// @resolution.member source=Path.from receiver=Path type=<from#1.'a>(&from#1.'a readonly Slice) => Path & (Builder) => Path kind=overload-set targets=[from#1, from#2]
/// @resolution.call source=Path.from(slice) parameters=(&'static readonly Slice) arguments=(provided(slice) as &'static readonly Slice) return=Path regions=("static" & "local") kind=symbol target=from#1 instance="Path.<extension#1>.from#1<\"static\" & \"local\">"
/// @generic.instantiation id="from#1<\"static\" & \"local\">" template=from#1 arguments=("static" & "local")
/// @resolution.name source=slice target=slice
/// @resolution.place source=slice placement="local" lifetime="static" access="immutable"
/// @resolution.access source=slice root=slice
"#,
        r#"
"#,
    );
}
