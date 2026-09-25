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

    session.assert_dir(
        "main.tspp",
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

=== dir ===
type Handlers<T> = {
/// @generic.template symbol=Handlers parameters=(T)
/// @type.symbol symbol=Handlers type={ [K in keyof T as `on-${K}`]: (T[K]) => void }
/// @definition.type symbol=Handlers template=(T) value={ [K in keyof T as `on-${K}`]: (T[K]) => void }
/// @type.symbol symbol=Handlers.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)

    [K in keyof T as `on-${K}`]: (value: T[K]) => void;
    /// @type.symbol symbol=Handlers.K source=[K in keyof T as `on-${K}`] type=K
    /// @resolution.name source=T target=Handlers.T
    /// @resolution.name source=K target=Handlers.K
    /// @type.symbol symbol=Handlers.value source="value: T[K]" type=T[K]
    /// @resolution.name source=T target=Handlers.T
    /// @resolution.name source=K target=Handlers.K

};

type Events = {
/// @type.symbol symbol=Events type={ ready: boolean; message: string }
/// @definition.type symbol=Events value={ ready: boolean; message: string }

    ready: boolean;
    /// @type.symbol symbol=Events.ready source="ready: boolean" type=boolean

    message: string;
    /// @type.symbol symbol=Events.message source="message: string" type=string

};

declare const handlers: Handlers<Events>;
/// @type.symbol symbol=handlers source=handlers type=Handlers<Events>
/// @resolution.pattern source=handlers kind=binding target=handlers
/// @resolution.name source=Handlers target=Handlers
/// @resolution.name source=Events target=Events

handlers["on-ready"] satisfies (value: boolean) => void;
/// @resolution.name source=handlers target=handlers
/// @resolution.place source="handlers[\"on-ready\"]" placement="local" lifetime="managed" access="mutable"
/// @resolution.access source="handlers[\"on-ready\"]" root=handlers keys=[on-ready]
/// @resolution.subscript source="handlers[\"on-ready\"]" type=(Events["ready"]) => void kind=member target="receiver=Handlers<Events>, target=field(receiver=Handlers<Events>, target=on-ready, type=(Events[\"ready\"]) => void), type=(Events[\"ready\"]) => void"
/// @resolution.place source=handlers placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handlers root=handlers
/// @type.symbol symbol=value#1 source="value: boolean" type=boolean

handlers["on-message"] satisfies (value: string) => void;
/// @resolution.name source=handlers target=handlers
/// @resolution.place source="handlers[\"on-message\"]" placement="local" lifetime="managed" access="mutable"
/// @resolution.access source="handlers[\"on-message\"]" root=handlers keys=[on-message]
/// @resolution.subscript source="handlers[\"on-message\"]" type=(Events["message"]) => void kind=member target="receiver=Handlers<Events>, target=field(receiver=Handlers<Events>, target=on-message, type=(Events[\"message\"]) => void), type=(Events[\"message\"]) => void"
/// @resolution.place source=handlers placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handlers root=handlers
/// @type.symbol symbol=value#2 source="value: string" type=string
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

    session.assert_dir(
        "main.tspp",
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

=== dir ===
type Getters<T> = {
/// @generic.template symbol=Getters parameters=(T)
/// @type.symbol symbol=Getters type={ [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K] }
/// @definition.type symbol=Getters template=(T) value={ [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K] }
/// @type.symbol symbol=Getters.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)

    [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K];
    /// @type.symbol symbol=Getters.K source=[K in keyof T as `get${Capitalize<string & K>}`] type=K
    /// @resolution.name source=T target=Getters.T
    /// @resolution.name source=Capitalize target=Capitalize
    /// @resolution.name source=K target=Getters.K
    /// @resolution.name source=T target=Getters.T
    /// @resolution.name source=K target=Getters.K

};

type Person = {
/// @type.symbol symbol=Person type={ name: string; age: int32 }
/// @definition.type symbol=Person value={ name: string; age: int32 }

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

};

declare const getters: Getters<Person>;
/// @type.symbol symbol=getters source=getters type=Getters<Person>
/// @resolution.pattern source=getters kind=binding target=getters
/// @resolution.name source=Getters target=Getters
/// @resolution.name source=Person target=Person

getters.getName satisfies () => string;
/// @resolution.name source=getters target=getters
/// @resolution.member source=getters.getName receiver=Getters<Person> type=() => Person["name"] kind=field target_receiver=Getters<Person> key=getName target_type=() => Person["name"]
/// @resolution.place source=getters placement="local" lifetime="static" access="immutable"
/// @resolution.access source=getters root=getters
/// @resolution.place source=getters.getName placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=getters.getName root=getters keys=[getName]

getters.getAge satisfies () => int32;
/// @resolution.name source=getters target=getters
/// @resolution.member source=getters.getAge receiver=Getters<Person> type=() => Person["age"] kind=field target_receiver=Getters<Person> key=getAge target_type=() => Person["age"]
/// @resolution.place source=getters placement="local" lifetime="static" access="immutable"
/// @resolution.access source=getters root=getters
/// @resolution.place source=getters.getAge placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=getters.getAge root=getters keys=[getAge]
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Handlers<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

type Value = Handlers<{ name: string }>["on-name"];

declare const value: Value;
value satisfies string;

=== dir ===
type Handlers<T> = {
/// @generic.template symbol=Handlers parameters=(T)
/// @type.symbol symbol=Handlers type={ [K in keyof T as `on-${K}`]: T[K] }
/// @definition.type symbol=Handlers template=(T) value={ [K in keyof T as `on-${K}`]: T[K] }
/// @type.symbol symbol=Handlers.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)

    [K in keyof T as `on-${K}`]: T[K];
    /// @type.symbol symbol=Handlers.K source=[K in keyof T as `on-${K}`] type=K
    /// @resolution.name source=T target=Handlers.T
    /// @resolution.name source=K target=Handlers.K
    /// @resolution.name source=T target=Handlers.T
    /// @resolution.name source=K target=Handlers.K

};

type Value = Handlers<{ name: string }>["on-name"];
/// @type.symbol symbol=Value source="type Value = Handlers<{ name: string }>[\"on-name\"]" type=string
/// @definition.type symbol=Value source="type Value = Handlers<{ name: string }>[\"on-name\"]" value=Handlers<{ name: string }>["on-name"]
/// @resolution.name source=Handlers target=Handlers
/// @type.symbol symbol=Value.name source="name: string" type=string

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

value satisfies string;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
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

    session.assert_dir(
        "main.tspp",
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

=== dir ===
type Events = {
/// @type.symbol symbol=Events type={ userCreated: string; orderPaid: int32 }
/// @definition.type symbol=Events value={ userCreated: string; orderPaid: int32 }

    userCreated: string;
    /// @type.symbol symbol=Events.userCreated source="userCreated: string" type=string

    orderPaid: int32;
    /// @type.symbol symbol=Events.orderPaid source="orderPaid: int32" type=int32

};

type Names<T> = {
/// @generic.template symbol=Names parameters=(T)
/// @type.symbol symbol=Names type={ [K in keyof T as K extends `${infer Name}Created` ? Names.Name : never]: T[K] }
/// @definition.type symbol=Names template=(T) value={ [K in keyof T as K extends `${infer Name}Created` ? Names.Name : never]: T[K] }
/// @type.symbol symbol=Names.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)

    [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K];
    /// @type.symbol symbol=Names.K source=[K in keyof T as K extends `${infer Name}Created` ? Name : never] type=K
    /// @resolution.name source=T target=Names.T
    /// @resolution.name source=K target=Names.K
    /// @resolution.name source=Name target=Names.Name
    /// @resolution.name source=T target=Names.T
    /// @resolution.name source=K target=Names.K

};

declare const names: Names<Events>;
/// @type.symbol symbol=names source=names type=Names<Events>
/// @resolution.pattern source=names kind=binding target=names
/// @resolution.name source=Names target=Names
/// @resolution.name source=Events target=Events

names.user satisfies string;
/// @resolution.name source=names target=names
/// @resolution.member source=names.user receiver=Names<Events> type=string kind=field target_receiver=Names<Events> key=user target_type=string
/// @resolution.place source=names placement="local" lifetime="static" access="immutable"
/// @resolution.access source=names root=names
/// @resolution.place source=names.user placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=names.user root=names keys=[user]
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
type Events = {
/// @type.symbol symbol=Events type={ userCreated: string; orderPaid: int32 }
/// @definition.type symbol=Events value={ userCreated: string; orderPaid: int32 }

    userCreated: string;
    /// @type.symbol symbol=Events.userCreated source="userCreated: string" type=string

    orderPaid: int32;
    /// @type.symbol symbol=Events.orderPaid source="orderPaid: int32" type=int32

};

type Names<T> = {
/// @generic.template symbol=Names parameters=(T)
/// @type.symbol symbol=Names type={ [K in keyof T as K extends `${infer Name}Created` ? Names.Name : never]: T[K] }
/// @definition.type symbol=Names template=(T) value={ [K in keyof T as K extends `${infer Name}Created` ? Names.Name : never]: T[K] }
/// @type.symbol symbol=Names.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)

    [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K];
    /// @type.symbol symbol=Names.K source=[K in keyof T as K extends `${infer Name}Created` ? Name : never] type=K
    /// @resolution.name source=T target=Names.T
    /// @resolution.name source=K target=Names.K
    /// @resolution.name source=Name target=Names.Name
    /// @resolution.name source=T target=Names.T
    /// @resolution.name source=K target=Names.K

};

declare const names: Names<Events>;
/// @type.symbol symbol=names source=names type=Names<Events>
/// @resolution.pattern source=names kind=binding target=names
/// @resolution.name source=Names target=Names
/// @resolution.name source=Events target=Events

const missing = names.orderPaid;
/// @type.symbol symbol=missing source=missing type=<error>
/// @resolution.pattern source=missing kind=binding target=missing
/// @resolution.name source=names target=names
/// @resolution.place source=names placement="local" lifetime="static" access="immutable"
/// @resolution.access source=names root=names
/// @resolution.rejected source=names.orderPaid
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'orderPaid' does not exist on type 'Names<Events>'"
/// @diagnostic.label line=12 column=23 span="orderPaid" line_source="const missing = names.orderPaid;"
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

const handlers: { on-open: boolean; on-close: boolean } = {
    "on-open": true,
    "on-close": false,
} satisfies HandlerMap<{ open: boolean; close: boolean }>;

handlers["on-open"] satisfies boolean;

=== dir ===
type HandlerMap<T> = {
/// @generic.template symbol=HandlerMap parameters=(T)
/// @type.symbol symbol=HandlerMap type={ [K in keyof T as `on-${K}`]: T[K] }
/// @definition.type symbol=HandlerMap template=(T) value={ [K in keyof T as `on-${K}`]: T[K] }
/// @type.symbol symbol=HandlerMap.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)

    [K in keyof T as `on-${K}`]: T[K];
    /// @type.symbol symbol=HandlerMap.K source=[K in keyof T as `on-${K}`] type=K
    /// @resolution.name source=T target=HandlerMap.T
    /// @resolution.name source=K target=HandlerMap.K
    /// @resolution.name source=T target=HandlerMap.T
    /// @resolution.name source=K target=HandlerMap.K

};

const handlers = {
/// @type.symbol symbol=handlers source=handlers type={ on-open: boolean; on-close: boolean }
/// @resolution.pattern source=handlers kind=binding target=handlers

    "on-open": true,
    "on-close": false,
} satisfies HandlerMap<{ open: boolean; close: boolean }>;
/// @resolution.name source=HandlerMap target=HandlerMap
/// @type.symbol symbol=open source="open: boolean" type=boolean
/// @type.symbol symbol=close source="close: boolean" type=boolean

handlers["on-open"] satisfies boolean;
/// @resolution.name source=handlers target=handlers
/// @resolution.place source="handlers[\"on-open\"]" placement="local" lifetime="managed" access="mutable"
/// @resolution.access source="handlers[\"on-open\"]" root=handlers keys=[on-open]
/// @resolution.subscript source="handlers[\"on-open\"]" type=boolean kind=member target="receiver={ on-open: boolean; on-close: boolean }, target=field(receiver={ on-open: boolean; on-close: boolean }, target=on-open, type=boolean), type=boolean"
/// @resolution.place source=handlers placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handlers root=handlers
"#,
    );
}
