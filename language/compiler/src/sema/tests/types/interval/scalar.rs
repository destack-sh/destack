use crate::tests::{DirRows, TestSession};

/// An interval alias keeps its bounds in the checked type.
#[test]
fn test_interval_alias_preserves_bounds() {
    let session = TestSession::single(
        r#"
type Count = 0..5;

declare const count: Count;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Count = 0..5;

declare const count: Count;

=== dir ===
type Count = 0..5;
/// @type.symbol symbol=Count source="type Count = 0..5" type=0..5
/// @definition.type symbol=Count source="type Count = 0..5" value=0..5

declare const count: Count;
/// @type.symbol symbol=count source=count type=Count
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.name source=Count target=Count
"#,
    );
}

/// An interval literal satisfies the integer domain interface.
#[test]
fn test_interval_satisfies_integer_domain() {
    let session = TestSession::single(
        r#"
import { IntegerDomain } from "tspp:math";

declare const value: 0..=255;

value satisfies IntegerDomain;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { IntegerDomain } from "tspp:math";

declare const value: 0..=255;

value satisfies IntegerDomain;

=== dir ===
import { IntegerDomain } from "tspp:math";

declare const value: 0..=255;
/// @type.symbol symbol=value source=value type=0..=255
/// @resolution.pattern source=value kind=binding target=value

value satisfies IntegerDomain;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.name source=IntegerDomain target=IntegerDomain
"#,
    );
}
