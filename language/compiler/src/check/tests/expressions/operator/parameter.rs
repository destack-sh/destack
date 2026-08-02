use crate::tests::{DirRows, TestSession};

#[test]
fn test_arithmetic_on_union_bounded_parameter_keeps_parameter() {
    let session = TestSession::single(
        r#"
function square<T: int32 | float64>(value: T): T {
    return value * value;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function square<T: int32 | float64>(value: T): T {
    return value * value;
}

=== checked ===
function square<T: int32 | float64>(value: T): T {
/// @generic.template symbol=square parameters=(T: int32 | float64)
/// @type.symbol symbol=square type=<T: int32 | float64>(T) => T
/// @type.symbol symbol=square.T source="T: int32 | float64" type=T
/// @type.symbol symbol=square.value source="value: T" type=T
/// @resolution.name source=T target=square.T
/// @resolution.name source=T target=square.T

    return value * value;
    /// @resolution.name source=value target=square.value
    /// @resolution.operator source="value * value" type=T operator="*" kind=builtin operands=[value as T families=(integer | float), value as T families=(integer | float)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=square.value
    /// @resolution.name source=value target=square.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=square.value

}
"#,
    );
}

#[test]
fn test_where_clause_bound_enables_arithmetic() {
    let session = TestSession::single(
        r#"
function scale<T>(left: T, right: T): T where T: int32 | float64 {
    return left * right;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function scale<T>(left: T, right: T): T where T: int32 | float64 {
    return left * right;
}

=== checked ===
function scale<T>(left: T, right: T): T where T: int32 | float64 {
/// @generic.template symbol=scale parameters=(T)
/// @type.symbol symbol=scale type=<T>(T, T) => T
/// @type.symbol symbol=scale.T source=T type=T
/// @type.symbol symbol=scale.left source="left: T" type=T
/// @resolution.name source=T target=scale.T
/// @type.symbol symbol=scale.right source="right: T" type=T
/// @resolution.name source=T target=scale.T
/// @resolution.name source=T target=scale.T
/// @resolution.name source=T target=scale.T

    return left * right;
    /// @resolution.name source=left target=scale.left
    /// @resolution.operator source="left * right" type=T operator="*" kind=builtin operands=[left as T families=(integer | float), right as T families=(integer | float)]
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=scale.left
    /// @resolution.name source=right target=scale.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=scale.right

}
"#,
    );
}

#[test]
fn test_literal_operand_adapts_into_bounded_parameter() {
    let session = TestSession::single(
        r#"
function decrement<T: int32 | float64>(value: T): T {
    return value - 1;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function decrement<T: int32 | float64>(value: T): T {
    return value - 1;
}

=== checked ===
function decrement<T: int32 | float64>(value: T): T {
/// @generic.template symbol=decrement parameters=(T: int32 | float64)
/// @type.symbol symbol=decrement type=<T: int32 | float64>(T) => T
/// @type.symbol symbol=decrement.T source="T: int32 | float64" type=T
/// @type.symbol symbol=decrement.value source="value: T" type=T
/// @resolution.name source=T target=decrement.T
/// @resolution.name source=T target=decrement.T

    return value - 1;
    /// @resolution.name source=value target=decrement.value
    /// @resolution.operator source="value - 1" type=T operator="-" kind=builtin operands=[value as T families=(integer | float), 1 as T families=(integer | float)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=decrement.value

}
"#,
    );
}

#[test]
fn test_literal_operand_must_fit_every_scalar_alternative() {
    let session = TestSession::single(
        r#"
function offset<T: int8 | int64>(value: T): T {
    return value + 128;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function offset<T: int8 | int64>(value: T): T {
    return value + 128;
}

=== checked ===
function offset<T: int8 | int64>(value: T): T {
/// @generic.template symbol=offset parameters=(T: int8 | int64)
/// @type.symbol symbol=offset type=<T: int8 | int64>(T) => T
/// @type.symbol symbol=offset.T source="T: int8 | int64" type=T
/// @type.symbol symbol=offset.value source="value: T" type=T
/// @resolution.name source=T target=offset.T
/// @resolution.name source=T target=offset.T

    return value + 128;
    /// @resolution.name source=value target=offset.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=offset.value

}
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '+' is not defined for 'T' and '128'"
/// @diagnostic.label line=3 column=18 span="+" line_source="return value + 128;"
"#,
    );
}

#[test]
fn test_literal_operand_fits_exact_scalar_constraint() {
    let session = TestSession::single(
        r#"
function offset<T: int64>(value: T): T {
    return value + 1000;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function offset<T: int64>(value: T): T {
    return value + 1000;
}

=== checked ===
function offset<T: int64>(value: T): T {
/// @generic.template symbol=offset parameters=(T: int64)
/// @type.symbol symbol=offset type=<T: int64>(T) => T
/// @type.symbol symbol=offset.T source="T: int64" type=T
/// @type.symbol symbol=offset.value source="value: T" type=T
/// @resolution.name source=T target=offset.T
/// @resolution.name source=T target=offset.T

    return value + 1000;
    /// @resolution.name source=value target=offset.value
    /// @resolution.operator source="value + 1000" type=T operator="+" kind=builtin operands=[value as T families=(integer), 1000 as T families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=offset.value

}
"#,
    );
}

#[test]
fn test_literal_operand_uses_symmetric_equality_constraint() {
    let session = TestSession::single(
        r#"
function offset<T>(value: T): T where int64 == T {
    return value + 1000;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function offset<T>(value: T): T where int64 == T {
    return value + 1000;
}

=== checked ===
function offset<T>(value: T): T where int64 == T {
/// @generic.template symbol=offset parameters=(T)
/// @type.symbol symbol=offset type=<T>(T) => T
/// @type.symbol symbol=offset.T source=T type=T
/// @type.symbol symbol=offset.value source="value: T" type=T
/// @resolution.name source=T target=offset.T
/// @resolution.name source=T target=offset.T
/// @resolution.name source=T target=offset.T

    return value + 1000;
    /// @resolution.name source=value target=offset.value
    /// @resolution.operator source="value + 1000" type=T operator="+" kind=builtin operands=[value as T families=(integer), 1000 as T families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=offset.value

}
"#,
    );
}

#[test]
fn test_literal_operand_uses_intersected_scalar_bound() {
    let session = TestSession::single(
        r#"
function subtract<T: int32 | float64>(value: T): T where T: float64 {
    return value - 0.5;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function subtract<T: int32 | float64>(value: T): T where T: float64 {
    return value - 0.5;
}

=== checked ===
function subtract<T: int32 | float64>(value: T): T where T: float64 {
/// @generic.template symbol=subtract parameters=(T: int32 | float64)
/// @type.symbol symbol=subtract type=<T: int32 | float64>(T) => T
/// @type.symbol symbol=subtract.T source="T: int32 | float64" type=T
/// @type.symbol symbol=subtract.value source="value: T" type=T
/// @resolution.name source=T target=subtract.T
/// @resolution.name source=T target=subtract.T
/// @resolution.name source=T target=subtract.T

    return value - 0.5;
    /// @resolution.name source=value target=subtract.value
    /// @resolution.operator source="value - 0.5" type=T operator="-" kind=builtin operands=[value as T families=(float), 0.5 as T families=(float)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=subtract.value

}
"#,
    );
}

#[test]
fn test_literal_operand_satisfies_conjoined_scalar_constraints() {
    let session = TestSession::single(
        r#"
function offset<T: int8 | float64>(value: T): T where T: uint8 {
    return value + 100;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function offset<T: int8 | float64>(value: T): T where T: uint8 {
    return value + 100;
}

=== checked ===
function offset<T: int8 | float64>(value: T): T where T: uint8 {
/// @generic.template symbol=offset parameters=(T: int8 | float64)
/// @type.symbol symbol=offset type=<T: int8 | float64>(T) => T
/// @type.symbol symbol=offset.T source="T: int8 | float64" type=T
/// @type.symbol symbol=offset.value source="value: T" type=T
/// @resolution.name source=T target=offset.T
/// @resolution.name source=T target=offset.T
/// @resolution.name source=T target=offset.T

    return value + 100;
    /// @resolution.name source=value target=offset.value
    /// @resolution.operator source="value + 100" type=T operator="+" kind=builtin operands=[value as T families=(integer), 100 as T families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=offset.value

}
"#,
    );
}

#[test]
fn test_literal_operand_must_satisfy_conjoined_scalar_constraints() {
    let session = TestSession::single(
        r#"
function offset<T: int8 | float64>(value: T): T where T: uint8 {
    return value + 200;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function offset<T: int8 | float64>(value: T): T where T: uint8 {
    return value + 200;
}

=== checked ===
function offset<T: int8 | float64>(value: T): T where T: uint8 {
/// @generic.template symbol=offset parameters=(T: int8 | float64)
/// @type.symbol symbol=offset type=<T: int8 | float64>(T) => T
/// @type.symbol symbol=offset.T source="T: int8 | float64" type=T
/// @type.symbol symbol=offset.value source="value: T" type=T
/// @resolution.name source=T target=offset.T
/// @resolution.name source=T target=offset.T
/// @resolution.name source=T target=offset.T

    return value + 200;
    /// @resolution.name source=value target=offset.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=offset.value

}
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '+' is not defined for 'T' and '200'"
/// @diagnostic.label line=3 column=18 span="+" line_source="return value + 200;"
"#,
    );
}

#[test]
fn test_comparison_on_bounded_parameter_yields_boolean() {
    let session = TestSession::single(
        r#"
function ordered<T: int32 | float64>(left: T, right: T): boolean {
    return left < right;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function ordered<T: int32 | float64>(left: T, right: T): boolean {
    return left < right;
}

=== checked ===
function ordered<T: int32 | float64>(left: T, right: T): boolean {
/// @generic.template symbol=ordered parameters=(T: int32 | float64)
/// @type.symbol symbol=ordered type=<T: int32 | float64>(T, T) => boolean
/// @type.symbol symbol=ordered.T source="T: int32 | float64" type=T
/// @type.symbol symbol=ordered.left source="left: T" type=T
/// @resolution.name source=T target=ordered.T
/// @type.symbol symbol=ordered.right source="right: T" type=T
/// @resolution.name source=T target=ordered.T

    return left < right;
    /// @resolution.name source=left target=ordered.left
    /// @resolution.operator source="left < right" type=boolean operator="<" kind=builtin operands=[left as T families=(integer | float), right as T families=(integer | float)]
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=ordered.left
    /// @resolution.name source=right target=ordered.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=ordered.right

}
"#,
    );
}

#[test]
fn test_negate_keeps_bounded_parameter() {
    let session = TestSession::single(
        r#"
function negate<T: int32 | float64>(value: T): T {
    return -value;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function negate<T: int32 | float64>(value: T): T {
    return -value;
}

=== checked ===
function negate<T: int32 | float64>(value: T): T {
/// @generic.template symbol=negate parameters=(T: int32 | float64)
/// @type.symbol symbol=negate type=<T: int32 | float64>(T) => T
/// @type.symbol symbol=negate.T source="T: int32 | float64" type=T
/// @type.symbol symbol=negate.value source="value: T" type=T
/// @resolution.name source=T target=negate.T
/// @resolution.name source=T target=negate.T

    return -value;
    /// @resolution.operator source=-value type=T operator="-" kind=builtin operands=[value as T families=(integer | float)]
    /// @resolution.name source=value target=negate.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=negate.value

}
"#,
    );
}

#[test]
fn test_bitwise_not_keeps_integer_bounded_parameter() {
    let session = TestSession::single(
        r#"
function flip<T: int32 | int64>(value: T): T {
    return ~value;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function flip<T: int32 | int64>(value: T): T {
    return ~value;
}

=== checked ===
function flip<T: int32 | int64>(value: T): T {
/// @generic.template symbol=flip parameters=(T: int32 | int64)
/// @type.symbol symbol=flip type=<T: int32 | int64>(T) => T
/// @type.symbol symbol=flip.T source="T: int32 | int64" type=T
/// @type.symbol symbol=flip.value source="value: T" type=T
/// @resolution.name source=T target=flip.T
/// @resolution.name source=T target=flip.T

    return ~value;
    /// @resolution.operator source=~value type=T operator="~" kind=builtin operands=[value as T families=(integer)]
    /// @resolution.name source=value target=flip.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=flip.value

}
"#,
    );
}

#[test]
fn test_mixed_parameters_require_explicit_conversion() {
    let session = TestSession::single(
        r#"
function mix<T: int32 | float64, U: int32 | float64>(left: T, right: U): T {
    return left * right;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function mix<T: int32 | float64, U: int32 | float64>(left: T, right: U): T {
    return left * right;
}

=== checked ===
function mix<T: int32 | float64, U: int32 | float64>(left: T, right: U): T {
/// @generic.template symbol=mix parameters=(T: int32 | float64, U: int32 | float64)
/// @type.symbol symbol=mix type=<T: int32 | float64, U: int32 | float64>(T, U) => T
/// @type.symbol symbol=mix.T source="T: int32 | float64" type=T
/// @type.symbol symbol=mix.U source="U: int32 | float64" type=U
/// @type.symbol symbol=mix.left source="left: T" type=T
/// @resolution.name source=T target=mix.T
/// @type.symbol symbol=mix.right source="right: U" type=U
/// @resolution.name source=U target=mix.U
/// @resolution.name source=T target=mix.T

    return left * right;
    /// @resolution.name source=left target=mix.left
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=mix.left
    /// @resolution.name source=right target=mix.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=mix.right

}
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '*' is not defined for 'T' and 'U'"
/// @diagnostic.label line=3 column=17 span="*" line_source="return left * right;"
"#,
    );
}

#[test]
fn test_unbounded_parameter_rejects_arithmetic() {
    let session = TestSession::single(
        r#"
function double<T>(value: T): T {
    return value + value;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function double<T>(value: T): T {
    return value + value;
}

=== checked ===
function double<T>(value: T): T {
/// @generic.template symbol=double parameters=(T)
/// @type.symbol symbol=double type=<T>(T) => T
/// @type.symbol symbol=double.T source=T type=T
/// @type.symbol symbol=double.value source="value: T" type=T
/// @resolution.name source=T target=double.T
/// @resolution.name source=T target=double.T

    return value + value;
    /// @resolution.name source=value target=double.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=double.value
    /// @resolution.name source=value target=double.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=double.value

}
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '+' is not defined for 'T' and 'T'"
/// @diagnostic.label line=3 column=18 span="+" line_source="return value + value;"
"#,
    );
}

#[test]
fn test_errored_operand_does_not_cascade() {
    let session = TestSession::single(
        r#"
const value = missing * 2;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value = missing * 2;

=== checked ===
const value = missing * 2;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.unresolved source=missing path=missing
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'missing'"
/// @diagnostic.label line=2 column=15 span="missing" line_source="const value = missing * 2;"
"#,
    );
}

#[test]
fn test_arithmetic_on_matching_union_operands_keeps_union() {
    let session = TestSession::single(
        r#"
declare const value: int32 | float64;
const doubled = value * value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const value: int32 | float64;
const doubled: int32 | float64 = value * value;

=== checked ===
declare const value: int32 | float64;
/// @type.symbol symbol=value source=value type=int32 | float64
/// @resolution.pattern source=value kind=binding target=value

const doubled = value * value;
/// @type.symbol symbol=doubled source=doubled type=int32 | float64
/// @resolution.pattern source=doubled kind=binding target=doubled
/// @resolution.name source=value target=value
/// @resolution.operator source="value * value" type=int32 | float64 operator="*" kind=builtin operands=[value as int32 | float64 families=(integer | float), value as int32 | float64 families=(integer | float)]
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
"#,
    );
}

#[test]
fn test_bounded_parameter_selects_overloaded_operator() {
    let session = TestSession::single(
        r#"
import { Multiply } from "destack:ops";

function square<T: Multiply<T>>(value: T): T.Output {
    return value * value;
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Multiply } from "destack:ops";

function square<T: Multiply<T>>(value: T): T.Output {
    return value * value;
}

=== checked ===
import { Multiply } from "destack:ops";

function square<T: Multiply<T>>(value: T): T.Output {
/// @generic.template symbol=square parameters=(T: Multiply<T>)
/// @type.symbol symbol=square type=<T: Multiply<T>>(T) => T.Output
/// @type.symbol symbol=square.T source="T: Multiply<T>" type=T
/// @resolution.name source=Multiply target=ops.multiply.Multiply
/// @resolution.name source=T target=square.T
/// @type.symbol symbol=square.value source="value: T" type=T
/// @resolution.name source=T target=square.T
/// @resolution.name source=T.Output target=square.T

    return value * value;
    /// @resolution.name source=value target=square.value
    /// @resolution.operator source="value * value" type=T.Output operator="*" kind=call parameters=(T) arguments=(provided(value) as T) return=T.Output kind=symbol target=ops.multiply.Multiply.multiply receiver=T instance=Multiply<T>.multiply
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=square.value
    /// @generic.instance source="value * value" id=Multiply<T>.multiply
    /// @resolution.name source=value target=square.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=square.value

}

/// @generic.instance id=Multiply<T>.multiply template=ops.multiply.Multiply.multiply arguments=(T)
"#);
}
