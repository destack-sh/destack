use crate::tests::{DirRows, TestSession};

#[test]
fn test_template_literal_key_remapping_preserves_values() {
    let session = TestSession::single(
        r#"
type Handlers<T> = {
    [K in keyof T as `on-${K}`]: (value: T[K]) => void;
};

type Events = {
    ready: boolean;
    message: string;
};

declare const handlers: Handlers<Events>;

handlers["on-ready"] satisfies (value: boolean) => void;
handlers["on-message"] satisfies (value: string) => void;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Handlers<T> = {
    [K in keyof T as `on-${K}`]: (value: T[K]) => void;
};

type Events = {
    ready: boolean;
    message: string;
};

declare const handlers: Handlers<Events>;

handlers["on-ready"] satisfies (value: boolean) => void;
handlers["on-message"] satisfies (value: string) => void;

=== checked ===
type Handlers<T> = {
/// @generic.template symbol=Handlers parameters=(T)
/// @type.symbol symbol=Handlers type={ [K in keyof T as `on-${K}`]: (value: T[K]) => void }

    [K in keyof T as `on-${K}`]: (value: T[K]) => void;
};

type Events = {
/// @type.symbol symbol=Events source="type Events = {\n    ready: boolean;\n    message: string;\n}" type={ ready: boolean; message: string }
/// @definition.type symbol=Events source="type Events = {\n    ready: boolean;\n    message: string;\n}" value={ ready: boolean; message: string }

    ready: boolean;
    message: string;
};

declare const handlers: Handlers<Events>;
/// @type.symbol symbol=handlers source=handlers type=Handlers<Events>
/// @resolution.name source=Handlers target=Handlers
/// @resolution.name source=Events target=Events
/// @generic.instance source="Handlers<Events>" id=Handlers<Events>

handlers["on-ready"] satisfies (value: boolean) => void;
/// @resolution.name source=handlers target=handlers
/// @resolution.member source="handlers[\"on-ready\"]" receiver=Handlers<Events> kind=field key=on-ready

handlers["on-message"] satisfies (value: string) => void;
/// @resolution.name source=handlers target=handlers
/// @resolution.member source="handlers[\"on-message\"]" receiver=Handlers<Events> kind=field key=on-message
/// @generic.instance id=Handlers<Events> template=Handlers arguments=(Events)
"#,
    );
}

#[test]
fn test_template_literal_key_remapping_supports_capitalized_string_keys() {
    let session = TestSession::single(
        r#"
type Getters<T> = {
    [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K];
};

type Person = {
    name: string;
    age: int32;
};

declare const getters: Getters<Person>;

getters.getName satisfies () => string;
getters.getAge satisfies () => int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Getters<T> = {
    [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K];
};

type Person = {
    name: string;
    age: int32;
};

declare const getters: Getters<Person>;

getters.getName satisfies () => string;
getters.getAge satisfies () => int32;

=== checked ===
type Getters<T> = {
/// @generic.template symbol=Getters parameters=(T)
/// @type.symbol symbol=Getters type={ [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K] }

    [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K];
};

type Person = {
/// @type.symbol symbol=Person source="type Person = {\n    name: string;\n    age: int32;\n}" type={ name: string; age: int32 }
/// @definition.type symbol=Person source="type Person = {\n    name: string;\n    age: int32;\n}" value={ name: string; age: int32 }

    name: string;
    age: int32;
};

declare const getters: Getters<Person>;
/// @type.symbol symbol=getters source=getters type=Getters<Person>
/// @resolution.name source=Getters target=Getters
/// @resolution.name source=Person target=Person
/// @generic.instance source="Getters<Person>" id=Getters<Person>

getters.getName satisfies () => string;
/// @resolution.name source=getters target=getters
/// @resolution.member source=getters.getName receiver=Getters<Person> kind=field key=getName

getters.getAge satisfies () => int32;
/// @resolution.name source=getters target=getters
/// @resolution.member source=getters.getAge receiver=Getters<Person> kind=field key=getAge
/// @generic.instance id=Getters<Person> template=Getters arguments=(Person)
"#,
    );
}

#[test]
fn test_template_literal_key_remapping_supports_indexed_access() {
    let session = TestSession::single(
        r#"
type Handlers<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

type Value = Handlers<{ name: string }>["on-name"];

declare const value: Value;
value satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Handlers<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

type Value = Handlers<{ name: string }>["on-name"];

declare const value: Value;
value satisfies string;

=== checked ===
type Handlers<T> = {
/// @generic.template symbol=Handlers parameters=(T)
/// @type.symbol symbol=Handlers type={ [K in keyof T as `on-${K}`]: T[K] }

    [K in keyof T as `on-${K}`]: T[K];
};

type Value = Handlers<{ name: string }>["on-name"];
/// @type.symbol symbol=Value source="type Value = Handlers<{ name: string }>[\"on-name\"]" type=string
/// @definition.type symbol=Value source="type Value = Handlers<{ name: string }>[\"on-name\"]" value=string
/// @resolution.name source=Handlers target=Handlers
/// @generic.instance source="Handlers<{ name: string }>" id=Handlers<{ name: string }>

declare const value: Value;
/// @type.symbol symbol=value source=value type=string
/// @resolution.name source=Value target=Value

value satisfies string;
/// @resolution.name source=value target=value
/// @generic.instance id=Handlers<{ name: string }> template=Handlers arguments=({ name: string })
"#,
    );
}

#[test]
fn test_template_literal_key_remapping_can_infer_original_key_part() {
    let session = TestSession::single(
        r#"
type Events = {
    userCreated: string;
    orderPaid: int32;
};

type Names<T> = {
    [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K];
};

declare const names: Names<Events>;

names.user satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Events = {
    userCreated: string;
    orderPaid: int32;
};

type Names<T> = {
    [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K];
};

declare const names: Names<Events>;

names.user satisfies string;

=== checked ===
type Events = {
/// @type.symbol symbol=Events source="type Events = {\n    userCreated: string;\n    orderPaid: int32;\n}" type={ userCreated: string; orderPaid: int32 }
/// @definition.type symbol=Events source="type Events = {\n    userCreated: string;\n    orderPaid: int32;\n}" value={ userCreated: string; orderPaid: int32 }

    userCreated: string;
    orderPaid: int32;
};

type Names<T> = {
/// @generic.template symbol=Names parameters=(T)
/// @type.symbol symbol=Names type={ [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K] }

    [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K];
};

declare const names: Names<Events>;
/// @type.symbol symbol=names source=names type=Names<Events>
/// @resolution.name source=Names target=Names
/// @resolution.name source=Events target=Events
/// @generic.instance source="Names<Events>" id=Names<Events>

names.user satisfies string;
/// @resolution.name source=names target=names
/// @resolution.member source=names.user receiver=Names<Events> kind=field key=user
/// @generic.instance id=Names<Events> template=Names arguments=(Events)
"#,
    );
}

#[test]
fn test_template_literal_key_remapping_drops_nonmatching_keys() {
    let session = TestSession::single(
        r#"
type Events = {
    userCreated: string;
    orderPaid: int32;
};

type Names<T> = {
    [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K];
};

declare const names: Names<Events>;
const missing = names.orderPaid;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Events = {
    userCreated: string;
    orderPaid: int32;
};

type Names<T> = {
    [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K];
};

declare const names: Names<Events>;
const missing = names.orderPaid;

=== checked ===
type Events = {
/// @type.symbol symbol=Events source="type Events = {\n    userCreated: string;\n    orderPaid: int32;\n}" type={ userCreated: string; orderPaid: int32 }
/// @definition.type symbol=Events source="type Events = {\n    userCreated: string;\n    orderPaid: int32;\n}" value={ userCreated: string; orderPaid: int32 }

    userCreated: string;
    orderPaid: int32;
};

type Names<T> = {
/// @generic.template symbol=Names parameters=(T)
/// @type.symbol symbol=Names type={ [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K] }

    [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K];
};

declare const names: Names<Events>;
/// @type.symbol symbol=names source=names type=Names<Events>
/// @resolution.name source=Names target=Names
/// @resolution.name source=Events target=Events
/// @generic.instance source="Names<Events>" id=Names<Events>

const missing = names.orderPaid;
/// @type.symbol symbol=missing type=<error>
/// @resolution.name source=names target=names
/// @generic.instance id=Names<Events> template=Names arguments=(Events)
"#,
        r#"
/// @diagnostic.error code=EC300 message="member 'orderPaid' does not exist on type 'Names<Events>'"
/// @diagnostic.label line=12 column=17 source="const missing = names.orderPaid;"
"#,
    );
}

#[test]
fn test_template_literal_key_remapping_contextualizes_satisfies() {
    let session = TestSession::single(
        r#"
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

const handlers = {
    "on-open": true,
    "on-close": false,
} satisfies HandlerMap<{ open: boolean; close: boolean }>;

handlers["on-open"] satisfies boolean;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

const handlers: { "on-open": boolean; "on-close": boolean } = {
    "on-open": true,
    "on-close": false,
} satisfies HandlerMap<{ open: boolean; close: boolean }>;

handlers["on-open"] satisfies boolean;

=== checked ===
type HandlerMap<T> = {
/// @generic.template symbol=HandlerMap parameters=(T)
/// @type.symbol symbol=HandlerMap type={ [K in keyof T as `on-${K}`]: T[K] }

    [K in keyof T as `on-${K}`]: T[K];
};

const handlers = {
/// @type.symbol symbol=handlers type={ "on-open": boolean; "on-close": boolean }

    "on-open": true,
    "on-close": false,
} satisfies HandlerMap<{ open: boolean; close: boolean }>;
/// @resolution.name source=HandlerMap target=HandlerMap
/// @generic.instance source="HandlerMap<{ open: boolean; close: boolean }>" id=HandlerMap<{ open: boolean; close: boolean }>

handlers["on-open"] satisfies boolean;
/// @resolution.name source=handlers target=handlers
/// @resolution.member source="handlers[\"on-open\"]" receiver={ "on-open": boolean; "on-close": boolean } kind=field key=on-open
/// @generic.instance id=HandlerMap<{ open: boolean; close: boolean }> template=HandlerMap arguments=({ open: boolean; close: boolean })
"#,
    );
}
