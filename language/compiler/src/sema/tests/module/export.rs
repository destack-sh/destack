use crate::tests::{DirRows, TestSession};

#[test]
fn test_expose_a_reexported_binding_through_a_star_export() {
    let session = TestSession::builder()
        .module(
            "source.tspp",
            r#"
export const value: int32 = 1;
"#,
        )
        .module(
            "index.tspp",
            r#"
export * from "./source.tspp";
"#,
        )
        .module(
            "main.tspp",
            r#"
import { value } from "./index.tspp";

const direct = value;
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { value } from "./index.tspp";

const direct: int32 = value;

=== dir ===
import { value } from "./index.tspp";

const direct = value;
/// @type.symbol symbol=direct source=direct type=int32
/// @resolution.pattern source=direct kind=binding target=direct
/// @type.node source=value type=int32
/// @resolution.name source=value target=source.value
/// @resolution.access source=value root=source.value
"#,
    );
}

#[test]
fn test_expose_a_member_binding_through_a_namespace_export() {
    let session = TestSession::builder()
        .module(
            "source.tspp",
            r#"
export const value: int32 = 1;
"#,
        )
        .module(
            "index.tspp",
            r#"
export * as source from "./source.tspp";
"#,
        )
        .module(
            "main.tspp",
            r#"
import { source } from "./index.tspp";

const namespaced = source.value;
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { source } from "./index.tspp";

const namespaced: int32 = source.value;

=== dir ===
import { source } from "./index.tspp";

const namespaced = source.value;
/// @type.symbol symbol=namespaced source=namespaced type=int32
/// @resolution.pattern source=namespaced kind=binding target=namespaced
/// @type.node source=source.value type=int32
/// @resolution.name source=source.value target=source.value
/// @resolution.access source=source.value root=source.value
"#,
    );
}

#[test]
fn test_suggest_import_for_an_unresolved_sibling_declaration() {
    let session = TestSession::builder()
        .module(
            "util.tspp",
            r#"
export const helper: int32 = 1;
export const sibling: int32 = 2;
"#,
        )
        .module(
            "main.tspp",
            r#"
import { helper } from "./util.tspp";

const first = helper;
const second = sibling;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { helper } from "./util.tspp";

const first: int32 = helper;
const second = sibling;

=== dir ===
import { helper } from "./util.tspp";

const first = helper;
/// @type.symbol symbol=first source=first type=int32
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=helper target=util.helper
/// @resolution.access source=helper root=util.helper

const second = sibling;
/// @type.symbol symbol=second source=second type=<error>
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.unresolved source=sibling path=sibling
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'sibling'"
/// @diagnostic.label line=5 column=16 span="sibling" line_source="const second = sibling;"
/// @diagnostic.related file="util.tspp" line=3 column=14 span="sibling" line_source="export const sibling: int32 = 2;" message="'sibling' is declared here"
/// @diagnostic.help message="import 'sibling' from its module"
"#,
    );
}

#[test]
fn test_suggest_import_for_an_unresolved_sibling_type() {
    let session = TestSession::builder()
        .module(
            "util.tspp",
            r#"
export type Helper = string;
export type Sibling = int32;
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Helper } from "./util.tspp";

type First = Helper;
type Second = Sibling;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Helper } from "./util.tspp";

type First = Helper;
type Second = Sibling;

=== dir ===
import { Helper } from "./util.tspp";

type First = Helper;
/// @type.symbol symbol=First source="type First = Helper" type=string
/// @definition.type symbol=First source="type First = Helper" value=util.Helper
/// @resolution.name source=Helper target=util.Helper

type Second = Sibling;
/// @type.symbol symbol=Second source="type Second = Sibling" type=<error>
/// @definition.type symbol=Second source="type Second = Sibling" value=<error>
/// @resolution.unresolved source=Sibling path=Sibling
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'Sibling'"
/// @diagnostic.label line=5 column=15 span="Sibling" line_source="type Second = Sibling;"
/// @diagnostic.related file="util.tspp" line=3 column=13 span="Sibling" line_source="export type Sibling = int32;" message="'Sibling' is declared here"
/// @diagnostic.help message="import 'Sibling' from its module"
"#,
    );
}

/// Export literal initializers without annotations.
#[test]
fn test_export_literal_initializers_without_annotations() {
    let session = TestSession::single(
        r#"
export const flag = true;
export const count = -3;
export const label = `name`;
export const pair = [1, 2];
export const config = { retries: 3, name: "job" };
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
export const flag: true = true;
export const count: -3 = -3;
export const label: "name" = `name`;
export const pair: int64[] = [1, 2];
export const config: { retries: int64; name: string } = { retries: 3, name: "job" };

=== dir ===
export const flag = true;
/// @type.symbol symbol=flag source=flag type=true
/// @resolution.pattern source=flag kind=binding target=flag

export const count = -3;
/// @type.symbol symbol=count source=count type=-3
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.operator source=-3 type=-3 operator="-" kind=builtin operands=[3 as 3 families=(integer)]

export const label = `name`;
/// @type.symbol symbol=label source=label type="name"
/// @resolution.pattern source=label kind=binding target=label

export const pair = [1, 2];
/// @type.symbol symbol=pair source=pair type=int64[]
/// @resolution.pattern source=pair kind=binding target=pair
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)

export const config = { retries: 3, name: "job" };
/// @type.symbol symbol=config source=config type={ retries: int64; name: string }
/// @resolution.pattern source=config kind=binding target=config
"#,
    );
}

/// Reject export initializers that need inference.
#[test]
fn test_reject_export_initializers_needing_inference() {
    let session = TestSession::single(
        r#"
function seed(): int32 {
    return 1;
}

export const computed = seed();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function seed(): int32 {
    return 1;
}

export const computed = seed();

=== dir ===
function seed(): int32 {
/// @type.symbol symbol=seed type=() => int32

    return 1;
}

export const computed = seed();
/// @type.symbol symbol=computed source=computed type=<error>
/// @resolution.pattern source=computed kind=binding target=computed
/// @resolution.name source=seed target=seed
/// @resolution.call source=seed() parameters=() return=int32 kind=symbol target=seed
"#,
        r#"
/// @diagnostic.error id=missing-export-binding-type message="exported binding needs a written type"
/// @diagnostic.label line=6 column=14 span="computed" line_source="export const computed = seed();"
/// @diagnostic.help message="state the type or initialize with a literal"
"#,
    );
}
