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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Topic<in out T: string> {
    type Channel = `topic:${T}`;
}

declare const channel: "topic:orders";
channel satisfies `topic:${"orders"}`;

=== dir ===
class Topic<T: string> {
/// @generic.template symbol=Topic parameters=(in out T: string)
/// @type.symbol symbol=Topic type=typeof Topic
/// @definition.class symbol=Topic template=(in out T: string)
/// @definition.associated.type symbol=Topic.Channel source="type Channel = `topic:${T}`" key=Channel value=`topic:${T}`
/// @type.symbol symbol=Topic.T source="T: string" type=T

    type Channel = `topic:${T}`;
    /// @type.symbol symbol=Topic.Channel source="type Channel = `topic:${T}`" type=`topic:${T}`
    /// @resolution.name source=T target=Topic.T

}

declare const channel: Topic<"orders">.Channel;
/// @type.symbol symbol=channel source=channel type="topic:orders"
/// @resolution.pattern source=channel kind=binding target=channel
/// @resolution.name source="Topic<\"orders\">.Channel" target=Topic.Channel
/// @resolution.name source=Topic target=Topic
/// @generic.instance id="Topic<\"orders\">" template=Topic arguments=("orders")

channel satisfies `topic:${"orders"}`;
/// @resolution.name source=channel target=channel
/// @resolution.place source=channel placement="local" lifetime="static" access="immutable"
/// @resolution.access source=channel root=channel
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface EventShape<in out T> {
    type Handlers = {
        [K in keyof T as `on-${K}`]: T[K];
    };
}

class Bus<in out T> implements EventShape<T> {}

declare const handlers: { on-ready: boolean; on-message: string };

handlers["on-ready"] satisfies boolean;
handlers["on-message"] satisfies string;

=== dir ===
interface EventShape<T> {
/// @generic.template symbol=EventShape parameters=(in out T#1, this: EventShape<T#1>)
/// @type.symbol symbol=EventShape type=EventShape
/// @definition.interface symbol=EventShape template=(in out T#1, this: EventShape<T#1>)
/// @definition.where symbol=EventShape relation=satisfies left=this right=EventShape<T#1>
/// @definition.associated.type symbol=EventShape.Handlers key=Handlers value={ [K in keyof T#1 as `on-${K}`]: T#1[K] }
/// @type.symbol symbol=EventShape.T source=T type=T#1

    type Handlers = {
    /// @type.symbol symbol=EventShape.Handlers type={ [K in keyof T#1 as `on-${K}`]: T#1[K] }
    /// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T#1)

        [K in keyof T as `on-${K}`]: T[K];
        /// @type.symbol symbol=EventShape.Handlers.K source=[K in keyof T as `on-${K}`] type=K
        /// @resolution.name source=T target=EventShape.T
        /// @resolution.name source=K target=EventShape.Handlers.K
        /// @resolution.name source=T target=EventShape.T
        /// @resolution.name source=K target=EventShape.Handlers.K

    };
}

class Bus<T> implements EventShape<T> {}
/// @generic.template symbol=Bus parameters=(in out T#2)
/// @type.symbol symbol=Bus source="class Bus<T> implements EventShape<T> {}" type=typeof Bus
/// @generic.instance id=EventShape<T#2> template=EventShape arguments=(T#2)
/// @definition.class symbol=Bus source="class Bus<T> implements EventShape<T> {}" template=(in out T#2)
/// @definition.where symbol=Bus source=EventShape<T> relation=satisfies left=this right=EventShape<T#2>
/// @definition.implements symbol=Bus source=EventShape<T> target=EventShape<T#2>
/// @definition.conformance symbol=Bus member=EventShape.Handlers requirement=EventShape.Handlers
/// @type.symbol symbol=Bus.T source=T type=T#2
/// @resolution.name source=EventShape target=EventShape
/// @resolution.name source=T target=Bus.T

declare const handlers: Bus<{ ready: boolean; message: string }>.Handlers;
/// @type.symbol symbol=handlers source=handlers type={ on-ready: boolean; on-message: string }
/// @resolution.pattern source=handlers kind=binding target=handlers
/// @resolution.name source="Bus<{ ready: boolean; message: string }>.Handlers" target=EventShape.Handlers
/// @resolution.name source=Bus target=Bus
/// @generic.instance id="Bus<{ ready: boolean; message: string }>" template=Bus arguments=({ ready: boolean; message: string })
/// @type.symbol symbol=ready source="ready: boolean" type=boolean
/// @type.symbol symbol=message source="message: string" type=string

handlers["on-ready"] satisfies boolean;
/// @resolution.name source=handlers target=handlers
/// @resolution.place source="handlers[\"on-ready\"]" placement="local" lifetime="managed" access="mutable"
/// @resolution.access source="handlers[\"on-ready\"]" root=handlers keys=[on-ready]
/// @resolution.subscript source="handlers[\"on-ready\"]" type=boolean kind=member target="receiver={ on-ready: boolean; on-message: string }, target=field(receiver={ on-ready: boolean; on-message: string }, target=on-ready, type=boolean), type=boolean"
/// @resolution.place source=handlers placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handlers root=handlers

handlers["on-message"] satisfies string;
/// @resolution.name source=handlers target=handlers
/// @resolution.place source="handlers[\"on-message\"]" placement="local" lifetime="managed" access="mutable"
/// @resolution.access source="handlers[\"on-message\"]" root=handlers keys=[on-message]
/// @resolution.subscript source="handlers[\"on-message\"]" type=string kind=member target="receiver={ on-ready: boolean; on-message: string }, target=field(receiver={ on-ready: boolean; on-message: string }, target=on-message, type=string), type=string"
/// @resolution.place source=handlers placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handlers root=handlers
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class EventName<in out T: string> {
    type Kind = T extends `evt:${infer Name}` ? Name : never;
}

declare const kind: "login";
kind satisfies "login";

=== dir ===
class EventName<T: string> {
/// @generic.template symbol=EventName parameters=(in out T: string)
/// @type.symbol symbol=EventName type=typeof EventName
/// @definition.class symbol=EventName template=(in out T: string)
/// @definition.associated.type symbol=EventName.Kind source="type Kind = T extends `evt:${infer Name}` ? Name : never" key=Kind value="T extends `evt:${infer Name}` ? EventName.Kind.Name : never"
/// @type.symbol symbol=EventName.T source="T: string" type=T

    type Kind = T extends `evt:${infer Name}` ? Name : never;
    /// @type.symbol symbol=EventName.Kind source="type Kind = T extends `evt:${infer Name}` ? Name : never" type=T extends `evt:${infer Name}` ? EventName.Kind.Name : never
    /// @resolution.name source=T target=EventName.T
    /// @resolution.name source=Name target=EventName.Kind.Name

}

declare const kind: EventName<"evt:login">.Kind;
/// @type.symbol symbol=kind source=kind type="login"
/// @resolution.pattern source=kind kind=binding target=kind
/// @resolution.name source="EventName<\"evt:login\">.Kind" target=EventName.Kind
/// @resolution.name source=EventName target=EventName
/// @generic.instance id="EventName<\"evt:login\">" template=EventName arguments=("evt:login")

kind satisfies "login";
/// @resolution.name source=kind target=kind
/// @resolution.place source=kind placement="local" lifetime="static" access="immutable"
/// @resolution.access source=kind root=kind
"#,
    );
}
