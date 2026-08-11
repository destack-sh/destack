use crate::tests::{DirRows, TestSession};

#[test]
fn test_closure_captures_lexical_binding() {
    let session = TestSession::single(
        r#"
function make(): () => float64 {
    let count = 1;
    const next = () => count + 1;
    return next;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_capture(),
        r#"
=== annotated ===
function make(): () => float64 {
    let count: float64 = 1;
    const next: () => float64 = (): float64 => count + 1;
    return next;
}

=== checked ===
function make(): () => float64 {
/// @type.symbol symbol=make type=() => Function<(), float64>
/// @capture.function function=make bindings=0

    let count = 1;
    /// @type.symbol symbol=make.count source=count type=float64
    /// @resolution.pattern source=count kind=binding target=make.count
    /// @type.node source=1 type=1

    const next = () => count + 1;
    /// @type.symbol symbol=make.next source=next type=Function<(), float64>
    /// @resolution.pattern source=next kind=binding target=make.next
    /// @type.symbol symbol=make.symbol3 source="() => count + 1" type=Function<(), float64>
    /// @type.node source="() => count + 1" type=Function<(), float64>
    /// @capture.function function=make.symbol3 bindings=1 frames=(main.<frame0>)
    /// @capture.binding function=make.symbol3 symbol=count mode=manage type=float64 frame=main.<frame0>
    /// @capture.frame frame=main.<frame0> scope=scope4 type=Managed<{ count: float64 }> fields={ count: float64 }
    /// @type.node source="count + 1" type=float64
    /// @type.node source=count type=float64
    /// @resolution.name source=count target=make.count
    /// @resolution.operator source="count + 1" type=float64 operator="+" kind=builtin operands=[count as float64 families=(float), 1 as float64 families=(float)]
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=make.count
    /// @type.node source=1 type=1

    return next;
    /// @type.node source=next type=Function<(), float64>
    /// @resolution.name source=next target=make.next
    /// @resolution.place source=next placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=next root=make.next

}
"#,
    );
}

#[test]
fn test_sibling_closures_share_capture_frame() {
    let session = TestSession::single(
        r#"
function run(): void {
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

    foo();
    boo();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_capture(),
        r#"
=== annotated ===
function run(): void {
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

    foo();
    boo();
}

=== checked ===
function run(): void {
/// @type.symbol symbol=run type=() => void
/// @capture.function function=run bindings=0

    let a = 0;
    /// @type.symbol symbol=run.a source=a type=float64
    /// @resolution.pattern source=a kind=binding target=run.a
    /// @type.node source=0 type=0

    let b = 0;
    /// @type.symbol symbol=run.b source=b type=float64
    /// @resolution.pattern source=b kind=binding target=run.b
    /// @type.node source=0 type=0

    let c = 0;
    /// @type.symbol symbol=run.c source=c type=float64
    /// @resolution.pattern source=c kind=binding target=run.c
    /// @type.node source=0 type=0

    const foo = () => {
    /// @type.symbol symbol=run.foo source=foo type=Function<(), void>
    /// @resolution.pattern source=foo kind=binding target=run.foo
    /// @type.symbol symbol=run.symbol5 type=Function<(), void>
    /// @type.node type=Function<(), void>
    /// @capture.function function=run.symbol5 bindings=2 frames=(main.<frame0>)
    /// @capture.binding function=run.symbol5 symbol=a mode=manage type=float64 frame=main.<frame0>
    /// @capture.binding function=run.symbol5 symbol=b mode=manage type=float64 frame=main.<frame0>
    /// @capture.frame frame=main.<frame0> scope=scope3 type=Managed<{ a: float64; b: float64; c: float64 }> fields={ a: float64, b: float64, c: float64 }

        a += 1;
        /// @type.node source="a += 1" type=float64
        /// @type.node source=a type=float64
        /// @resolution.name source=a target=run.a
        /// @resolution.operator source="a += 1" type=float64 operator="+" kind=builtin operands=[a as float64 families=(float), 1 as float64 families=(float)]
        /// @resolution.pattern.assign source=a kind=place
        /// @resolution.place source=a placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=a read=binding(run.a) write=binding(run.a) type=float64
        /// @resolution.access source=a root=run.a
        /// @type.node source=1 type=1

        b += 1;
        /// @type.node source="b += 1" type=float64
        /// @type.node source=b type=float64
        /// @resolution.name source=b target=run.b
        /// @resolution.operator source="b += 1" type=float64 operator="+" kind=builtin operands=[b as float64 families=(float), 1 as float64 families=(float)]
        /// @resolution.pattern.assign source=b kind=place
        /// @resolution.place source=b placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=b read=binding(run.b) write=binding(run.b) type=float64
        /// @resolution.access source=b root=run.b
        /// @type.node source=1 type=1

    };

    const boo = () => {
    /// @type.symbol symbol=run.boo source=boo type=Function<(), void>
    /// @resolution.pattern source=boo kind=binding target=run.boo
    /// @type.symbol symbol=run.symbol7 type=Function<(), void>
    /// @type.node type=Function<(), void>
    /// @capture.function function=run.symbol7 bindings=2 frames=(main.<frame0>)
    /// @capture.binding function=run.symbol7 symbol=b mode=manage type=float64 frame=main.<frame0>
    /// @capture.binding function=run.symbol7 symbol=c mode=manage type=float64 frame=main.<frame0>

        b += 1;
        /// @type.node source="b += 1" type=float64
        /// @type.node source=b type=float64
        /// @resolution.name source=b target=run.b
        /// @resolution.operator source="b += 1" type=float64 operator="+" kind=builtin operands=[b as float64 families=(float), 1 as float64 families=(float)]
        /// @resolution.pattern.assign source=b kind=place
        /// @resolution.place source=b placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=b read=binding(run.b) write=binding(run.b) type=float64
        /// @resolution.access source=b root=run.b
        /// @type.node source=1 type=1

        c += 1;
        /// @type.node source="c += 1" type=float64
        /// @type.node source=c type=float64
        /// @resolution.name source=c target=run.c
        /// @resolution.operator source="c += 1" type=float64 operator="+" kind=builtin operands=[c as float64 families=(float), 1 as float64 families=(float)]
        /// @resolution.pattern.assign source=c kind=place
        /// @resolution.place source=c placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=c read=binding(run.c) write=binding(run.c) type=float64
        /// @resolution.access source=c root=run.c
        /// @type.node source=1 type=1

    };

    foo();
    /// @type.node source=foo type=Function<(), void>
    /// @type.node source=foo() type=void
    /// @resolution.name source=foo target=run.foo
    /// @resolution.call source=foo() parameters=() return=void kind=expression target=expression
    /// @resolution.place source=foo placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=foo root=run.foo

    boo();
    /// @type.node source=boo type=Function<(), void>
    /// @type.node source=boo() type=void
    /// @resolution.name source=boo target=run.boo
    /// @resolution.call source=boo() parameters=() return=void kind=expression target=expression
    /// @resolution.place source=boo placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boo root=run.boo

}
"#,
    );
}

#[test]
fn test_capture_directives_select_capture_mode() {
    let session = TestSession::single(
        r#"
function make(): () => float64 {
    let count = 1;
    let step = 2;

    @capture({
        default: "manage",
        step: "copy",
    })
    const next = () => count + step;
    return next;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_capture(),
        r#"
=== annotated ===
function make(): () => float64 {
    let count: float64 = 1;
    let step: float64 = 2;

    @capture({
        default: "manage",
        step: "copy",
    })
    const next: () => float64 = (): float64 => count + step;
    return next;
}

=== checked ===
function make(): () => float64 {
/// @type.symbol symbol=make type=() => Function<(), float64>
/// @capture.function function=make bindings=0

    let count = 1;
    /// @type.symbol symbol=make.count source=count type=float64
    /// @resolution.pattern source=count kind=binding target=make.count
    /// @type.node source=1 type=1

    let step = 2;
    /// @type.symbol symbol=make.step source=step type=float64
    /// @resolution.pattern source=step kind=binding target=make.step
    /// @type.node source=2 type=2

    @capture({
    /// @type.node source=capture type=capture
    /// @resolution.name source=capture target=decorator.capture.capture
    /// @type.node type={ default: "manage"; step: "copy" }

        default: "manage",
        /// @type.node source="\"manage\"" type="manage"

        step: "copy",
        /// @type.node source="\"copy\"" type="copy"

    })
    const next = () => count + step;
    /// @type.symbol symbol=make.next source=next type=Function<(), float64>
    /// @resolution.pattern source=next kind=binding target=make.next
    /// @type.symbol symbol=make.symbol4 source="() => count + step" type=Function<(), float64>
    /// @type.node source="() => count + step" type=Function<(), float64>
    /// @capture.function function=make.symbol4 bindings=2 frames=(main.<frame0>)
    /// @capture.binding function=make.symbol4 symbol=count mode=manage type=float64 frame=main.<frame0>
    /// @capture.binding function=make.symbol4 symbol=step mode=copy type=float64
    /// @capture.directive function=make.symbol4 default=manage rules=1
    /// @capture.rule function=make.symbol4 binding=step mode=copy
    /// @capture.frame frame=main.<frame0> scope=scope4 type=Managed<{ count: float64 }> fields={ count: float64 }
    /// @type.node source="count + step" type=float64
    /// @type.node source=count type=float64
    /// @resolution.name source=count target=make.count
    /// @resolution.operator source="count + step" type=float64 operator="+" kind=builtin operands=[count as float64 families=(float), step as float64 families=(float)]
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=make.count
    /// @type.node source=step type=float64
    /// @resolution.name source=step target=make.step
    /// @resolution.place source=step placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=step root=make.step

    return next;
    /// @type.node source=next type=Function<(), float64>
    /// @resolution.name source=next target=make.next
    /// @resolution.place source=next placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=next root=make.next

}
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

function connect(): void {
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

    send("ping");
}
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

function connect(): void {
    let count: float64 = 0;
    let socket: Socket = Socket {};

    @capture({
        default: "manage",
        socket: "move",
    })
    const send: ^((arg0: string) => void) = (message: string): void => {
        count += 1;
        socket.write(message);
    };

    send("ping");
}

=== checked ===
struct Socket {
/// @type.symbol symbol=Socket type=Socket
/// @definition.struct symbol=Socket
/// @definition.method symbol=Socket.write source="write(message: string): void {}" slot=write type=<Socket.write.'a>(this: &Socket.write.'a exclusive this, string) => void

    write(message: string): void {}
    /// @generic.template symbol=Socket.write parameters=('a)
    /// @type.symbol symbol=Socket.write source="write(message: string): void {}" type=<Socket.write.'a>(this: &Socket.write.'a exclusive this, string) => void
    /// @capture.function function=Socket.write bindings=0
    /// @type.symbol symbol=Socket.write.message source="message: string" type=string

}

function connect(): void {
/// @type.symbol symbol=connect type=() => void
/// @capture.function function=connect bindings=0

    let count = 0;
    /// @type.symbol symbol=connect.count source=count type=float64
    /// @resolution.pattern source=count kind=binding target=connect.count
    /// @type.node source=0 type=0

    let socket = Socket {};
    /// @type.symbol symbol=connect.socket source=socket type=Socket
    /// @resolution.pattern source=socket kind=binding target=connect.socket
    /// @type.node source="Socket {}" type=Socket
    /// @resolution.name source=Socket target=Socket

    @capture({
    /// @type.node source=capture type=capture
    /// @resolution.name source=capture target=decorator.capture.capture
    /// @type.node type={ default: "manage"; socket: "move" }

        default: "manage",
        /// @type.node source="\"manage\"" type="manage"

        socket: "move",
        /// @type.node source="\"move\"" type="move"

    })
    const send: ^Function<(string,), void> = (message) => {
    /// @type.symbol symbol=connect.send source=send type=Owned<Function<(string,), void>>
    /// @resolution.pattern source=send kind=binding target=connect.send
    /// @resolution.name source=Function target=types.function.Function
    /// @type.symbol symbol=connect.symbol8 type=Function<(string,), void>
    /// @type.node type=Owned<Function<(string,), void>>
    /// @capture.function function=connect.symbol8 bindings=2 frames=(main.<frame0>)
    /// @capture.binding function=connect.symbol8 symbol=count mode=manage type=float64 frame=main.<frame0>
    /// @capture.binding function=connect.symbol8 symbol=socket mode=move type=Socket
    /// @capture.directive function=connect.symbol8 default=manage rules=1
    /// @capture.rule function=connect.symbol8 binding=socket mode=move
    /// @capture.frame frame=main.<frame0> scope=scope6 type=Managed<{ count: float64 }> fields={ count: float64 }
    /// @type.symbol symbol=connect.symbol8.message source=message type=string

        count += 1;
        /// @type.node source="count += 1" type=float64
        /// @type.node source=count type=float64
        /// @resolution.name source=count target=connect.count
        /// @resolution.operator source="count += 1" type=float64 operator="+" kind=builtin operands=[count as float64 families=(float), 1 as float64 families=(float)]
        /// @resolution.pattern.assign source=count kind=place
        /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=count read=binding(connect.count) write=binding(connect.count) type=float64
        /// @resolution.access source=count root=connect.count
        /// @type.node source=1 type=1

        socket.write(message);
        /// @type.node source=socket type=Socket
        /// @type.node source=socket.write type=<Socket.write.'a>(this: &Socket.write.'a exclusive Socket, string) => void
        /// @type.node source=socket.write(message) type=void
        /// @resolution.name source=socket target=connect.socket
        /// @resolution.member source=socket.write receiver=Socket type=<Socket.write.'a>(this: &Socket.write.'a exclusive Socket, string) => void kind=symbol target_receiver=Socket target=Socket.write
        /// @resolution.call source=socket.write(message) parameters=(string) arguments=(provided(message) as string) return=void kind=symbol target=Socket.write receiver=Socket adjustments=(borrow(&'frame exclusive Socket))
        /// @resolution.place source=socket placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=socket root=connect.socket
        /// @type.node source=message type=string
        /// @resolution.name source=message target=connect.symbol8.message
        /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=message root=connect.symbol8.message

    };

    send("ping");
    /// @type.node source="send(\"ping\")" type=void
    /// @type.node source=send type=Owned<Function<(string,), void>>
    /// @resolution.name source=send target=connect.send
    /// @resolution.call source="send(\"ping\")" parameters=(string) arguments=(provided("ping") as string) return=void kind=expression target=expression
    /// @resolution.place source=send placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=send root=connect.send
    /// @type.node source="\"ping\"" type="ping"

}
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

function make(): () => Promise<string> {
    let client = new Client();

    @capture("copy")
    const load = async () => await client.read();
    return load;
}
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

function make(): () => Promise<string> {
    let client: Client = new Client();

    @capture("copy")
    const load = async () => await client.read();
    return load;
}

=== checked ===
declare class Client {
/// @type.symbol symbol=Client type=Client
/// @definition.class symbol=Client
/// @definition.method symbol=Client.read source="read(): Promise<string>" slot=read type=(this: this) => Promise<string>

    read(): Promise<string>;
    /// @type.symbol symbol=Client.read source="read(): Promise<string>" type=(this: this) => Promise<string>
    /// @resolution.name source=Promise target=async.promise.Promise

}

function make(): () => Promise<string> {
/// @type.symbol symbol=make type=() => Function<(), Promise<string>>
/// @capture.function function=make bindings=0
/// @resolution.name source=Promise target=async.promise.Promise

    let client = new Client();
    /// @type.symbol symbol=make.client source=client type=Client
    /// @resolution.pattern source=client kind=binding target=make.client
    /// @type.node source="new Client()" type=Client
    /// @resolution.construct source="new Client()" parameters=() return=Client kind=class target=Client constructor=default
    /// @resolution.name source=Client target=Client

    @capture("copy")
    /// @type.node source=capture type=capture
    /// @resolution.name source=capture target=decorator.capture.capture
    /// @type.node source="\"copy\"" type="copy"

    const load = async () => await client.read();
    /// @type.symbol symbol=make.load source=load type=Function<(), Promise<string>>
    /// @resolution.pattern source=load kind=binding target=make.load
    /// @type.symbol symbol=make.symbol6 source="async () => await client.read()" type=Function<(), Promise<string>>
    /// @type.node source="async () => await client.read()" type=Function<(), Promise<string>>
    /// @generic.instance source="async () => await client.read()" id=Promise<string>
    /// @capture.function function=make.symbol6 bindings=1
    /// @capture.binding function=make.symbol6 symbol=client mode=copy type=Client
    /// @capture.directive function=make.symbol6 default=copy rules=0
    /// @type.node source="await client.read()" type=string
    /// @type.node source=client type=Client
    /// @type.node source=client.read type=(this: Client) => Promise<string>
    /// @type.node source=client.read() type=Promise<string>
    /// @resolution.name source=client target=make.client
    /// @resolution.member source=client.read receiver=Client type=(this: Client) => Promise<string> kind=symbol target_receiver=Client target=Client.read
    /// @resolution.call source=client.read() parameters=() return=Promise<string> kind=symbol target=Client.read receiver=Client
    /// @resolution.place source=client placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=client root=make.client
    /// @generic.instance source=client.read id=Promise<string>
    /// @generic.instance source=client.read() id=Promise<string>

    return load;
    /// @type.node source=load type=Function<(), Promise<string>>
    /// @resolution.name source=load target=make.load
    /// @resolution.place source=load placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=load root=make.load
    /// @generic.instance source=load id=Promise<string>

}

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
        /// @resolution.member source=this.value receiver=Counter type=int32 kind=field target_receiver=Counter key=value target=Counter.value target_type=int32
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=Counter.make.this
        /// @resolution.place source=this.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.value root=Counter.make.this keys=[value]

    }
}
"#,
    );
}
