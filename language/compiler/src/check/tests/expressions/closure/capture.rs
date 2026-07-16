use crate::tests::{DirRows, TestSession};

#[test]
fn test_closure_captures_lexical_binding() {
    let session = TestSession::single(
        r#"
let count = 1;
const next = () => count + 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_capture(),
        r#"
=== annotated ===
let count: float64 = 1;
const next: () => float64 = (): float64 => count + 1;

=== checked ===
let count = 1;
/// @type.symbol symbol=count source=count type=float64
/// @type.node source=1 type=1

const next = () => count + 1;
/// @type.symbol symbol=next source=next type=Function<(), float64>
/// @type.symbol symbol=symbol2 source="() => count + 1" type=Function<(), float64>
/// @type.node source="() => count + 1" type=Function<(), float64>
/// @capture.function function=symbol2 bindings=1 frames=(main.<frame0>)
/// @capture.binding function=symbol2 symbol=count mode=manage type=float64 frame=main.<frame0>
/// @capture.frame frame=main.<frame0> scope=<module> type=Managed<{ count: float64 }> fields={ count: float64 }
/// @type.node source="count + 1" type=float64
/// @type.node source=count type=float64
/// @resolution.name source=count target=count
/// @resolution.call source="count + 1" parameters=() return=float64 kind=builtin builtin=binary.add
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_sibling_closures_share_capture_frame() {
    let session = TestSession::single(
        r#"
let a = 0;
let b = 0;
let c = 0;

const foo = () => {
    a += 1;
    b += 1;
};

const boo = () => {
    b += 1;
    c += 1;
};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_capture(),
        r#"
=== annotated ===
let a: float64 = 0;
let b: float64 = 0;
let c: float64 = 0;

const foo: () => void = (): void => {
    a += 1;
    b += 1;
};

const boo: () => void = (): void => {
    b += 1;
    c += 1;
};

=== checked ===
let a = 0;
/// @type.symbol symbol=a source=a type=float64
/// @type.node source=0 type=0

let b = 0;
/// @type.symbol symbol=b source=b type=float64
/// @type.node source=0 type=0

let c = 0;
/// @type.symbol symbol=c source=c type=float64
/// @type.node source=0 type=0

const foo = () => {
/// @type.symbol symbol=foo source=foo type=Function<(), void>
/// @type.symbol symbol=symbol4 type=Function<(), void>
/// @type.node type=Function<(), void>
/// @capture.function function=symbol4 bindings=2 frames=(main.<frame0>)
/// @capture.binding function=symbol4 symbol=a mode=manage type=float64 frame=main.<frame0>
/// @capture.binding function=symbol4 symbol=b mode=manage type=float64 frame=main.<frame0>
/// @capture.frame frame=main.<frame0> scope=<module> type=Managed<{ a: float64; b: float64; c: float64 }> fields={ a: float64, b: float64, c: float64 }

    a += 1;
    /// @type.node source="a += 1" type=float64
    /// @type.node source=a type=float64
    /// @resolution.call source="a += 1" parameters=() return=float64 kind=builtin builtin=binary.add
    /// @resolution.pattern.assign source=a kind=place place=binding(a) type=float64
    /// @type.node source=1 type=1

    b += 1;
    /// @type.node source="b += 1" type=float64
    /// @type.node source=b type=float64
    /// @resolution.call source="b += 1" parameters=() return=float64 kind=builtin builtin=binary.add
    /// @resolution.pattern.assign source=b kind=place place=binding(b) type=float64
    /// @type.node source=1 type=1

};

const boo = () => {
/// @type.symbol symbol=boo source=boo type=Function<(), void>
/// @type.symbol symbol=symbol6 type=Function<(), void>
/// @type.node type=Function<(), void>
/// @capture.function function=symbol6 bindings=2 frames=(main.<frame0>)
/// @capture.binding function=symbol6 symbol=b mode=manage type=float64 frame=main.<frame0>
/// @capture.binding function=symbol6 symbol=c mode=manage type=float64 frame=main.<frame0>

    b += 1;
    /// @type.node source="b += 1" type=float64
    /// @type.node source=b type=float64
    /// @resolution.call source="b += 1" parameters=() return=float64 kind=builtin builtin=binary.add
    /// @resolution.pattern.assign source=b kind=place place=binding(b) type=float64
    /// @type.node source=1 type=1

    c += 1;
    /// @type.node source="c += 1" type=float64
    /// @type.node source=c type=float64
    /// @resolution.call source="c += 1" parameters=() return=float64 kind=builtin builtin=binary.add
    /// @resolution.pattern.assign source=c kind=place place=binding(c) type=float64
    /// @type.node source=1 type=1

};
"#,
    );
}

#[test]
fn test_capture_directives_select_capture_mode() {
    let session = TestSession::single(
        r#"
let count = 1;
let step = 2;

@capture({
    default: "manage",
    step: "copy",
})
const next = () => count + step;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_capture(),
        r#"
=== annotated ===
let count: float64 = 1;
let step: float64 = 2;

@capture({
    default: "manage",
    step: "copy",
})
const next: () => float64 = (): float64 => count + step;

=== checked ===
let count = 1;
/// @type.symbol symbol=count source=count type=float64
/// @type.node source=1 type=1

let step = 2;
/// @type.symbol symbol=step source=step type=float64
/// @type.node source=2 type=2

@capture({
/// @resolution.name source=capture target=decorator.capture.capture

    default: "manage",
    step: "copy",
})
const next = () => count + step;
/// @type.symbol symbol=next source=next type=Function<(), float64>
/// @type.symbol symbol=symbol3 source="() => count + step" type=Function<(), float64>
/// @type.node source="() => count + step" type=Function<(), float64>
/// @capture.function function=symbol3 bindings=2 frames=(main.<frame0>)
/// @capture.binding function=symbol3 symbol=count mode=manage type=float64 frame=main.<frame0>
/// @capture.binding function=symbol3 symbol=step mode=copy type=float64
/// @capture.directive function=symbol3 default=manage rules=1
/// @capture.rule function=symbol3 binding=step mode=copy
/// @capture.frame frame=main.<frame0> scope=<module> type=Managed<{ count: float64 }> fields={ count: float64 }
/// @type.node source="count + step" type=float64
/// @type.node source=count type=float64
/// @resolution.name source=count target=count
/// @resolution.call source="count + step" parameters=() return=float64 kind=builtin builtin=binary.add
/// @type.node source=step type=float64
/// @resolution.name source=step target=step
"#,
    );
}

#[test]
fn test_owned_callable_captures_mixed_capture_modes() {
    let session = TestSession::single(
        r#"
struct Socket {
    write(message: string): void {}
}

let count = 0;
let socket = Socket {};

@capture({
    default: "manage",
    socket: "move",
})
const send: ^Function<(string,), void> = (message) => {
    count += 1;
    socket.write(message);
};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_capture(),
        r#"
=== annotated ===
struct Socket {
    write(message: string): void {}
}

let count: float64 = 0;
let socket: Socket = Socket {};

@capture({
    default: "manage",
    socket: "move",
})
const send: ^Function<(string,), void> = ((message: string): void => {
    count += 1;
    socket.write(message);
}) as ^Function<(string,), void>;

=== checked ===
struct Socket {
/// @type.symbol symbol=Socket type=Socket
/// @definition.struct symbol=Socket
/// @definition.method symbol=Socket.write source="write(message: string): void {}" slot=write type=(this: this, string) => void

    write(message: string): void {}
    /// @type.symbol symbol=Socket.write source="write(message: string): void {}" type=(this: this, string) => void
    /// @capture.function function=Socket.write bindings=0
    /// @type.symbol symbol=Socket.write.message source="message: string" type=string

}

let count = 0;
/// @type.symbol symbol=count source=count type=float64
/// @type.node source=0 type=0

let socket = Socket {};
/// @type.symbol symbol=socket source=socket type=Socket
/// @type.node source="Socket {}" type=Socket
/// @resolution.name source=Socket target=Socket

@capture({
/// @resolution.name source=capture target=decorator.capture.capture

    default: "manage",
    socket: "move",
})
const send: ^Function<(string,), void> = (message) => {
/// @type.symbol symbol=send source=send type=Owned<Function<(string,), void>>
/// @resolution.name source=Function target=types.function.Function
/// @type.symbol symbol=symbol7 type=Function<(string,), void>
/// @type.node type=Function<(string,), void>
/// @capture.function function=symbol7 bindings=2 frames=(main.<frame0>)
/// @capture.binding function=symbol7 symbol=count mode=manage type=float64 frame=main.<frame0>
/// @capture.binding function=symbol7 symbol=socket mode=move type=Socket
/// @capture.directive function=symbol7 default=manage rules=1
/// @capture.rule function=symbol7 binding=socket mode=move
/// @capture.frame frame=main.<frame0> scope=<module> type=Managed<{ count: float64 }> fields={ count: float64 }
/// @type.symbol symbol=symbol7.message source=message type=string

    count += 1;
    /// @type.node source="count += 1" type=float64
    /// @type.node source=count type=float64
    /// @resolution.call source="count += 1" parameters=() return=float64 kind=builtin builtin=binary.add
    /// @resolution.pattern.assign source=count kind=place place=binding(count) type=float64
    /// @type.node source=1 type=1

    socket.write(message);
    /// @type.node source=socket type=Socket
    /// @type.node source=socket.write type=(this: Socket, string) => void
    /// @type.node source=socket.write(message) type=void
    /// @resolution.name source=socket target=socket
    /// @resolution.member source=socket.write receiver=Socket kind=symbol target=Socket.write
    /// @resolution.call source=socket.write(message) parameters=(string) arguments=(provided(message) as string) return=void kind=symbol target=Socket.write receiver=Socket
    /// @type.node source=message type=string
    /// @resolution.name source=message target=symbol7.message

};

/// @generic.instance id="Function<(string,), void>" template=types.function.Function arguments=((string,), void)
"#,
    );
}

#[test]
fn test_managed_closure_captures_copy_binding() {
    let session = TestSession::single(
        r#"
declare class Client {
    read(): Promise<string>;
}

let client = new Client();

@capture("copy")
const load = async () => await client.read();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_capture(),
        r#"
=== annotated ===
declare class Client {
    read(): Promise<string>;
}

let client: Client = new Client();

@capture("copy")
const load = async () => await client.read();

=== checked ===
declare class Client {
/// @type.symbol symbol=Client type=Client
/// @definition.class symbol=Client
/// @definition.method symbol=Client.read source="read(): Promise<string>" slot=read type=(this: this) => Promise<string>

    read(): Promise<string>;
    /// @type.symbol symbol=Client.read source="read(): Promise<string>" type=(this: this) => Promise<string>
    /// @resolution.name source=Promise target=async.promise.Promise

}

let client = new Client();
/// @type.symbol symbol=client source=client type=Client
/// @type.node source="new Client()" type=Client
/// @resolution.construct source="new Client()" parameters=() return=Client kind=class target=Client constructor=default
/// @resolution.name source=Client target=Client

@capture("copy")
/// @resolution.name source=capture target=decorator.capture.capture

const load = async () => await client.read();
/// @type.symbol symbol=load source=load type=Function<(), Promise<string>>
/// @type.symbol symbol=symbol5 source="async () => await client.read()" type=Function<(), Promise<string>>
/// @type.node source="async () => await client.read()" type=Function<(), Promise<string>>
/// @generic.instance source="async () => await client.read()" id=Promise<string>
/// @capture.function function=symbol5 bindings=1
/// @capture.binding function=symbol5 symbol=client mode=copy type=Client
/// @capture.directive function=symbol5 default=copy rules=0
/// @type.node source="await client.read()" type=string
/// @type.node source=client type=Client
/// @type.node source=client.read type=(this: Client) => Promise<string>
/// @type.node source=client.read() type=Promise<string>
/// @resolution.name source=client target=client
/// @resolution.member source=client.read receiver=Client kind=symbol target=Client.read
/// @resolution.call source=client.read() parameters=() return=Promise<string> kind=symbol target=Client.read receiver=Client
/// @generic.instance source=client.read id=Promise<string>
/// @generic.instance source=client.read() id=Promise<string>

/// @generic.instance id=Promise<string> template=async.promise.Promise arguments=(string)
"#,
    );
}

#[test]
fn test_closure_captures_lexical_this_receiver() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32 = 0;

    make(): () => int32 {
        return () => this.value;
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_capture(),
        r#"
=== annotated ===
class Counter {
    value: int32 = 0;

    make(): () => int32 {
        return (): int32 => this.value;
    }
}

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Counter.make slot=make type=(this: this) => Function<(), int32>

    value: int32 = 0;
    /// @type.symbol symbol=Counter.value source="value: int32 = 0" type=int32
    /// @type.node source=0 type=0

    make(): () => int32 {
    /// @type.symbol symbol=Counter.make type=(this: this) => Function<(), int32>
    /// @capture.function function=Counter.make bindings=0

        return () => this.value;
        /// @type.symbol symbol=Counter.make.symbol6 source="() => this.value" type=Function<(), int32>
        /// @type.node source="() => this.value" type=Function<(), int32>
        /// @capture.function function=Counter.make.symbol6 bindings=0
        /// @capture.receiver function=Counter.make.symbol6 symbol=this mode=manage type=Counter
        /// @type.node source=this type=Counter
        /// @type.node source=this.value type=int32
        /// @resolution.name source=this target=Counter.make.this
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value

    }
}
"#,
    );
}
