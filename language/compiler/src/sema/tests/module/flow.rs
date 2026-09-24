use crate::tests::{DirRows, TestSession};

#[test]
fn test_record_flow_conclusions_for_a_checked_module() {
    let session = TestSession::single(
        r#"
function run(): int32 {
    let counted = 1;
    counted = 2;
    const idle = 3;
    const observe = () => counted;
    return counted;
    counted;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_flows(),
        r#"
=== annotated ===
function run(): int32 {
    let counted: int32 = 1;
    counted = 2;
    const idle: 3 = 3;
    const observe: () => int32 = (): int32 => counted;
    return counted;
    counted;
}

=== dir ===
function run(): int32 {
    let counted = 1;
    /// @flow.use symbol=counted uses=read+written+captured

    counted = 2;
    /// @flow.access source=counted root=run.counted uses=written

    const idle = 3;
    const observe = () => counted;
    /// @flow.access source=counted root=run.counted uses=read

    return counted;
    /// @flow.diverging source="return counted"
    /// @flow.access source=counted root=run.counted uses=read

    counted;
    /// @flow.unreachable source=counted
    /// @flow.access source=counted root=run.counted uses=read

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_record_field_and_foreign_uses_in_the_flow_segment() {
    let session = TestSession::builder()
        .module(
            "math.ds",
            r#"
export function max(left: int32, right: int32): int32 {
    return left > right ? left : right;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { max } from "./math.ds";

struct Point {
    x: int32;
    y: int32;
}

function shift(point: &Point): int32 {
    point.x = point.x + 1;

    return max(point.x, point.y);
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_flows(),
        r#"
=== annotated ===
import { max } from "./math.ds";

struct Point {
    x: int32;
    y: int32;
}

function shift<'a>(point: &'a Point): int32 {
    point.x = point.x + 1;

    return max(point.x, point.y);
}

=== dir ===
import { max } from "./math.ds";

struct Point {
/// @flow.use symbol=Point uses=read

    x: int32;
    /// @flow.use symbol=x uses=read+written

    y: int32;
    /// @flow.use symbol=y uses=read

}

function shift(point: &Point): int32 {
/// @flow.use symbol=point uses=read+mutated

    point.x = point.x + 1;
    /// @flow.access source=point root=shift.point uses=read
    /// @flow.access source=point.x root=shift.point keys=[x] uses=written+mutated
    /// @flow.access source=point root=shift.point uses=read
    /// @flow.access source=point.x root=shift.point keys=[x] uses=read

    return max(point.x, point.y);
    /// @flow.diverging source="return max(point.x, point.y)"
    /// @flow.access source=point root=shift.point uses=read
    /// @flow.access source=point.x root=shift.point keys=[x] uses=read
    /// @flow.access source=point root=shift.point uses=read
    /// @flow.access source=point.y root=shift.point keys=[y] uses=read

}

/// @flow.foreign symbol=math.max uses=read
"#,
        r#"
"#,
    );
}

#[test]
fn test_record_mutable_binding_storage_uses() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    increment(&this): void {
        this.value += 1;
    }
}

class Service {
    value: int32 = 0;

    increment(&this): void {
        this.value += 1;
    }
}

declare function modify(value: &int32): void;

let borrowed: int32 = 0;
&borrowed;

let passed: int32 = 0;
modify(passed);

let field: Counter = Counter { value: 0 };
field.value = 1;

let called: Counter = Counter { value: 0 };
called.increment();

let referenced: Service = new Service();
referenced.increment();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_flows(),
        r#"
=== annotated ===
struct Counter {
    value: int32;

    increment(&this): void {
        this.value += 1;
    }
}

class Service {
    value: int32 = 0;

    increment(&this): void {
        this.value += 1;
    }
}

declare function modify<'a>(value: &int32): void;

let borrowed: int32 = 0;
&borrowed;

let passed: int32 = 0;
modify<"static">(passed as &'static int32);

let field: Counter = Counter { value: 0 };
field.value = 1;

let called: Counter = Counter { value: 0 };
called.increment<"static">();

let referenced: Service = new Service();
referenced.increment<"managed">();

=== dir ===
struct Counter {
/// @flow.use symbol=Counter uses=read

    value: int32;
    /// @flow.use symbol=value#1 uses=read+written

    increment(&this): void {
    /// @flow.use symbol=increment#1 uses=read

        this.value += 1;
        /// @flow.access source=this.value root=this keys=[value] uses=written+mutated

    }
}

class Service {
/// @flow.use symbol=Service uses=read

    value: int32 = 0;
    /// @flow.use symbol=value#2 uses=read+written

    increment(&this): void {
    /// @flow.use symbol=increment#2 uses=read

        this.value += 1;
        /// @flow.access source=this.value root=this keys=[value] uses=written+mutated

    }
}

declare function modify(value: &int32): void;
/// @flow.use symbol=modify uses=read

let borrowed: int32 = 0;
/// @flow.use symbol=borrowed uses=read+mutated+mutable

&borrowed;
/// @flow.access source=borrowed root=borrowed uses=read+mutated+mutable

let passed: int32 = 0;
/// @flow.use symbol=passed uses=read+mutated+mutable

modify(passed);
/// @flow.access source=passed root=passed uses=read+mutated+mutable

let field: Counter = Counter { value: 0 };
/// @flow.use symbol=field uses=read+mutated

field.value = 1;
/// @flow.access source=field root=field uses=read
/// @flow.access source=field.value root=field keys=[value] uses=written+mutated

let called: Counter = Counter { value: 0 };
/// @flow.use symbol=called uses=read+mutated+mutable

called.increment();
/// @flow.access source=called root=called uses=read+mutated+mutable

let referenced: Service = new Service();
/// @flow.use symbol=referenced uses=read+mutable

referenced.increment();
/// @flow.access source=referenced root=referenced uses=read+mutable
"#,
        r#"
"#,
    );
}

#[test]
fn test_record_unreachable_branches_in_the_flow_segment() {
    let session = TestSession::single(
        r#"
function pick(flag: boolean): int32 {
    if (flag) {
        return 1;
        flag;
    }

    loop {
        break;
    }

    return 2;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_flows(),
        r#"
=== annotated ===
function pick(flag: boolean): int32 {
    if (flag) {
        return 1;
        flag;
    }

    loop {
        break;
    }

    return 2;
}

=== dir ===
function pick(flag: boolean): int32 {
/// @flow.use symbol=flag uses=read

    if (flag) {
    /// @flow.access source=flag root=pick.flag uses=read

        return 1;
        /// @flow.diverging source="return 1"

        flag;
        /// @flow.unreachable source=flag
        /// @flow.access source=flag root=pick.flag uses=read

    }

    loop {
    /// @flow.single_pass

        break;
        /// @flow.diverging source=break

    }

    return 2;
    /// @flow.diverging source="return 2"

}
"#,
        r#"
"#,
    );
}

/// Loops whose body never reaches another iteration record the single-pass fact.
#[test]
fn test_record_single_pass_loops_in_the_flow_segment() {
    let session = TestSession::single(
        r#"
function consume(limit: int32): int32 {
    let end = limit;
    for (const value of 0..end) {
        end -= value;
        break;
    }

    while (end > 0) {
        end -= 1;
    }

    return end;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none().with_flows(),
        r#"
=== annotated ===
function consume(limit: int32): int32 {
    let end: int32 = limit;
    for (const value of 0..end) {
        end -= value;
        break;
    }

    while (end > 0) {
        end -= 1;
    }

    return end;
}

=== dir ===
function consume(limit: int32): int32 {
/// @flow.use symbol=limit uses=read

    let end = limit;
    /// @flow.use symbol=end uses=read+written
    /// @flow.access source=limit root=consume.limit uses=read

    for (const value of 0..end) {
    /// @flow.single_pass
    /// @flow.use symbol=value uses=read
    /// @flow.access source=end root=consume.end uses=read

        end -= value;
        /// @flow.access source=end root=consume.end uses=written
        /// @flow.access source=value root=consume.value uses=read

        break;
        /// @flow.diverging source=break

    }

    while (end > 0) {
    /// @flow.access source=end root=consume.end uses=read

        end -= 1;
        /// @flow.access source=end root=consume.end uses=written

    }

    return end;
    /// @flow.diverging source="return end"
    /// @flow.access source=end root=consume.end uses=read

}
"#,
        r#"
"#,
    );
}
