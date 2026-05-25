use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_lambda_captures() {
    let session = TestSession::single(
        r#"
let count = 1;
const next = () => count + 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
let count = 1;
/// @type.symbol symbol=count type=int32

const next = () => count + 1;
/// @type.symbol symbol=next type=Function<(), int32>
/// @resolution.name source=count target=count
/// @type.node source="count + 1" type=int32
/// @capture.function function=next bindings=1 frames=[main.<frame0>]
/// @capture.output function=next symbol=count mode=manage type=int32 frame=main.<frame0>
/// @capture.frame frame=main.<frame0> scope=<module> type=Managed<{ count: int32 }> fields=[count: int32]
"#,
    );
}

#[test]
fn test_check_records_binding_frame_captures() {
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
        DirRows::checked(),
        r#"
let a = 0;
/// @type.symbol symbol=a type=int32

let b = 0;
/// @type.symbol symbol=b type=int32

let c = 0;
/// @type.symbol symbol=c type=int32

const foo = () => {
/// @type.symbol symbol=foo type=Function<(), void>

    a += 1;
    /// @resolution.name source=a target=a
    /// @type.node source="a += 1" type=int32

    b += 1;
    /// @resolution.name source=b target=b
    /// @type.node source="b += 1" type=int32
};
/// @capture.function function=foo bindings=2 frames=[main.<frame0>]
/// @capture.output function=foo symbol=a mode=manage type=int32 frame=main.<frame0>
/// @capture.output function=foo symbol=b mode=manage type=int32 frame=main.<frame0>
/// @capture.frame frame=main.<frame0> scope=<module> type=Managed<{ a: int32; b: int32; c: int32 }> fields=[a: int32, b: int32, c: int32]

const boo = () => {
/// @type.symbol symbol=boo type=Function<(), void>

    b += 1;
    /// @resolution.name source=b target=b
    /// @type.node source="b += 1" type=int32

    c += 1;
    /// @resolution.name source=c target=c
    /// @type.node source="c += 1" type=int32
};
/// @capture.function function=boo bindings=2 frames=[main.<frame0>]
/// @capture.output function=boo symbol=b mode=manage type=int32 frame=main.<frame0>
/// @capture.output function=boo symbol=c mode=manage type=int32 frame=main.<frame0>
"#,
    );
}

#[test]
fn test_check_records_capture_directives() {
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
        DirRows::checked(),
        r#"
let count = 1;
/// @type.symbol symbol=count type=int32

let step = 2;
/// @type.symbol symbol=step type=int32

@capture({
    default: "manage",
    step: "copy",
})
const next = () => count + step;
/// @type.symbol symbol=next type=Function<(), int32>
/// @resolution.name source=count target=count
/// @resolution.name source=step target=step
/// @type.node source="count + step" type=int32
/// @capture.function function=next bindings=2 frames=[main.<frame0>]
/// @capture.output function=next symbol=count mode=manage type=int32 frame=main.<frame0>
/// @capture.output function=next symbol=step mode=copy type=int32
/// @capture.directive function=next default=manage rules=1
/// @capture.rule function=next binding=step mode=copy
/// @capture.frame frame=main.<frame0> scope=<module> type=Managed<{ count: int32 }> fields=[count: int32]
"#,
    );
}

#[test]
fn test_check_records_mixed_owned_callable_and_binding_captures() {
    let session = TestSession::single(
        r#"
struct Socket {
    write(message: string): void;
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
        DirRows::checked(),
        r#"
struct Socket {
/// @type.symbol symbol=Socket type=Socket

    write(message: string): void;
    /// @type.symbol symbol=Socket.write type=(this: Socket, string) => void
}

let count = 0;
/// @type.symbol symbol=count type=int32

let socket = Socket {};
/// @type.symbol symbol=socket type=Socket

@capture({
    default: "manage",
    socket: "move",
})
const send: ^Function<(string,), void> = (message) => {
/// @type.symbol symbol=send type=Owned<Function<(string,), void>>

    count += 1;
    /// @resolution.name source=count target=count
    /// @type.node source="count += 1" type=int32

    socket.write(message);
    /// @resolution.name source=socket target=socket
    /// @resolution.member source=socket.write receiver=Socket kind=symbol target=Socket.write
    /// @resolution.name source=message target=message
    /// @resolution.call source="socket.write(message)" parameters=[string] return=void kind=symbol target=Socket.write receiver=Socket
};
/// @capture.function function=send bindings=2 frames=[main.<frame0>]
/// @capture.output function=send symbol=count mode=manage type=int32 frame=main.<frame0>
/// @capture.output function=send symbol=socket mode=move type=Socket
/// @capture.directive function=send default=manage rules=1
/// @capture.rule function=send binding=socket mode=move
/// @capture.frame frame=main.<frame0> scope=<module> type=Managed<{ count: int32 }> fields=[count: int32]
"#,
    );
}

#[test]
fn test_check_records_copy_captures_for_managed_callbacks() {
    let session = TestSession::single(
        r#"
class Client {
    read(): Promise<string>;
}

let client = new Client();

@capture("copy")
const load = async () => await client.read();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
class Client {
/// @type.symbol symbol=Client type=Client

    read(): Promise<string>;
    /// @type.symbol symbol=Client.read type=(this: Client) => Promise<string>
}

let client = new Client();
/// @resolution.name source=Client target=Client
/// @resolution.call source="new Client()" parameters=[] return=Client kind=construct target=Client.constructor
/// @type.symbol symbol=client type=Client

@capture("copy")
const load = async () => await client.read();
/// @type.symbol symbol=load type=Function<(), Promise<string>>
/// @resolution.name source=client target=client
/// @resolution.member source=client.read receiver=Client kind=symbol target=Client.read
/// @resolution.call source="client.read()" parameters=[] return=Promise<string> kind=symbol target=Client.read receiver=Client
/// @type.node source="await client.read()" type=string
/// @capture.function function=load bindings=1
/// @capture.output function=load symbol=client mode=copy type=Client
/// @capture.directive function=load default=copy rules=0
"#,
    );
}

#[test]
fn test_check_records_lexical_this_capture() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32;

    make(): () => int32 {
        return () => this.value;
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
class Counter {
/// @type.symbol symbol=Counter type=Counter

    value: int32;
    /// @type.symbol symbol=Counter.value type=int32

    make(): () => int32 {
    /// @type.symbol symbol=Counter.make type=(this: Counter) => Function<(), int32>

        return () => this.value;
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
        /// @type.node source=this.value type=int32
        /// @capture.function function=Counter.make.<lambda0> bindings=0
        /// @capture.receiver function=Counter.make.<lambda0> symbol=this mode=manage type=Counter
    }
}
"#,
    );
}
