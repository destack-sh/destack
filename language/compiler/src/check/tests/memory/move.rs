use crate::tests::{DirRows, TestSession};

#[test]
fn test_reject_use_of_a_moved_owned_value() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32 = 0;
}

function consume(counter: ^Counter): int32 {
    const taken = counter;
    return counter.count;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    count: int32 = 0;
}

function consume(counter: ^Counter): int32 {
    const taken: ^Counter = counter;
    return counter.count;
}

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.count source="count: int32 = 0" key=count type=int32

    count: int32 = 0;
    /// @type.symbol symbol=Counter.count source="count: int32 = 0" type=int32

}

function consume(counter: ^Counter): int32 {
/// @type.symbol symbol=consume type=(Owned<Counter>) => int32
/// @type.symbol symbol=consume.counter source="counter: ^Counter" type=Owned<Counter>
/// @resolution.name source=Counter target=Counter

    const taken = counter;
    /// @type.symbol symbol=consume.taken source=taken type=Owned<Counter>
    /// @resolution.pattern source=taken kind=binding target=consume.taken
    /// @resolution.name source=counter target=consume.counter
    /// @resolution.access source=counter root=consume.counter

    return counter.count;
    /// @resolution.name source=counter target=consume.counter
    /// @resolution.member source=counter.count receiver=Owned<Counter> type=int32 kind=field target_receiver=Owned<Counter> key=count target=Counter.count target_type=int32
    /// @resolution.place source=counter placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=counter root=consume.counter
    /// @resolution.place source=counter.count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=counter.count root=consume.counter keys=[count]

}
"#,
        r#"
/// @diagnostic.error id=use-after-moved message="'counter' is used after being moved"
/// @diagnostic.label line=8 column=12 span="counter" line_source="return counter.count;"
/// @diagnostic.help message="reassign the binding before this use, or copy instead of moving"
"#,
    );
}

#[test]
fn test_read_a_moved_copyable_value_from_a_copy() {
    let session = TestSession::single(
        r#"
function double(value: int32): int32 {
    const taken = value;
    return value + taken;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function double(value: int32): int32 {
    const taken: int32 = value;
    return value + taken;
}

=== checked ===
function double(value: int32): int32 {
/// @type.symbol symbol=double type=(int32) => int32
/// @type.symbol symbol=double.value source="value: int32" type=int32

    const taken = value;
    /// @type.symbol symbol=double.taken source=taken type=int32
    /// @resolution.pattern source=taken kind=binding target=double.taken
    /// @resolution.name source=value target=double.value
    /// @resolution.access source=value root=double.value

    return value + taken;
    /// @resolution.name source=value target=double.value
    /// @resolution.operator source="value + taken" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), taken as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=double.value
    /// @resolution.name source=taken target=double.taken
    /// @resolution.place source=taken placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=taken root=double.taken

}
"#,
    );
}

#[test]
fn test_revive_a_moved_binding_through_reassignment() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32 = 0;
}

function recycle(): int32 {
    let counter = new Counter();
    const taken = counter;
    counter = new Counter();
    return counter.count;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    count: int32 = 0;
}

function recycle(): int32 {
    let counter: Counter = new Counter();
    const taken: Counter = counter;
    counter = new Counter();
    return counter.count;
}

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.count source="count: int32 = 0" key=count type=int32

    count: int32 = 0;
    /// @type.symbol symbol=Counter.count source="count: int32 = 0" type=int32

}

function recycle(): int32 {
/// @type.symbol symbol=recycle type=() => int32

    let counter = new Counter();
    /// @type.symbol symbol=recycle.counter source=counter type=Counter
    /// @resolution.pattern source=counter kind=binding target=recycle.counter
    /// @resolution.construct source="new Counter()" parameters=() return=Counter kind=class target=Counter constructor=default
    /// @resolution.name source=Counter target=Counter

    const taken = counter;
    /// @type.symbol symbol=recycle.taken source=taken type=Counter
    /// @resolution.pattern source=taken kind=binding target=recycle.taken
    /// @resolution.name source=counter target=recycle.counter
    /// @resolution.access source=counter root=recycle.counter

    counter = new Counter();
    /// @resolution.pattern.assign source=counter kind=place
    /// @resolution.assignment source=counter write=binding(recycle.counter) type=Counter
    /// @resolution.construct source="new Counter()" parameters=() return=Counter kind=class target=Counter constructor=default
    /// @resolution.name source=Counter target=Counter

    return counter.count;
    /// @resolution.name source=counter target=recycle.counter
    /// @resolution.member source=counter.count receiver=Counter type=int32 kind=field target_receiver=Counter key=count target=Counter.count target_type=int32
    /// @resolution.place source=counter placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=counter root=recycle.counter
    /// @resolution.place source=counter.count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=counter.count root=recycle.counter keys=[count]

}
"#,
    );
}

#[test]
fn test_reject_use_after_a_move_in_one_branch() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32 = 0;
}

function gamble(flag: boolean, counter: ^Counter): int32 {
    if (flag) {
        const taken = counter;
    }
    return counter.count;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    count: int32 = 0;
}

function gamble(flag: boolean, counter: ^Counter): int32 {
    if (flag) {
        const taken: ^Counter = counter;
    }
    return counter.count;
}

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.count source="count: int32 = 0" key=count type=int32

    count: int32 = 0;
    /// @type.symbol symbol=Counter.count source="count: int32 = 0" type=int32

}

function gamble(flag: boolean, counter: ^Counter): int32 {
/// @type.symbol symbol=gamble type=(boolean, Owned<Counter>) => int32
/// @type.symbol symbol=gamble.flag source="flag: boolean" type=boolean
/// @type.symbol symbol=gamble.counter source="counter: ^Counter" type=Owned<Counter>
/// @resolution.name source=Counter target=Counter

    if (flag) {
    /// @resolution.name source=flag target=gamble.flag
    /// @resolution.place source=flag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=flag root=gamble.flag

        const taken = counter;
        /// @type.symbol symbol=gamble.taken source=taken type=Owned<Counter>
        /// @resolution.pattern source=taken kind=binding target=gamble.taken
        /// @resolution.name source=counter target=gamble.counter
        /// @resolution.access source=counter root=gamble.counter

    }
    return counter.count;
    /// @resolution.name source=counter target=gamble.counter
    /// @resolution.member source=counter.count receiver=Owned<Counter> type=int32 kind=field target_receiver=Owned<Counter> key=count target=Counter.count target_type=int32
    /// @resolution.place source=counter placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=counter root=gamble.counter
    /// @resolution.place source=counter.count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=counter.count root=gamble.counter keys=[count]

}
"#,
        r#"
/// @diagnostic.error id=use-after-moved message="'counter' is used after being moved"
/// @diagnostic.label line=10 column=12 span="counter" line_source="return counter.count;"
/// @diagnostic.help message="reassign the binding before this use, or copy instead of moving"
"#,
    );
}
