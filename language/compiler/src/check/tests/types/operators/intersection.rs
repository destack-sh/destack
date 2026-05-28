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
type Named = { name: string };
/// @type.symbol symbol=Named type={ name: string }

type Aged = { age: int32 };
/// @type.symbol symbol=Aged type={ age: int32 }

type Person = Named & Aged;
/// @resolution.name source=Named target=Named
/// @resolution.name source=Aged target=Aged
/// @type.symbol symbol=Person type={ name: string; age: int32 }

declare const person: Person;
/// @resolution.name source=Person target=Person
/// @type.symbol symbol=person type={ name: string; age: int32 }

const name = person.name;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string; age: int32 } kind=symbol target=person.name
/// @type.symbol symbol=name type=string

const age = person.age;
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver={ name: string; age: int32 } kind=symbol target=person.age
/// @type.symbol symbol=age type=int32
"#,
    );
}
