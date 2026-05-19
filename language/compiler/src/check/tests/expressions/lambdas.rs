use super::super::snapshot::assert_check_snapshot;

#[test]
fn test_check_records_lambda_captures() {
    assert_check_snapshot(
        r#"
let count = 1;
const next = () => count + 1;
"#,
        r#"
let count = 1;
/// @type.symbol key=count value=int32

const next = () => count + 1;
/// @type.symbol key=next value=() => int32
/// @resolution.name source=count target=count
/// @type.node source="count + 1" value=int32
/// @capture.function key=next bindings=1
/// @capture.binding key=next symbol=count mode=borrow

/// @type.summary types=3 nodes=1 symbols=2
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=1 labels=0 members=0 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=1 bindings=1 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}

#[test]
fn test_check_records_capture_directives() {
    assert_check_snapshot(
        r#"
let count = 1;
let step = 2;

@capture({
    default: "borrow",
    step: "copy",
})
const next = () => count + step;
"#,
        r#"
let count = 1;
/// @type.symbol key=count value=int32

let step = 2;
/// @type.symbol key=step value=int32

@capture({
    default: "borrow",
    step: "copy",
})
const next = () => count + step;
/// @type.symbol key=next value=() => int32
/// @resolution.name source=count target=count
/// @resolution.name source=step target=step
/// @type.node source="count + step" value=int32
/// @capture.function key=next bindings=2
/// @capture.binding key=next symbol=count mode=borrow
/// @capture.binding key=next symbol=step mode=copy
/// @capture.directive key=next default=borrow rules=1
/// @capture.rule key=next binding=step mode=copy

/// @type.summary types=3 nodes=1 symbols=3
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=2 labels=0 members=0 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=1 bindings=2 directives=1 rules=1
/// @layout.summary layouts=0 types=0
"#,
    );
}

#[test]
fn test_check_records_lexical_this_capture() {
    assert_check_snapshot(
        r#"
class Counter {
    value: int32;

    make(): () => int32 {
        return () => this.value;
    }
}
"#,
        r#"
class Counter {
/// @type.symbol key=Counter value=Counter

    value: int32;
    /// @type.symbol key=Counter.value value=int32

    make(): () => int32 {
    /// @type.symbol key=Counter.make value=(this: Counter) => () => int32

        return () => this.value;
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Counter kind=direct target=Counter.value
        /// @type.node source=this.value value=int32
        /// @capture.function key=Counter.make.<lambda0> bindings=0 this=this:borrow
    }
}

/// @type.summary types=3 nodes=1 symbols=3
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=1 labels=0 members=1 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=1 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}
