use crate::tests::{DirRows, TestSession};

#[test]
fn test_associated_alias_uses_outer_template_argument() {
    let session = TestSession::single(
        r#"
class Topic<T: string> {
    type Channel = `topic:${T}`;
}

declare const channel: Topic<"orders">.Channel;
channel satisfies `topic:${"orders"}`;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Topic<T: string> {
    type Channel = `topic:${T}`;
}

declare const channel: Topic<"orders">.Channel;
channel satisfies `topic:${"orders"}`;

=== checked ===
class Topic<T: string> {
/// @generic.template symbol=Topic parameters=(T: string)
/// @type.symbol symbol=Topic type=<T: string> Topic<T>

    type Channel = `topic:${T}`;
    /// @type.symbol symbol=Topic.Channel type=`topic:${T}`
    /// @resolution.name source=T target=Topic.T
}

declare const channel: Topic<"orders">.Channel;
/// @type.symbol symbol=channel source=channel type=`topic:${"orders"}`
/// @resolution.name source=Topic target=Topic
/// @generic.instance source="Topic<\"orders\">" id="Topic<\"orders\">"

channel satisfies `topic:${"orders"}`;
/// @resolution.name source=channel target=channel
/// @generic.instance id="Topic<\"orders\">" template=Topic arguments=("orders")
"#,
    );
}

#[test]
fn test_associated_alias_remaps_template_keys() {
    let session = TestSession::single(
        r#"
interface EventShape<T> {
    type Handlers = {
        [K in keyof T as `on-${K}`]: T[K];
    };
}

class Bus<T> implements EventShape<T> {}

declare const handlers: Bus<{ ready: boolean; message: string }>.Handlers;

handlers["on-ready"] satisfies boolean;
handlers["on-message"] satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface EventShape<T> {
    type Handlers = {
        [K in keyof T as `on-${K}`]: T[K];
    };
}

class Bus<T> implements EventShape<T> {}

declare const handlers: Bus<{ ready: boolean; message: string }>.Handlers;

handlers["on-ready"] satisfies boolean;
handlers["on-message"] satisfies string;

=== checked ===
interface EventShape<T> {
/// @generic.template symbol=EventShape parameters=(T)
/// @type.symbol symbol=EventShape type=<T> EventShape<T>

    type Handlers = {
    /// @type.symbol symbol=EventShape.Handlers type={ [K in keyof T as `on-${K}`]: T[K] }

        [K in keyof T as `on-${K}`]: T[K];
    };
}

class Bus<T> implements EventShape<T> {}
/// @generic.template symbol=Bus parameters=(T)
/// @type.symbol symbol=Bus type=<T> Bus<T>
/// @resolution.name source=EventShape target=EventShape
/// @resolution.name source=T target=Bus.T

declare const handlers: Bus<{ ready: boolean; message: string }>.Handlers;
/// @type.symbol symbol=handlers source=handlers type={ "on-ready": boolean; "on-message": string }
/// @resolution.name source=Bus target=Bus
/// @generic.instance source="Bus<{ ready: boolean; message: string }>" id="Bus<{ ready: boolean; message: string }>"

handlers["on-ready"] satisfies boolean;
/// @resolution.name source=handlers target=handlers
/// @resolution.member source="handlers[\"on-ready\"]" receiver={ "on-ready": boolean; "on-message": string } kind=field key=on-ready

handlers["on-message"] satisfies string;
/// @resolution.name source=handlers target=handlers
/// @resolution.member source="handlers[\"on-message\"]" receiver={ "on-ready": boolean; "on-message": string } kind=field key=on-message
/// @generic.instance id="Bus<{ ready: boolean; message: string }>" template=Bus arguments=({ ready: boolean; message: string })
"#,
    );
}

#[test]
fn test_associated_alias_infers_template_span() {
    let session = TestSession::single(
        r#"
class EventName<T: string> {
    type Kind = T extends `evt:${infer Name}` ? Name : never;
}

declare const kind: EventName<"evt:login">.Kind;
kind satisfies "login";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class EventName<T: string> {
    type Kind = T extends `evt:${infer Name}` ? Name : never;
}

declare const kind: EventName<"evt:login">.Kind;
kind satisfies "login";

=== checked ===
class EventName<T: string> {
/// @generic.template symbol=EventName parameters=(T: string)
/// @type.symbol symbol=EventName type=<T: string> EventName<T>

    type Kind = T extends `evt:${infer Name}` ? Name : never;
    /// @type.symbol symbol=EventName.Kind type=T extends `evt:${infer Name}` ? Name : never
}

declare const kind: EventName<"evt:login">.Kind;
/// @type.symbol symbol=kind source=kind type="login"
/// @resolution.name source=EventName target=EventName
/// @generic.instance source="EventName<\"evt:login\">" id="EventName<\"evt:login\">"

kind satisfies "login";
/// @resolution.name source=kind target=kind
/// @generic.instance id="EventName<\"evt:login\">" template=EventName arguments=("evt:login")
"#,
    );
}
