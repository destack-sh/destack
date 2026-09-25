use crate::tests::{DirRows, TestSession};

/// Type aliases supply types, including through imports, but no expression value.
#[test]
fn test_reference_type_aliases_as_values() {
    let session = TestSession::builder()
        .module(
            "types.tspp",
            r#"
export type Count = int32;
"#,
        )
        .module(
            "namespace.tspp",
            r#"
export { Count } from "./types.tspp";
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Count } from "./types.tspp";
import * as types from "./namespace.tspp";

type LocalCount = int32;

const local = LocalCount;

const imported = Count;

const qualified = types.Count;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import * as types from "./namespace.tspp";
import { Count } from "./types.tspp";

type LocalCount = int32;

const local = LocalCount;

const imported = Count;

const qualified = types.Count;

=== dir ===
import { Count } from "./types.tspp";
import * as types from "./namespace.tspp";

type LocalCount = int32;
/// @type.symbol symbol=LocalCount source="type LocalCount = int32" type=int32
/// @definition.type symbol=LocalCount source="type LocalCount = int32" value=int32

const local = LocalCount;
/// @type.symbol symbol=local source=local type=<error>
/// @resolution.pattern source=local kind=binding target=local
/// @resolution.name source=LocalCount target=LocalCount

const imported = Count;
/// @type.symbol symbol=imported source=imported type=<error>
/// @resolution.pattern source=imported kind=binding target=imported
/// @resolution.name source=Count target=types.Count

const qualified = types.Count;
/// @type.symbol symbol=qualified source=qualified type=<error>
/// @resolution.pattern source=qualified kind=binding target=qualified
/// @resolution.name source=types.Count target=types.Count
"#,
        r#"
/// @diagnostic.error id=invalid-value-reference message="'LocalCount' is not a value"
/// @diagnostic.label line=7 column=15 span="LocalCount" line_source="const local = LocalCount;"
/// @diagnostic.error id=invalid-value-reference message="'Count' is not a value"
/// @diagnostic.label line=9 column=18 span="Count" line_source="const imported = Count;"
/// @diagnostic.error id=invalid-value-reference message="'Count' is not a value"
/// @diagnostic.label line=11 column=25 span="Count" line_source="const qualified = types.Count;"
"#,
    );
}

#[test]
fn test_name_expression_resolves_local_binding() {
    let session = TestSession::single(
        r#"
const value = 1;
const copy = value;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 1 = 1;
const copy: 1 = value;

=== dir ===
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

const copy = value;
/// @type.symbol symbol=copy source=copy type=1
/// @resolution.pattern source=copy kind=binding target=copy
/// @type.node source=value type=1
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

#[test]
fn test_misspelled_reference_suggests_a_reviewed_rename() {
    let session = TestSession::single(
        r#"
const value = 1;
const copy = valeu;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const value: 1 = 1;
const copy = valeu;

=== dir ===
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value

const copy = valeu;
/// @type.symbol symbol=copy source=copy type=<error>
/// @resolution.pattern source=copy kind=binding target=copy
/// @resolution.unresolved source=valeu path=valeu
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'valeu'; did you mean 'value'?"
/// @diagnostic.label line=3 column=14 span="valeu" line_source="const copy = valeu;"
/// @diagnostic.suggestion message="rename to 'value'" applicability=dangerous patched="const copy = value;"
"#,
    );
}

#[test]
fn test_case_mismatched_reference_suggests_an_automatic_rename() {
    let session = TestSession::single(
        r#"
const JSON = 1;
const copy = json;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const JSON: 1 = 1;
const copy = json;

=== dir ===
const JSON = 1;
/// @type.symbol symbol=JSON source=JSON type=1
/// @resolution.pattern source=JSON kind=binding target=JSON

const copy = json;
/// @type.symbol symbol=copy source=copy type=<error>
/// @resolution.pattern source=copy kind=binding target=copy
/// @resolution.unresolved source=json path=json
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'json'; did you mean 'JSON'?"
/// @diagnostic.label line=3 column=14 span="json" line_source="const copy = json;"
/// @diagnostic.suggestion message="rename to 'JSON'" applicability=automatic patched="const copy = JSON;"
"#,
    );
}

#[test]
fn test_redundant_cast_suggests_removal() {
    let session = TestSession::single(
        r#"
const value: int32 = 1;
const same = value as int32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const value: int32 = 1;
const same: int32 = value as int32;

=== dir ===
const value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value

const same = value as int32;
/// @type.symbol symbol=same source=same type=int32
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.warning id=redundant-cast message="cast to 'int32' has no effect"
/// @diagnostic.label line=3 column=20 span="as" line_source="const same = value as int32;"
/// @diagnostic.suggestion message="remove the cast" applicability=automatic patched="const same = value;"
"#,
    );
}

#[test]
fn test_misspelled_member_suggests_a_rename() {
    let session = TestSession::single(
        r#"
struct Point {
    length: int32;
}

declare const point: Point;
const size = point.lenght;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    length: int32;
}

declare const point: Point;
const size = point.lenght;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.length source="length: int32" key=length type=int32

    length: int32;
    /// @type.symbol symbol=Point.length source="length: int32" type=int32

}

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

const size = point.lenght;
/// @type.symbol symbol=size source=size type=<error>
/// @resolution.pattern source=size kind=binding target=size
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
/// @resolution.rejected source=point.lenght
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'lenght' does not exist on type 'Point'; did you mean 'length'?"
/// @diagnostic.label line=7 column=20 span="lenght" line_source="const size = point.lenght;"
/// @diagnostic.suggestion message="rename to 'length'" applicability=dangerous patched="const size = point.length;"
"#,
    );
}
