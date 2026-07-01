use crate::tests::{DirRows, TestSession};

#[test]
fn test_finite_object_assigns_to_structural_shape() {
    let session = TestSession::single(
        r#"
type Person = { name: string };

const source = { name: "Ada" };
const person: Person = source;

person.name satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Person = { name: string };

const source: { name: string } = { name: "Ada" };
const person: Person = source;

person.name satisfies string;

=== checked ===
type Person = { name: string };
/// @type.symbol symbol=Person source="type Person = { name: string }" type={ name: string }
/// @definition.type symbol=Person source="type Person = { name: string }" value={ name: string }

const source = { name: "Ada" };
/// @type.symbol symbol=source source=source type={ name: string }
/// @type.node source={ name: "Ada" } type={ name: "Ada" }
/// @type.node source="\"Ada\"" type="Ada"

const person: Person = source;
/// @type.symbol symbol=person source=person type=Person reduced={ name: string }
/// @resolution.name source=Person target=Person
/// @type.node source=source type={ name: string }
/// @resolution.name source=source target=source

person.name satisfies string;
/// @type.node source="person.name satisfies string" type=string
/// @type.node source=person type=Person reduced={ name: string }
/// @type.node source=person.name type=string
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string } kind=field key=name
"#,
    );
}
