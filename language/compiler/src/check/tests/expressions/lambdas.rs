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
/// @type.symbol symbol=next type=() => int32
/// @resolution.name source=count target=count
/// @type.node source="count + 1" type=int32
/// @capture.function function=next bindings=1
/// @capture.binding function=next symbol=count mode=borrow
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
    default: "borrow",
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
    default: "borrow",
    step: "copy",
})
const next = () => count + step;
/// @type.symbol symbol=next type=() => int32
/// @resolution.name source=count target=count
/// @resolution.name source=step target=step
/// @type.node source="count + step" type=int32
/// @capture.function function=next bindings=2
/// @capture.binding function=next symbol=count mode=borrow
/// @capture.binding function=next symbol=step mode=copy
/// @capture.directive function=next default=borrow rules=1
/// @capture.rule function=next binding=step mode=copy
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
/// @type.symbol symbol=load type=() => Promise<string>
/// @resolution.name source=client target=client
/// @resolution.member source=client.read receiver=Client kind=direct target=Client.read
/// @resolution.call source="client.read()" parameters=[] return=Promise<string> kind=direct target=Client.read receiver=Client
/// @type.node source="await client.read()" type=string
/// @capture.function function=load bindings=1
/// @capture.binding function=load symbol=client mode=copy
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
    /// @type.symbol symbol=Counter.make type=(this: Counter) => () => int32

        return () => this.value;
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Counter kind=direct target=Counter.value
        /// @type.node source=this.value type=int32
        /// @capture.function function=Counter.make.<lambda0> this=this:borrow
    }
}
"#,
    );
}
