use crate::tests::{DirRows, TestSession};

/// A using binding records the Dispose call its scope exit runs.
#[test]
fn test_record_the_dispose_call_of_a_using_binding() {
    let session = TestSession::single(
        r#"
import { Dispose } from "tspp:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

function run(): void {
    using file = File { handle: 1 };
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Dispose } from "tspp:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

function run(): void {
    using file: File = File { handle: 1 };
}

=== dir ===
import { Dispose } from "tspp:memory";

struct File implements Dispose {
/// @type.symbol symbol=File type=File
/// @definition.struct symbol=File
/// @definition.where symbol=File source=Dispose relation=satisfies left=this right=Dispose
/// @definition.implements symbol=File source=Dispose target=Dispose
/// @definition.field symbol=File.handle source="handle: int32" key=handle type=int32
/// @definition.method symbol=File.dispose source="dispose(&this): void {}" slot=dispose type=<File.dispose.'a>(this: &File.dispose.'a File) => void
/// @definition.conformance symbol=File member=File.dispose requirement=Dispose.dispose
/// @resolution.name source=Dispose target=Dispose

    handle: int32;
    /// @type.symbol symbol=File.handle source="handle: int32" type=int32

    dispose(&this): void {}
    /// @generic.template symbol=File.dispose parent=template#0 parameters=('a)
    /// @type.symbol symbol=File.dispose source="dispose(&this): void {}" type=<File.dispose.'a>(this: &File.dispose.'a File) => void
    /// @type.symbol symbol=File.dispose.this source=&this type=&File.dispose.'a File

}

function run(): void {
/// @type.symbol symbol=run type=() => void

    using file = File { handle: 1 };
    /// @type.symbol symbol=run.file source=file type=File
    /// @resolution.disposal source="file = File { handle: 1 }" dispose="File.dispose(parameters=(), arguments=(), return=void, regions=(\"frame\" & \"local\"))"
    /// @resolution.pattern source=file kind=binding target=run.file
    /// @generic.instantiation id="File.dispose<\"frame\" & \"local\">" template=File.dispose arguments=("frame" & "local")
    /// @generic.instance id="File.dispose<\"bound0\" & \"local\">" template=File.dispose arguments=("bound0" & "local")
    /// @resolution.name source=File target=File

}
"#);
}

/// A nullable using binding records the Dispose call of its present arm.
#[test]
fn test_record_the_dispose_call_of_a_nullable_using_binding() {
    let session = TestSession::single(
        r#"
import { Dispose } from "tspp:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

function run(file: File | undefined): void {
    using resource = file;
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Dispose } from "tspp:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

function run(file: File | undefined): void {
    using resource: File | undefined = file;
}

=== dir ===
import { Dispose } from "tspp:memory";

struct File implements Dispose {
/// @type.symbol symbol=File type=File
/// @definition.struct symbol=File
/// @definition.where symbol=File source=Dispose relation=satisfies left=this right=Dispose
/// @definition.implements symbol=File source=Dispose target=Dispose
/// @definition.field symbol=File.handle source="handle: int32" key=handle type=int32
/// @definition.method symbol=File.dispose source="dispose(&this): void {}" slot=dispose type=<File.dispose.'a>(this: &File.dispose.'a File) => void
/// @definition.conformance symbol=File member=File.dispose requirement=Dispose.dispose
/// @resolution.name source=Dispose target=Dispose

    handle: int32;
    /// @type.symbol symbol=File.handle source="handle: int32" type=int32

    dispose(&this): void {}
    /// @generic.template symbol=File.dispose parent=template#0 parameters=('a)
    /// @type.symbol symbol=File.dispose source="dispose(&this): void {}" type=<File.dispose.'a>(this: &File.dispose.'a File) => void
    /// @type.symbol symbol=File.dispose.this source=&this type=&File.dispose.'a File

}

function run(file: File | undefined): void {
/// @type.symbol symbol=run type=(File | undefined) => void
/// @type.symbol symbol=run.file source="file: File | undefined" type=File | undefined
/// @resolution.name source=File target=File

    using resource = file;
    /// @type.symbol symbol=run.resource source=resource type=File | undefined
    /// @resolution.disposal source="resource = file" dispose="File.dispose(parameters=(), arguments=(), return=void, regions=(\"frame\" & \"local\"))"
    /// @resolution.pattern source=resource kind=binding target=run.resource
    /// @generic.instantiation id="File.dispose<\"frame\" & \"local\">" template=File.dispose arguments=("frame" & "local")
    /// @generic.instance id="File.dispose<\"bound0\" & \"local\">" template=File.dispose arguments=("bound0" & "local")
    /// @resolution.name source=file target=run.file
    /// @resolution.access source=file root=run.file

}
"#);
}

/// An await using binding records the AsyncDispose call and the park awaiting its completion.
#[test]
fn test_record_the_async_dispose_call_of_an_await_using_binding() {
    let session = TestSession::single(
        r#"
import { Promise } from "tspp:async";
import { AsyncDispose } from "tspp:memory";

class Connection implements AsyncDispose {
    async asyncDispose(): Promise<void> {}
}

function connect(): Connection {
    return new Connection();
}

async function run(): Promise<void> {
    await using connection = connect();
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Promise } from "tspp:async";
import { AsyncDispose } from "tspp:memory";

class Connection implements AsyncDispose {
    async asyncDispose(): Promise<void> {}
}

function connect(): Connection {
    return new Connection();
}

async function run(): Promise<void> {
    await using connection: Connection = connect();
}

=== dir ===
import { Promise } from "tspp:async";
import { AsyncDispose } from "tspp:memory";

class Connection implements AsyncDispose {
/// @type.symbol symbol=Connection type=typeof Connection
/// @definition.class symbol=Connection
/// @definition.where symbol=Connection source=AsyncDispose relation=satisfies left=this right=AsyncDispose
/// @definition.implements symbol=Connection source=AsyncDispose target=AsyncDispose
/// @definition.method symbol=Connection.asyncDispose source="async asyncDispose(): Promise<void> {}" slot=asyncDispose type=async (this: Connection) => Promise<void>
/// @definition.conformance symbol=Connection member=Connection.asyncDispose requirement=AsyncDispose.asyncDispose
/// @resolution.name source=AsyncDispose target=AsyncDispose

    async asyncDispose(): Promise<void> {}
    /// @type.symbol symbol=Connection.asyncDispose source="async asyncDispose(): Promise<void> {}" type=async (this: Connection) => Promise<void>
    /// @type.symbol symbol=Connection.asyncDispose.this type=Connection
    /// @resolution.call source="async asyncDispose(): Promise<void> {}" parameters=(^Function<(), void, "once">) arguments=(supplied(0) as ^Function<(), void, "once">) return=Promise<void> kind=symbol target=Promise.create instance=Promise.create<void>
    /// @generic.instantiation id=Promise.create<void> template=Promise.create arguments=(void)
    /// @generic.instance id=Promise.create<void> template=Promise.create arguments=(void)
    /// @generic.instance id=Promise.fulfill<void> template=Promise.fulfill arguments=(void)
    /// @generic.instance id=Promise.pending<void> template=Promise.pending arguments=(void)
    /// @generic.instance id=Promise.queueWaiters<void> template=Promise.queueWaiters arguments=(void)
    /// @generic.instance id=Promise.symbol12<void> template=Promise.symbol12 arguments=(void)
    /// @generic.instance id=Promise<void> template=Promise arguments=(void)
    /// @resolution.name source=Promise target=Promise

}

function connect(): Connection {
/// @type.symbol symbol=connect type=() => Connection
/// @resolution.name source=Connection target=Connection

    return new Connection();
    /// @resolution.construct source="new Connection()" parameters=() return=Connection kind=class target=Connection constructor=default
    /// @resolution.name source=Connection target=Connection

}

async function run(): Promise<void> {
/// @type.symbol symbol=run type=async () => Promise<void>
/// @resolution.call parameters=(^Function<(), void, "once">) arguments=(supplied(0) as ^Function<(), void, "once">) return=Promise<void> kind=symbol target=Promise.create instance=Promise.create<void>
/// @resolution.name source=Promise target=Promise

    await using connection = connect();
    /// @type.symbol symbol=run.connection source=connection type=Connection
    /// @resolution.disposal source="connection = connect()" dispose="Connection.asyncDispose(parameters=(), arguments=(), return=Promise<void>)" await="Promise.park(parameters=(Promise<void>), arguments=(supplied(0) as Promise<void>), return=void)"
    /// @resolution.pattern source=connection kind=binding target=run.connection
    /// @generic.instantiation id="Promise.park<void, void>" template=Promise.park arguments=(void, void)
    /// @generic.instance id="Promise.park<void, void>" template=Promise.park arguments=(void, void)
    /// @generic.instance id=Promise.addWaiter<void> template=Promise.addWaiter arguments=(void)
    /// @generic.instance id=Promise.observe<void> template=Promise.observe arguments=(void)
    /// @generic.instance id=Promise.queueWaiter<void> template=Promise.queueWaiter arguments=(void)
    /// @generic.instance id=PromiseAwaiter.symbol161<void> template=PromiseAwaiter.symbol161 arguments=(void)
    /// @generic.instance id=PromiseAwaiter<void> template=PromiseAwaiter arguments=(void)
    /// @generic.instance id=PromiseForwarded<void> template=PromiseForwarded arguments=(void)
    /// @generic.instance id=PromiseFulfilled<void> template=PromiseFulfilled arguments=(void)
    /// @resolution.name source=connect target=connect
    /// @resolution.call source=connect() parameters=() return=Connection kind=symbol target=connect

}
"#);
}

/// An await using binding falls back to the Dispose call of a synchronously disposable resource.
#[test]
fn test_fall_back_to_the_dispose_call_for_an_await_using_binding() {
    let session = TestSession::single(
        r#"
import { Dispose } from "tspp:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

async function run(): Promise<void> {
    await using file = File { handle: 1 };
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Dispose } from "tspp:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

async function run(): Promise<void> {
    await using file: File = File { handle: 1 };
}

=== dir ===
import { Dispose } from "tspp:memory";

struct File implements Dispose {
/// @type.symbol symbol=File type=File
/// @definition.struct symbol=File
/// @definition.where symbol=File source=Dispose relation=satisfies left=this right=Dispose
/// @definition.implements symbol=File source=Dispose target=Dispose
/// @definition.field symbol=File.handle source="handle: int32" key=handle type=int32
/// @definition.method symbol=File.dispose source="dispose(&this): void {}" slot=dispose type=<File.dispose.'a>(this: &File.dispose.'a File) => void
/// @definition.conformance symbol=File member=File.dispose requirement=Dispose.dispose
/// @resolution.name source=Dispose target=Dispose

    handle: int32;
    /// @type.symbol symbol=File.handle source="handle: int32" type=int32

    dispose(&this): void {}
    /// @generic.template symbol=File.dispose parent=template#0 parameters=('a)
    /// @type.symbol symbol=File.dispose source="dispose(&this): void {}" type=<File.dispose.'a>(this: &File.dispose.'a File) => void
    /// @type.symbol symbol=File.dispose.this source=&this type=&File.dispose.'a File

}

async function run(): Promise<void> {
/// @type.symbol symbol=run type=async () => Promise<void>
/// @resolution.call parameters=(^Function<(), void, "once">) arguments=(supplied(0) as ^Function<(), void, "once">) return=Promise<void> kind=symbol target=Promise.create instance=Promise.create<void>
/// @generic.instantiation id=Promise.create<void> template=Promise.create arguments=(void)
/// @resolution.name source=Promise target=Promise

    await using file = File { handle: 1 };
    /// @type.symbol symbol=run.file source=file type=File
    /// @resolution.disposal source="file = File { handle: 1 }" dispose="File.dispose(parameters=(), arguments=(), return=void, regions=(\"frame\" & \"local\"))"
    /// @resolution.pattern source=file kind=binding target=run.file
    /// @generic.instantiation id="File.dispose<\"frame\" & \"local\">" template=File.dispose arguments=("frame" & "local")
    /// @resolution.name source=File target=File

}
"#, r#"
"#);
}

/// A using binding of a resource without a disposal protocol is rejected.
#[test]
fn test_reject_a_using_binding_without_a_disposal_protocol() {
    let session = TestSession::single(
        r#"
function run(): void {
    using value = 1;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
function run(): void {
    using value: int64 = 1;
}

=== dir ===
function run(): void {
/// @type.symbol symbol=run type=() => void

    using value = 1;
    /// @type.symbol symbol=run.value source=value type=int64
    /// @resolution.pattern source=value kind=binding target=run.value

}
"#, r#"
/// @diagnostic.error id=using-resource-not-disposable message="'using' resource does not implement Dispose"
/// @diagnostic.label line=3 column=11 span="value" line_source="using value = 1;"
"#);
}

/// A for-of using binding records the Dispose call each pass runs on its resource.
#[test]
fn test_record_the_dispose_call_of_a_for_of_using_binding() {
    let session = TestSession::single(
        r#"
import { Dispose } from "tspp:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

function total(files: File[]): int32 {
    let sum: int32 = 0;
    for (using file of files) {
        sum += file.handle;
    }

    return sum;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Dispose } from "tspp:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

function total(files: File[]): int32 {
    let sum: int32 = 0;
    for (using file of files) {
        sum += file.handle;
    }

    return sum;
}

=== dir ===
import { Dispose } from "tspp:memory";

struct File implements Dispose {
/// @type.symbol symbol=File type=File
/// @definition.struct symbol=File
/// @definition.where symbol=File source=Dispose relation=satisfies left=this right=Dispose
/// @definition.implements symbol=File source=Dispose target=Dispose
/// @definition.field symbol=File.handle source="handle: int32" key=handle type=int32
/// @definition.method symbol=File.dispose source="dispose(&this): void {}" slot=dispose type=<File.dispose.'a>(this: &File.dispose.'a File) => void
/// @definition.conformance symbol=File member=File.dispose requirement=Dispose.dispose
/// @resolution.name source=Dispose target=Dispose

    handle: int32;
    /// @type.symbol symbol=File.handle source="handle: int32" type=int32

    dispose(&this): void {}
    /// @generic.template symbol=File.dispose parent=template#0 parameters=('a)
    /// @type.symbol symbol=File.dispose source="dispose(&this): void {}" type=<File.dispose.'a>(this: &File.dispose.'a File) => void
    /// @type.symbol symbol=File.dispose.this source=&this type=&File.dispose.'a File

}

function total(files: File[]): int32 {
/// @type.symbol symbol=total type=(File[]) => int32
/// @type.symbol symbol=total.files source="files: File[]" type=File[]
/// @resolution.name source=File target=File

    let sum: int32 = 0;
    /// @type.symbol symbol=total.sum source=sum type=int32
    /// @resolution.pattern source=sum kind=binding target=total.sum

    for (using file of files) {
    /// @resolution.iteration iterator="iterator#2(parameters=(), arguments=(), return=Iterator<File>)" next="dynamic(Iterator<File> as Iterator<File>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<File, void>, regions=(\"managed\" & \"local\"))" dispose="File.dispose(parameters=(), arguments=(), return=void, regions=(\"frame\" & \"local\"))"
    /// @generic.instantiation id="File.dispose<\"frame\" & \"local\">" template=File.dispose arguments=("frame" & "local")
    /// @generic.instantiation id=iterator#2<File> template=iterator#2 arguments=(File)
    /// @type.symbol symbol=total.file source=file type=File
    /// @resolution.pattern source=file kind=binding target=total.file
    /// @resolution.name source=files target=total.files
    /// @resolution.place source=files placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=files root=total.files

        sum += file.handle;
        /// @resolution.name source=sum target=total.sum
        /// @resolution.operator source="sum += file.handle" type=int32 operator="+" kind=builtin operands=[sum as int32 families=(integer), file.handle as int32 families=(integer)]
        /// @resolution.pattern.assign source=sum kind=place
        /// @resolution.place source=sum placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=sum read=binding(total.sum) write=binding(total.sum) type=int32
        /// @resolution.access source=sum root=total.sum
        /// @resolution.name source=file target=total.file
        /// @resolution.member source=file.handle receiver=File type=int32 kind=field target_receiver=File key=handle target=File.handle target_type=int32
        /// @resolution.place source=file placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=file root=total.file
        /// @resolution.place source=file.handle placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=file.handle root=total.file keys=[handle]

    }

    return sum;
    /// @resolution.name source=sum target=total.sum
    /// @resolution.place source=sum placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=sum root=total.sum

}
"#, r#"
"#);
}
