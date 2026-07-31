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
/// @type.symbol symbol=Handlers type={ [K in keyof T as `on-${K}`]: Function<(T[K],), void> }
/// @definition.type symbol=Handlers template=(T) value={ [K in keyof T as `on-${K}`]: Function<(T[K],), void> }
/// @type.symbol symbol=Handlers.T source=T type=T

    [K in keyof T as `on-${K}`]: (value: T[K]) => void;
    /// @generic.template source=mapped_type_parameter parent=template#0 parameters=(K: keyof T)
    /// @type.symbol symbol=Handlers.K source=[K in keyof T as `on-${K}`] type=K
    /// @resolution.name source=T target=Handlers.T
    /// @resolution.name source=K target=Handlers.K
    /// @resolution.name source=T target=Handlers.T
    /// @resolution.name source=K target=Handlers.K

};

type Events = {
/// @type.symbol symbol=Events type={ ready: boolean; message: string }
/// @definition.type symbol=Events value={ ready: boolean; message: string }

    ready: boolean;
    message: string;
};

declare const handlers: Handlers<Events>;
/// @type.symbol symbol=handlers source=handlers type=Handlers<Events> reduced={ on-ready: Function<(boolean,), void>; on-message: Function<(string,), void> }
/// @resolution.pattern source=handlers kind=binding target=handlers
/// @resolution.name source=Handlers target=Handlers
/// @resolution.name source=Events target=Events

handlers["on-ready"] satisfies (value: boolean) => void;
/// @resolution.name source=handlers target=handlers
/// @resolution.place source="handlers[\"on-ready\"]" placement="local" lifetime="static" access="exclusive"
/// @resolution.access source="handlers[\"on-ready\"]" root=handlers keys=[on-ready]
/// @resolution.subscript source="handlers[\"on-ready\"]" type=Function<(Events["ready"],), void> kind=member target="receiver={ on-ready: Function<(Events[\"ready\"],), void>; on-message: Function<(Events[\"message\"],), void> }, target=field(receiver={ on-ready: Function<(Events[\"ready\"],), void>; on-message: Function<(Events[\"message\"],), void> }, target=on-ready, type=Function<(Events[\"ready\"],), void>), type=Function<(Events[\"ready\"],), void>"
/// @resolution.place source=handlers placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=handlers root=handlers

handlers["on-message"] satisfies (value: string) => void;
/// @resolution.name source=handlers target=handlers
/// @resolution.place source="handlers[\"on-message\"]" placement="local" lifetime="static" access="exclusive"
/// @resolution.access source="handlers[\"on-message\"]" root=handlers keys=[on-message]
/// @resolution.subscript source="handlers[\"on-message\"]" type=Function<(Events["message"],), void> kind=member target="receiver={ on-ready: Function<(Events[\"ready\"],), void>; on-message: Function<(Events[\"message\"],), void> }, target=field(receiver={ on-ready: Function<(Events[\"ready\"],), void>; on-message: Function<(Events[\"message\"],), void> }, target=on-message, type=Function<(Events[\"message\"],), void>), type=Function<(Events[\"message\"],), void>"
/// @resolution.place source=handlers placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=handlers root=handlers

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
/// @type.symbol symbol=Getters type={ [K in keyof T as `get${Capitalize<string & K>}`]: Function<(), T[K]> }
/// @definition.type symbol=Getters template=(T) value={ [K in keyof T as `get${Capitalize<string & K>}`]: Function<(), T[K]> }
/// @type.symbol symbol=Getters.T source=T type=T

    [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K];
    /// @generic.template source=mapped_type_parameter parent=template#0 parameters=(K: keyof T)
    /// @type.symbol symbol=Getters.K source=[K in keyof T as `get${Capitalize<string & K>}`] type=K
    /// @resolution.name source=T target=Getters.T
    /// @resolution.name source=Capitalize target=types.string.Capitalize
    /// @resolution.name source=K target=Getters.K
    /// @resolution.name source=T target=Getters.T
    /// @resolution.name source=K target=Getters.K

};

type Person = {
/// @type.symbol symbol=Person type={ name: string; age: int32 }
/// @definition.type symbol=Person value={ name: string; age: int32 }

    name: string;
    age: int32;
};

declare const getters: Getters<Person>;
/// @type.symbol symbol=getters source=getters type=Getters<Person> reduced={ getName: Function<(), string>; getAge: Function<(), int32> }
/// @resolution.pattern source=getters kind=binding target=getters
/// @resolution.name source=Getters target=Getters
/// @resolution.name source=Person target=Person

getters.getName satisfies () => string;
/// @resolution.name source=getters target=getters
/// @resolution.member source=getters.getName receiver={ getName: Function<(), Person["name"]>; getAge: Function<(), Person["age"]> } type=Function<(), Person["name"]> kind=field target_receiver={ getName: Function<(), Person["name"]>; getAge: Function<(), Person["age"]> } key=getName target_type=Function<(), Person["name"]>
/// @resolution.place source=getters placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=getters root=getters
/// @resolution.place source=getters.getName placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=getters.getName root=getters keys=[getName]

getters.getAge satisfies () => int32;
/// @resolution.name source=getters target=getters
/// @resolution.member source=getters.getAge receiver={ getName: Function<(), Person["name"]>; getAge: Function<(), Person["age"]> } type=Function<(), Person["age"]> kind=field target_receiver={ getName: Function<(), Person["name"]>; getAge: Function<(), Person["age"]> } key=getAge target_type=Function<(), Person["age"]>
/// @resolution.place source=getters placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=getters root=getters
/// @resolution.place source=getters.getAge placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=getters.getAge root=getters keys=[getAge]

/// @generic.instance id="Capitalize<string & K>" template=types.string.Capitalize arguments=(string & K)
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
/// @definition.type symbol=Handlers template=(T) value={ [K in keyof T as `on-${K}`]: T[K] }
/// @type.symbol symbol=Handlers.T source=T type=T

    [K in keyof T as `on-${K}`]: T[K];
    /// @generic.template source=mapped_type_parameter parent=template#0 parameters=(K: keyof T)
    /// @type.symbol symbol=Handlers.K source=[K in keyof T as `on-${K}`] type=K
    /// @resolution.name source=T target=Handlers.T
    /// @resolution.name source=K target=Handlers.K
    /// @resolution.name source=T target=Handlers.T
    /// @resolution.name source=K target=Handlers.K

};

type Value = Handlers<{ name: string }>["on-name"];
/// @type.symbol symbol=Value source="type Value = Handlers<{ name: string }>[\"on-name\"]" type=Handlers<{ name: string }>["on-name"] reduced=string
/// @definition.type symbol=Value source="type Value = Handlers<{ name: string }>[\"on-name\"]" value=Handlers<{ name: string }>["on-name"] reduced=string
/// @resolution.name source=Handlers target=Handlers

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value reduced=string
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

value satisfies string;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

/// @generic.instance id="Handlers<{ name: string }>" template=Handlers arguments=({ name: string })
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
/// @type.symbol symbol=Events type={ userCreated: string; orderPaid: int32 }
/// @definition.type symbol=Events value={ userCreated: string; orderPaid: int32 }

    userCreated: string;
    orderPaid: int32;
};

type Names<T> = {
/// @generic.template symbol=Names parameters=(T)
/// @type.symbol symbol=Names type={ [K in keyof T as K extends `${infer Name}Created` ? Names.Name : never]: T[K] }
/// @definition.type symbol=Names template=(T) value={ [K in keyof T as K extends `${infer Name}Created` ? Names.Name : never]: T[K] }
/// @type.symbol symbol=Names.T source=T type=T

    [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K];
    /// @generic.template source=mapped_type_parameter parent=template#0 parameters=(K: keyof T)
    /// @type.symbol symbol=Names.K source=[K in keyof T as K extends `${infer Name}Created` ? Name : never] type=K
    /// @resolution.name source=T target=Names.T
    /// @resolution.name source=K target=Names.K
    /// @resolution.name source=Name target=Names.Name
    /// @resolution.name source=T target=Names.T
    /// @resolution.name source=K target=Names.K

};

declare const names: Names<Events>;
/// @type.symbol symbol=names source=names type=Names<Events> reduced={ user: string }
/// @resolution.pattern source=names kind=binding target=names
/// @resolution.name source=Names target=Names
/// @resolution.name source=Events target=Events

names.user satisfies string;
/// @resolution.name source=names target=names
/// @resolution.member source=names.user receiver={ user: string } type=string kind=field target_receiver={ user: string } key=user target_type=string
/// @resolution.place source=names placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=names root=names
/// @resolution.place source=names.user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=names.user root=names keys=[user]

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
/// @type.symbol symbol=Events type={ userCreated: string; orderPaid: int32 }
/// @definition.type symbol=Events value={ userCreated: string; orderPaid: int32 }

    userCreated: string;
    orderPaid: int32;
};

type Names<T> = {
/// @generic.template symbol=Names parameters=(T)
/// @type.symbol symbol=Names type={ [K in keyof T as K extends `${infer Name}Created` ? Names.Name : never]: T[K] }
/// @definition.type symbol=Names template=(T) value={ [K in keyof T as K extends `${infer Name}Created` ? Names.Name : never]: T[K] }
/// @type.symbol symbol=Names.T source=T type=T

    [K in keyof T as K extends `${infer Name}Created` ? Name : never]: T[K];
    /// @generic.template source=mapped_type_parameter parent=template#0 parameters=(K: keyof T)
    /// @type.symbol symbol=Names.K source=[K in keyof T as K extends `${infer Name}Created` ? Name : never] type=K
    /// @resolution.name source=T target=Names.T
    /// @resolution.name source=K target=Names.K
    /// @resolution.name source=Name target=Names.Name
    /// @resolution.name source=T target=Names.T
    /// @resolution.name source=K target=Names.K

};

declare const names: Names<Events>;
/// @type.symbol symbol=names source=names type=Names<Events> reduced={ user: string }
/// @resolution.pattern source=names kind=binding target=names
/// @resolution.name source=Names target=Names
/// @resolution.name source=Events target=Events

const missing = names.orderPaid;
/// @type.symbol symbol=missing source=missing type=<error>
/// @resolution.pattern source=missing kind=binding target=missing
/// @resolution.name source=names target=names
/// @resolution.place source=names placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=names root=names

/// @generic.instance id=Names<Events> template=Names arguments=(Events)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

const handlers: { on-open: true; on-close: false } = {
    "on-open": true,
    "on-close": false,
} satisfies HandlerMap<{ open: boolean; close: boolean }>;

handlers["on-open"] satisfies boolean;

=== checked ===
type HandlerMap<T> = {
/// @generic.template symbol=HandlerMap parameters=(T)
/// @type.symbol symbol=HandlerMap type={ [K in keyof T as `on-${K}`]: T[K] }
/// @definition.type symbol=HandlerMap template=(T) value={ [K in keyof T as `on-${K}`]: T[K] }
/// @type.symbol symbol=HandlerMap.T source=T type=T

    [K in keyof T as `on-${K}`]: T[K];
    /// @generic.template source=mapped_type_parameter parent=template#0 parameters=(K: keyof T)
    /// @type.symbol symbol=HandlerMap.K source=[K in keyof T as `on-${K}`] type=K
    /// @resolution.name source=T target=HandlerMap.T
    /// @resolution.name source=K target=HandlerMap.K
    /// @resolution.name source=T target=HandlerMap.T
    /// @resolution.name source=K target=HandlerMap.K

};

const handlers = {
/// @type.symbol symbol=handlers source=handlers type={ on-open: true; on-close: false }
/// @resolution.pattern source=handlers kind=binding target=handlers

    "on-open": true,
    "on-close": false,
} satisfies HandlerMap<{ open: boolean; close: boolean }>;
/// @resolution.name source=HandlerMap target=HandlerMap

handlers["on-open"] satisfies boolean;
/// @resolution.name source=handlers target=handlers
/// @resolution.place source="handlers[\"on-open\"]" placement="local" lifetime="static" access="exclusive"
/// @resolution.access source="handlers[\"on-open\"]" root=handlers keys=[on-open]
/// @resolution.subscript source="handlers[\"on-open\"]" type=true kind=member target="receiver={ on-open: true; on-close: false }, target=field(receiver={ on-open: true; on-close: false }, target=on-open, type=true), type=true"
/// @resolution.place source=handlers placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=handlers root=handlers
"#,
    );
}
