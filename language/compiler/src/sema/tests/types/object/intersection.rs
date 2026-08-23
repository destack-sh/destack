use crate::tests::{DirRows, TestSession};

/// Reduce a written intersection inside one union arm at interpretation.
#[test]
fn test_reduce_intersection_inside_union_arm() {
    let session = TestSession::single(
        r#"
type Unit = "year" | "month";

type Round =
    | Unit
    | ({ smallest: Unit } | { largest: Unit }) & { mode?: int32 };

export function pick(value: Round): int32 {
    return 1;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Unit = "year" | "month";

type Round = Unit | ({ smallest: Unit } | { largest: Unit }) & { mode?: int32 };

export function pick(
    value: Unit | { smallest: Unit; mode?: int32 } | { largest: Unit; mode?: int32 },
): int32 {
    return 1;
}

=== dir ===
type Unit = "year" | "month";
/// @type.symbol symbol=Unit source="type Unit = \"year\" | \"month\"" type="year" | "month"
/// @definition.type symbol=Unit source="type Unit = \"year\" | \"month\"" value="year" | "month"

type Round =
/// @type.symbol symbol=Round type=Unit | { smallest: Unit; mode?: int32 } | { largest: Unit; mode?: int32 }
/// @definition.type symbol=Round value=Unit | { smallest: Unit; mode?: int32 } | { largest: Unit; mode?: int32 }

    | Unit
    /// @resolution.name source=Unit target=Unit

    | ({ smallest: Unit } | { largest: Unit }) & { mode?: int32 };
    /// @resolution.name source=Unit target=Unit
    /// @resolution.name source=Unit target=Unit

export function pick(value: Round): int32 {
/// @type.symbol symbol=pick type=(Round) => int32
/// @type.symbol symbol=pick.value source="value: Round" type=Unit | { smallest: Unit; mode?: int32 } | { largest: Unit; mode?: int32 }
/// @resolution.name source=Round target=Round

    return 1;
}
"#,
    );
}

/// Reduce an intersection of imported alias rows at elaboration.
#[test]
fn test_reduce_imported_alias_intersection() {
    let session = TestSession::builder()
        .module(
            "options.ds",
            r#"
export type Precision = {
    digits?: int32;
};

export type Calendar = {
    calendarName?: int32;
};
"#,
        )
        .module(
            "main.ds",
            r#"
import { Calendar, Precision } from "./options.ds";

export type Both = Precision & Calendar;

export function pick(value: Both): int32 {
    return 1;
}
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Calendar, Precision } from "./options.ds";

export type Both = Precision & Calendar;

export function pick(value: { digits?: int32; calendarName?: int32 }): int32 {
    return 1;
}

=== dir ===
import { Calendar, Precision } from "./options.ds";

export type Both = Precision & Calendar;
/// @type.symbol symbol=Both source="export type Both = Precision & Calendar" type={ digits?: int32; calendarName?: int32 }
/// @definition.type symbol=Both source="export type Both = Precision & Calendar" value={ digits?: int32; calendarName?: int32 }
/// @resolution.name source=Precision target=options.Precision
/// @resolution.name source=Calendar target=options.Calendar

export function pick(value: Both): int32 {
/// @type.symbol symbol=pick type=(Both) => int32
/// @type.symbol symbol=pick.value source="value: Both" type={ digits?: int32; calendarName?: int32 }
/// @resolution.name source=Both target=Both

    return 1;
}
"#,
    );
}

/// Reduce an intersection over an alias imported through a module cycle.
#[test]
fn test_reduce_cyclic_alias_intersection() {
    let session = TestSession::builder()
        .module(
            "plain.ds",
            r#"
import { Zoned } from "./zoned.ds";

export type PlainLike = {
    day?: int32;
};

export declare function toZoned(value: PlainLike): Zoned;
"#,
        )
        .module(
            "zoned.ds",
            r#"
import { PlainLike } from "./plain.ds";

export type ZonedLike = PlainLike & {
    offset?: int32;
};

export class Zoned {
    static from(value: ZonedLike): Zoned {
        return new Zoned();
    }
}
"#,
        )
        .build();

    session.assert_dir(
        "zoned.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { PlainLike } from "./plain.ds";

export type ZonedLike = PlainLike & {
    offset?: int32;
};

export class Zoned {
    static from(value: { day?: int32; offset?: int32 }): Zoned {
        return new Zoned();
    }
}

=== dir ===
import { PlainLike } from "./plain.ds";

export type ZonedLike = PlainLike & {
/// @type.symbol symbol=ZonedLike type={ day?: int32; offset?: int32 }
/// @definition.type symbol=ZonedLike value={ day?: int32; offset?: int32 }
/// @resolution.name source=PlainLike target=plain.PlainLike

    offset?: int32;
};

export class Zoned {
/// @type.symbol symbol=Zoned type=Zoned
/// @definition.class symbol=Zoned
/// @definition.method symbol=Zoned.from slot=from static=true type=(ZonedLike) => Zoned

    static from(value: ZonedLike): Zoned {
    /// @type.symbol symbol=Zoned.from type=(ZonedLike) => Zoned
    /// @type.symbol symbol=Zoned.from.value source="value: ZonedLike" type={ day?: int32; offset?: int32 }
    /// @resolution.name source=ZonedLike target=ZonedLike
    /// @resolution.name source=Zoned target=Zoned

        return new Zoned();
        /// @resolution.construct source="new Zoned()" parameters=() return=Zoned kind=class target=Zoned constructor=default
        /// @resolution.name source=Zoned target=Zoned

    }
}
"#,
    );
}

/// Annihilate an intersection of unrelated classes to never.
#[test]
fn test_annihilate_unrelated_class_intersection() {
    let session = TestSession::single(
        r#"
class Zoned {
    offset: int32 = 0;
}

class Plain {
    day: int32 = 0;
}

type Narrowed = (Zoned | Plain) & Plain;

export declare function pick(value: Narrowed): int32;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Zoned {
    offset: int32 = 0;
}

class Plain {
    day: int32 = 0;
}

type Narrowed = (Zoned | Plain) & Plain;

export declare function pick(value: Plain): int32;

=== dir ===
class Zoned {
/// @type.symbol symbol=Zoned type=Zoned
/// @definition.class symbol=Zoned
/// @definition.field symbol=Zoned.offset source="offset: int32 = 0" key=offset type=int32

    offset: int32 = 0;
    /// @type.symbol symbol=Zoned.offset source="offset: int32 = 0" type=int32

}

class Plain {
/// @type.symbol symbol=Plain type=Plain
/// @definition.class symbol=Plain
/// @definition.field symbol=Plain.day source="day: int32 = 0" key=day type=int32

    day: int32 = 0;
    /// @type.symbol symbol=Plain.day source="day: int32 = 0" type=int32

}

type Narrowed = (Zoned | Plain) & Plain;
/// @type.symbol symbol=Narrowed source="type Narrowed = (Zoned | Plain) & Plain" type=Plain
/// @definition.type symbol=Narrowed source="type Narrowed = (Zoned | Plain) & Plain" value=Plain
/// @resolution.name source=Zoned target=Zoned
/// @resolution.name source=Plain target=Plain
/// @resolution.name source=Plain target=Plain

export declare function pick(value: Narrowed): int32;
/// @type.symbol symbol=pick source="export declare function pick(value: Narrowed): int32" type=(Narrowed) => int32
/// @type.symbol symbol=pick.value source="value: Narrowed" type=Plain
/// @resolution.name source=Narrowed target=Narrowed
"#,
    );
}
