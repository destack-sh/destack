use crate::tests::{DirRows, TestSession};

#[test]
fn test_reject_cross_module_duplicate_single_slot_member() {
    let session = TestSession::builder()
        .module(
            "widget.ds",
            r#"
export struct Widget {}

export extension of Widget {
    get size(): usize {
        return 1;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Widget } from "./widget.ds";

extension of Widget {
    get size(): usize {
        return 2;
    }
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { Widget } from "./widget.ds";

extension of Widget {
    get size(): usize {
        return 2;
    }
}

=== dir ===
import { Widget } from "./widget.ds";

extension of Widget {
    get size(): usize {
        return 2;
    }
}
"#,
        r#"
/// @diagnostic.error id=duplicate-member message="member 'size' is already declared for 'Widget' by another visible extension"
/// @diagnostic.label line=5 column=9 span="size" line_source="get size(): usize {"
"#,
    );
}

#[test]
fn test_reject_same_module_repeated_member() {
    let session = TestSession::single(
        r#"
struct Widget {}

extension of Widget {
    get size(): usize {
        return 1;
    }
}

extension of Widget {
    get size(): usize {
        return 2;
    }
}

declare const widget: Widget;
const size = widget.size;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
struct Widget {}

extension of Widget {
    get size(): usize {
        return 1;
    }
}

extension of Widget {
    get size(): usize {
        return 2;
    }
}

declare const widget: Widget;
const size: usize = widget.size;

=== dir ===
struct Widget {}

extension of Widget {
    get size(): usize {
        return 1;
    }
}

extension of Widget {
    get size(): usize {
        return 2;
    }
}

declare const widget: Widget;
const size = widget.size;
"#,
        r#"
/// @diagnostic.error id=ambiguous-member message="member 'size' is ambiguous"
/// @diagnostic.label line=17 column=21 span="size" line_source="const size = widget.size;"
/// @diagnostic.error id=duplicate-member message="member 'size' is already declared for 'Widget' by another visible extension"
/// @diagnostic.label line=11 column=9 span="size" line_source="get size(): usize {"
"#,
    );
}

#[test]
fn test_reject_an_unnamed_exported_extension_of_a_nonlocal_type() {
    let session = TestSession::builder()
        .module(
            "widget.ds",
            r#"
export struct Widget {}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Widget } from "./widget.ds";

export extension of Widget {
    get size(): usize {
        return 1;
    }
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { Widget } from "./widget.ds";

export extension of Widget {
    get size(): usize {
        return 1;
    }
}

=== dir ===
import { Widget } from "./widget.ds";

export extension of Widget {
    get size(): usize {
        return 1;
    }
}
"#,
        r#"
/// @diagnostic.error id=unnamed-exported-nonlocal-extension message="exported extension on nonlocal type 'Widget' must have a name"
/// @diagnostic.label line=4 column=21 span="Widget" line_source="export extension of Widget {"
"#,
    );
}

#[test]
fn test_select_static_members_across_form_qualified_extensions() {
    let session = TestSession::single(
        r#"
import { Set } from "destack:collections";

function build(values: [int32]): void {
    const owned: ^Set<int32> = Set.from([...values]);
    const managed: Set<int32> = Set.from([...values]);
    const mapped = Array.from([...values], (value) => value);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Set } from "destack:collections";

function build(values: [int32]): void {
    const owned: ^Set<int32, Equality<int32>> = Set.from<int32, Equality<int32>>([
        ...values,
    ] as Iterable<int32>);
    const managed: Set<int32, Equality<int32>> = Set.from<int32, Equality<int32>>([
        ...values,
    ] as Iterable<int32>) as Set<int32, Equality<int32>>;
    const mapped: ^int32[] = Array.from<int32, int32>(
        [...values] as Iterable<int32>,
        (value: int32): int32 => value,
    );
}

=== dir ===
import { Set } from "destack:collections";

function build(values: [int32]): void {
/// @type.symbol symbol=build type=(Slice<int32>) => void
/// @type.symbol symbol=build.values source="values: [int32]" type=Slice<int32>

    const owned: ^Set<int32> = Set.from([...values]);
    /// @type.symbol symbol=build.owned source=owned type=^Set<int32, Equality<int32>>
    /// @resolution.pattern source=owned kind=binding target=build.owned
    /// @resolution.name source=Set target=Set
    /// @resolution.name source=Set target=Set
    /// @resolution.member source=Set.from receiver=typeof Set type=(Iterable<T#6>) => ^Set<T#6, E#6> & (Iterable<T#6>, E#6) => ^Set<T#6, E#6> kind=overload-set targets=[from#1, from#2]
    /// @resolution.call source=Set.from([...values]) parameters=(Iterable<int32>) arguments=(provided([...values]) as Iterable<int32>) return=^Set<int32, Equality<int32>> kind=symbol target=from#1 instance="Set<int32, Equality<int32>>.<extension#6>.from#1"
    /// @generic.instantiation id="from#1<int32, Equality<int32>>" template=from#1 arguments=(int32, Equality<int32>)
    /// @resolution.call source=[...values] parameters=(^Slice<int32>) arguments=(rest(spread(provided(...values) as Slice<int32>, iterator=iterator#1(parameters=(), arguments=(), return=Iterator<int32>, regions=("managed" & "local")), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=("managed" & "local"))) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @generic.instantiation id="iterator#1<int32, \"managed\" & \"local\">" template=iterator#1 arguments=(int32, "managed" & "local")
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @resolution.name source=values target=build.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=build.values

    const managed: Set<int32> = Set.from([...values]);
    /// @type.symbol symbol=build.managed source=managed type=Set<int32, Equality<int32>>
    /// @resolution.pattern source=managed kind=binding target=build.managed
    /// @resolution.name source=Set target=Set
    /// @resolution.name source=Set target=Set
    /// @resolution.member source=Set.from receiver=typeof Set type=(Iterable<T#6>) => ^Set<T#6, E#6> & (Iterable<T#6>, E#6) => ^Set<T#6, E#6> kind=overload-set targets=[from#1, from#2]
    /// @resolution.call source=Set.from([...values]) parameters=(Iterable<int32>) arguments=(provided([...values]) as Iterable<int32>) return=^Set<int32, Equality<int32>> kind=symbol target=from#1 instance="Set<int32, Equality<int32>>.<extension#6>.from#1"
    /// @resolution.call source=[...values] parameters=(^Slice<int32>) arguments=(rest(spread(provided(...values) as Slice<int32>, iterator=iterator#1(parameters=(), arguments=(), return=Iterator<int32>, regions=("managed" & "local")), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=("managed" & "local"))) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @resolution.name source=values target=build.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=build.values

    const mapped = Array.from([...values], (value) => value);
    /// @type.symbol symbol=build.mapped source=mapped type=^int32[]
    /// @resolution.pattern source=mapped kind=binding target=build.mapped
    /// @resolution.name source=Array target=Array
    /// @resolution.member source=Array.from receiver=typeof Array type=(Iterable<T#6>) => ^T#6[] & <from.U>(Iterable<from.U>, (from.U, isize) => T#6) => ^T#6[] kind=overload-set targets=[from#1, from#2]
    /// @resolution.call source="Array.from([...values], (value) => value)" parameters=(Iterable<int32>, (int32, isize) => int32) arguments=(provided([...values]) as Iterable<int32>, provided((value) => value) as (int32, isize) => int32) return=^int32[] kind=symbol target=from#2 instance=Array<int32>.<extension#6>.from#2<int32>
    /// @generic.instantiation id="from#2<int32, int32>" template=from#2 arguments=(int32, int32)
    /// @resolution.call source=[...values] parameters=(^Slice<int32>) arguments=(rest(spread(provided(...values) as Slice<int32>, iterator=iterator#1(parameters=(), arguments=(), return=Iterator<int32>, regions=("managed" & "local")), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=("managed" & "local"))) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @resolution.name source=values target=build.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=build.values
    /// @type.symbol symbol=build.symbol6 source="(value) => value" type=Function<(int32,), int32, "readonly">
    /// @type.symbol symbol=build.symbol6.value source=value type=int32
    /// @resolution.name source=value target=build.symbol6.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=build.symbol6.value

}
"#,
        r#"

"#,
    );
}
