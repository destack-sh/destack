use crate::tests::{DirRows, TestSession};

#[test]
fn test_intersection_merges_object_shape_members() {
    let session = TestSession::single(
        r#"
type Named = { name: string };
type Aged = { age: int32 };
type Person = Named & Aged;

declare const person: Person;
const name = person.name;
const age = person.age;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Named = { name: string };
type Aged = { age: int32 };
type Person = Named & Aged;

declare const person: Person;
const name: string = person.name;
const age: int32 = person.age;

=== checked ===
type Named = { name: string };
/// @type.symbol symbol=Named source="type Named = { name: string }" type={ name: string }
/// @definition.type symbol=Named source="type Named = { name: string }" value={ name: string }

type Aged = { age: int32 };
/// @type.symbol symbol=Aged source="type Aged = { age: int32 }" type={ age: int32 }
/// @definition.type symbol=Aged source="type Aged = { age: int32 }" value={ age: int32 }

type Person = Named & Aged;
/// @type.symbol symbol=Person source="type Person = Named & Aged" type={ name: string; age: int32 }
/// @definition.type symbol=Person source="type Person = Named & Aged" value={ name: string; age: int32 }
/// @resolution.name source=Named target=Named
/// @resolution.name source=Aged target=Aged

declare const person: Person;
/// @type.symbol symbol=person source=person type={ name: string } & { age: int32 }
/// @resolution.name source=Person target=Person

const name = person.name;
/// @type.symbol symbol=name source=name type=string
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string } & { age: int32 } kind=field key=name

const age = person.age;
/// @type.symbol symbol=age source=age type=int32
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver={ name: string } & { age: int32 } kind=field key=age
"#,
    );
}
