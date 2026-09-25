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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function square<T: int32 | float64>(value: T): T {
    return value * value;
}

=== dir ===
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function scale<T>(left: T, right: T): T where T: int32 | float64 {
    return left * right;
}

=== dir ===
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
fn test_literal_operand_must_fit_every_scalar_alternative() {
    let session = TestSession::single(
        r#"
function offset<T: int8 | int64>(value: T): T {
    return value + 128;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function offset<T: int8 | int64>(value: T): T {
    return value + 128;
}

=== dir ===
function offset<T: int8 | int64>(value: T): T {
/// @generic.template symbol=offset parameters=(T: int8 | int64)
/// @type.symbol symbol=offset type=<T: int8 | int64>(T) => T
/// @type.symbol symbol=offset.T source="T: int8 | int64" type=T
/// @type.symbol symbol=offset.value source="value: T" type=T
/// @resolution.name source=T target=offset.T
/// @resolution.name source=T target=offset.T

    return value + 128;
    /// @resolution.name source=value target=offset.value
    /// @resolution.rejected source="value + 128"
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
fn test_reject_a_literal_operand_outside_one_conjoined_bound() {
    let session = TestSession::single(
        r#"
function offset<T: int8 | float64>(value: T): T where T: uint8 {
    return value + 200;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function offset<T: int8 | float64>(value: T): T where T: uint8 {
    return value + 200;
}

=== dir ===
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
    /// @resolution.rejected source="value + 200"
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function ordered<T: int32 | float64>(left: T, right: T): boolean {
    return left < right;
}

=== dir ===
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function negate<T: int32 | float64>(value: T): T {
    return -value;
}

=== dir ===
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function flip<T: int32 | int64>(value: T): T {
    return ~value;
}

=== dir ===
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function mix<T: int32 | float64, U: int32 | float64>(left: T, right: U): T {
    return left * right;
}

=== dir ===
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
    /// @resolution.rejected source="left * right"
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function double<T>(value: T): T {
    return value + value;
}

=== dir ===
function double<T>(value: T): T {
/// @generic.template symbol=double parameters=(T)
/// @type.symbol symbol=double type=<T>(T) => T
/// @type.symbol symbol=double.T source=T type=T
/// @type.symbol symbol=double.value source="value: T" type=T
/// @resolution.name source=T target=double.T
/// @resolution.name source=T target=double.T

    return value + value;
    /// @resolution.name source=value target=double.value
    /// @resolution.rejected source="value + value"
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
fn test_scalar_domain_parameter_rejects_arithmetic() {
    let session = TestSession::single(
        r#"
import { IntegerDomain } from "tspp:math";

function double<T: IntegerDomain>(value: T): T {
    return value + value;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { IntegerDomain } from "tspp:math";

function double<T: IntegerDomain>(value: T): T {
    return value + value;
}

=== dir ===
import { IntegerDomain } from "tspp:math";

function double<T: IntegerDomain>(value: T): T {
/// @generic.template symbol=double parameters=(T: IntegerDomain)
/// @type.symbol symbol=double type=<T: IntegerDomain>(T) => T
/// @type.symbol symbol=double.T source="T: IntegerDomain" type=T
/// @resolution.name source=IntegerDomain target=IntegerDomain
/// @type.symbol symbol=double.value source="value: T" type=T
/// @resolution.name source=T target=double.T
/// @resolution.name source=T target=double.T

    return value + value;
    /// @resolution.name source=value target=double.value
    /// @resolution.rejected source="value + value"
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=double.value
    /// @resolution.name source=value target=double.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=double.value

}
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '+' is not defined for 'T' and 'T'"
/// @diagnostic.label line=5 column=18 span="+" line_source="return value + value;"
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const value = missing * 2;

=== dir ===
const value = missing * 2;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.poisoned source="missing * 2"
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const value: int32 | float64;
const doubled: int32 | float64 = value * value;

=== dir ===
declare const value: int32 | float64;
/// @type.symbol symbol=value source=value type=int32 | float64
/// @resolution.pattern source=value kind=binding target=value

const doubled = value * value;
/// @type.symbol symbol=doubled source=doubled type=int32 | float64
/// @resolution.pattern source=doubled kind=binding target=doubled
/// @resolution.name source=value target=value
/// @resolution.operator source="value * value" type=int32 | float64 operator="*" kind=builtin operands=[value as int32 | float64 families=(integer | float), value as int32 | float64 families=(integer | float)]
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

#[test]
fn test_bounded_parameter_selects_overloaded_operator() {
    let session = TestSession::single(
        r#"
import { Multiply } from "tspp:ops";

function square<T: Multiply<T>>(value: T): T.Output {
    return value * value;
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Multiply } from "tspp:ops";

function square<T: Multiply<T>>(value: T): T.Output {
    return value * value;
}

=== dir ===
import { Multiply } from "tspp:ops";

function square<T: Multiply<T>>(value: T): T.Output {
/// @generic.template symbol=square parameters=(T: Multiply<T>)
/// @type.symbol symbol=square type=<T: Multiply<T>>(T) => T.Output
/// @type.symbol symbol=square.T source="T: Multiply<T>" type=T
/// @resolution.name source=Multiply target=Multiply
/// @generic.instance id=Multiply<T> template=Multiply arguments=(T)
/// @resolution.name source=T target=square.T
/// @type.symbol symbol=square.value source="value: T" type=T
/// @resolution.name source=T target=square.T
/// @resolution.name source=T.Output target=square.T
/// @resolution.path source=T.Output index=1 target=Multiply.Output

    return value * value;
    /// @resolution.name source=value target=square.value
    /// @resolution.operator source="value * value" type=T.Output operator="*" kind=call parameters=(T) arguments=(provided(value) as T) return=T.Output kind=symbol target=Multiply.multiply receiver=T instance=Multiply<T>.multiply
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=square.value
    /// @generic.instantiation id="Multiply.multiply<T, T>" template=Multiply.multiply arguments=(T) owner=square
    /// @generic.instance id="Multiply.multiply<T, T>" template=Multiply.multiply arguments=(T)
    /// @resolution.name source=value target=square.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=square.value

}
"#);
}

#[test]
fn test_warn_constant_shift_amount_past_width() {
    let session = TestSession::single(
        r#"
declare const bits: int32;
const spilled = bits << 32;
const kept = bits << 3;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::none(),
        r#"
=== annotated ===
declare const bits: int32;
const spilled: int32 = bits << 32;
const kept: int32 = bits << 3;

=== dir ===
declare const bits: int32;
const spilled = bits << 32;
const kept = bits << 3;
"#,
        r#"
/// @diagnostic.warning id=shift-out-of-range message="shift amount 32 is out of range for 'int32'"
/// @diagnostic.label line=3 column=22 span="<<" line_source="const spilled = bits << 32;"
"#,
    );
}

#[test]
fn test_reject_a_literal_operand_at_a_rigid_parameter() {
    let session = TestSession::single(
        r#"
import { Integer } from "tspp:math";

function decrement<T: Integer>(value: T): T {
    return value - 1;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Integer } from "tspp:math";

function decrement<T: Integer>(value: T): T {
    return value - 1;
}

=== dir ===
import { Integer } from "tspp:math";

function decrement<T: Integer>(value: T): T {
/// @generic.template symbol=decrement parameters=(T: Integer)
/// @type.symbol symbol=decrement type=<T: Integer>(T) => T
/// @type.symbol symbol=decrement.T source="T: Integer" type=T
/// @resolution.name source=Integer target=Integer
/// @type.symbol symbol=decrement.value source="value: T" type=T
/// @resolution.name source=T target=decrement.T
/// @resolution.name source=T target=decrement.T

    return value - 1;
    /// @resolution.name source=value target=decrement.value
    /// @resolution.rejected source="value - 1"
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=decrement.value

}
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '-' is not defined for 'T' and '1'"
/// @diagnostic.label line=5 column=18 span="-" line_source="return value - 1;"
"#,
    );
}

/// Compare two generic operands through the comparison their bounds declare.
#[test]
fn test_compare_generic_operands_through_their_bounds() {
    let session = TestSession::single(
        r#"
import { Compare, Ordering } from "tspp:ops";

function before<T: Compare<U>, U>(left: &immutable T, right: &immutable U): boolean {
    return *left <= *right;
}

function ordered<T: Compare>(left: &immutable T, right: &immutable T): boolean {
    return *left < *right;
}

function isLess<T: Compare<U>, U>(left: &immutable T, right: &immutable U): boolean {
    return left.compare(right) == Ordering.Less;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Compare, Ordering } from "tspp:ops";

function before<T: Compare<U>, U, 'a, 'b>(left: &'a immutable T, right: &'b immutable U): boolean {
    return *left <= (*right as &'b immutable U);
}

function ordered<T: Compare, 'a, 'b>(left: &'a immutable T, right: &'b immutable T): boolean {
    return *left < (*right as &'b immutable T);
}

function isLess<T: Compare<U>, U, 'a, 'b>(left: &'a immutable T, right: &'b immutable U): boolean {
    return left.compare<U, 'a, 'b>(right) == Ordering.Less;
}

=== dir ===
import { Compare, Ordering } from "tspp:ops";

function before<T: Compare<U>, U>(left: &immutable T, right: &immutable U): boolean {
/// @generic.template symbol=before parameters=(T#1: Compare<U#1>, U#1, 'a, 'b)
/// @type.symbol symbol=before type=<T#1: Compare<U#1>, U#1, before.'a, before.'b>(&before.'a immutable T#1, &before.'b immutable U#1) => boolean
/// @type.symbol symbol=before.T source="T: Compare<U>" type=T#1
/// @resolution.name source=Compare target=Compare
/// @generic.instance id=Compare<U#1> template=Compare arguments=(U#1)
/// @resolution.name source=U target=before.U
/// @type.symbol symbol=before.U source=U type=U#1
/// @type.symbol symbol=before.left source="left: &immutable T" type=&before.'a immutable T#1
/// @resolution.name source=T target=before.T
/// @type.symbol symbol=before.right source="right: &immutable U" type=&before.'b immutable U#1
/// @resolution.name source=U target=before.U

    return *left <= *right;
    /// @resolution.operator source="*left <= *right" type=boolean operator="<=" kind=call parameters=(&before.'b immutable U#1) arguments=(provided(*right) as &before.'b immutable U#1) return=Ordering regions=(before.'a, before.'b) kind=symbol target=Compare.compare receiver=^T#1 adjustments=(borrow(&before.'a immutable T#1)) instance="Compare<U#1>.compare<before.'a, before.'b>"
    /// @resolution.place source=*left placement=before.'a lifetime=before.'a access="immutable"
    /// @resolution.operator source=*left type=^T#1 operator="*" kind=builtin operands=[left as &before.'a immutable T#1]
    /// @generic.instantiation id="Compare.compare<T#1, U#1, before.'a, before.'b>" template=Compare.compare arguments=(U#1, before.'a, before.'b) owner=before
    /// @generic.instance id="Compare.compare<T#1, U#1, before.'a, before.'b>" template=Compare.compare arguments=(U#1, before.'a, before.'b)
    /// @resolution.name source=left target=before.left
    /// @resolution.place source=left placement=before.'a lifetime=before.'a access="immutable"
    /// @resolution.access source=left root=before.left
    /// @resolution.place source=*right placement=before.'b lifetime=before.'b access="immutable"
    /// @resolution.operator source=*right type=^U#1 operator="*" kind=builtin operands=[right as &before.'b immutable U#1]
    /// @resolution.name source=right target=before.right
    /// @resolution.place source=right placement=before.'b lifetime=before.'b access="immutable"
    /// @resolution.access source=right root=before.right

}

function ordered<T: Compare>(left: &immutable T, right: &immutable T): boolean {
/// @generic.template symbol=ordered parameters=(T#2: Compare, 'a, 'b)
/// @type.symbol symbol=ordered type=<T#2: Compare, ordered.'a, ordered.'b>(&ordered.'a immutable T#2, &ordered.'b immutable T#2) => boolean
/// @type.symbol symbol=ordered.T source="T: Compare" type=T#2
/// @resolution.name source=Compare target=Compare
/// @type.symbol symbol=ordered.left source="left: &immutable T" type=&ordered.'a immutable T#2
/// @resolution.name source=T target=ordered.T
/// @type.symbol symbol=ordered.right source="right: &immutable T" type=&ordered.'b immutable T#2
/// @resolution.name source=T target=ordered.T

    return *left < *right;
    /// @resolution.operator source="*left < *right" type=boolean operator="<" kind=call parameters=(&ordered.'b immutable T#2) arguments=(provided(*right) as &ordered.'b immutable T#2) return=Ordering regions=(ordered.'a, ordered.'b) kind=symbol target=Compare.compare receiver=^T#2 adjustments=(borrow(&ordered.'a immutable T#2)) instance="Compare<this>.compare<ordered.'a, ordered.'b>"
    /// @resolution.place source=*left placement=ordered.'a lifetime=ordered.'a access="immutable"
    /// @resolution.operator source=*left type=^T#2 operator="*" kind=builtin operands=[left as &ordered.'a immutable T#2]
    /// @generic.instantiation id="Compare.compare<T#2, this, ordered.'a, ordered.'b>" template=Compare.compare arguments=(this, ordered.'a, ordered.'b) owner=ordered
    /// @resolution.name source=left target=ordered.left
    /// @resolution.place source=left placement=ordered.'a lifetime=ordered.'a access="immutable"
    /// @resolution.access source=left root=ordered.left
    /// @resolution.place source=*right placement=ordered.'b lifetime=ordered.'b access="immutable"
    /// @resolution.operator source=*right type=^T#2 operator="*" kind=builtin operands=[right as &ordered.'b immutable T#2]
    /// @resolution.name source=right target=ordered.right
    /// @resolution.place source=right placement=ordered.'b lifetime=ordered.'b access="immutable"
    /// @resolution.access source=right root=ordered.right

}

function isLess<T: Compare<U>, U>(left: &immutable T, right: &immutable U): boolean {
/// @generic.template symbol=isLess parameters=(T#3: Compare<U#2>, U#2, 'a, 'b)
/// @type.symbol symbol=isLess type=<T#3: Compare<U#2>, U#2, isLess.'a, isLess.'b>(&isLess.'a immutable T#3, &isLess.'b immutable U#2) => boolean
/// @type.symbol symbol=isLess.T source="T: Compare<U>" type=T#3
/// @resolution.name source=Compare target=Compare
/// @generic.instance id=Compare<U#2> template=Compare arguments=(U#2)
/// @resolution.name source=U target=isLess.U
/// @type.symbol symbol=isLess.U source=U type=U#2
/// @type.symbol symbol=isLess.left source="left: &immutable T" type=&isLess.'a immutable T#3
/// @resolution.name source=T target=isLess.T
/// @type.symbol symbol=isLess.right source="right: &immutable U" type=&isLess.'b immutable U#2
/// @resolution.name source=U target=isLess.U

    return left.compare(right) == Ordering.Less;
    /// @resolution.name source=left target=isLess.left
    /// @resolution.member source=left.compare receiver=&isLess.'a immutable T#3 type=<Compare.compare.'a, Compare.compare.'b>(this: &Compare.compare.'a immutable T#3, &Compare.compare.'b immutable U#2) => Ordering kind=symbol target_receiver=&isLess.'a immutable T#3 target=Compare.compare
    /// @resolution.call source=left.compare(right) parameters=(&isLess.'b immutable U#2) arguments=(provided(right) as &isLess.'b immutable U#2) return=Ordering regions=(isLess.'a, isLess.'b) kind=symbol target=Compare.compare receiver=&isLess.'a immutable T#3 instance="Compare<U#2>.compare<isLess.'a, isLess.'b>"
    /// @resolution.operator source="left.compare(right) == Ordering.Less" type=boolean operator="==" kind=builtin operands=[left.compare(right) as Ordering families=(Ordering), Ordering.Less as Ordering.Less families=(Ordering)]
    /// @resolution.place source=left placement=isLess.'a lifetime=isLess.'a access="immutable"
    /// @resolution.access source=left root=isLess.left
    /// @generic.instantiation id="Compare.compare<T#3, U#2, isLess.'a, isLess.'b>" template=Compare.compare arguments=(U#2, isLess.'a, isLess.'b) owner=isLess
    /// @generic.instantiation id=Compare.compare<U#2> template=Compare.compare arguments=(U#2) owner=isLess
    /// @generic.instance id="Compare.compare<T#3, U#2, isLess.'a, isLess.'b>" template=Compare.compare arguments=(U#2, isLess.'a, isLess.'b)
    /// @resolution.name source=right target=isLess.right
    /// @resolution.place source=right placement=isLess.'b lifetime=isLess.'b access="immutable"
    /// @resolution.access source=right root=isLess.right
    /// @resolution.name source=Ordering target=Ordering
    /// @resolution.member source=Ordering.Less receiver=Ordering type=Ordering.Less kind=symbol target_receiver=Ordering target=Ordering.Less

}
"#,
    );
}
