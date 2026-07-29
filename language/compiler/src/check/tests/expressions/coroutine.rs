use crate::tests::{DirRows, TestSession};

#[test]
fn test_type_async_function_results_as_promises() {
    let session = TestSession::single(
        r#"
async function fetchCount(): Promise<int32> {
    return 1;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
async function fetchCount(): Promise<int32> {
    return 1;
}

=== checked ===
async function fetchCount(): Promise<int32> {
/// @type.symbol symbol=fetchCount type=async () => Promise<int32>
/// @resolution.name source=Promise target=async.promise.Promise

    return 1;
}

/// @generic.instance id=Promise<int32> template=async.promise.Promise arguments=(int32)
"#,
    );
}

#[test]
fn test_unwrap_awaited_promise_values() {
    let session = TestSession::single(
        r#"
async function fetchCount(): Promise<int32> {
    return 1;
}

async function double(): Promise<int32> {
    const count = await fetchCount();

    return count + count;
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
async function fetchCount(): Promise<int32> {
    return 1;
}

async function double(): Promise<int32> {
    const count: int32 = await fetchCount();

    return count + count;
}

=== checked ===
async function fetchCount(): Promise<int32> {
/// @type.symbol symbol=fetchCount type=async () => Promise<int32>
/// @resolution.name source=Promise target=async.promise.Promise

    return 1;
}

async function double(): Promise<int32> {
/// @type.symbol symbol=double type=async () => Promise<int32>
/// @resolution.name source=Promise target=async.promise.Promise

    const count = await fetchCount();
    /// @type.symbol symbol=double.count source=count type=int32
    /// @resolution.pattern source=count kind=binding target=double.count
    /// @resolution.name source=fetchCount target=fetchCount
    /// @resolution.call source=fetchCount() parameters=() return=Promise<int32> kind=symbol target=fetchCount

    return count + count;
    /// @resolution.name source=count target=double.count
    /// @resolution.operator source="count + count" type=int32 operator="+" kind=builtin operands=[count as int32 families=(integer), count as int32 families=(integer)]
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=double.count
    /// @resolution.name source=count target=double.count
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=double.count

}

/// @generic.instance id=Promise<int32> template=async.promise.Promise arguments=(int32)
"#);
}

#[test]
fn test_type_generator_functions_and_yields() {
    let session = TestSession::single(
        r#"
function* count(limit: int32): Generator<int32, void, void> {
    for (let value: int32 = 0; value < limit; value += 1) {
        yield value;
    }
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
function* count(limit: int32): Generator<int32, void, void> {
    for (let value: int32 = 0; value < limit; value += 1) {
        yield value;
    }
}

=== checked ===
function* count(limit: int32): Generator<int32, void, void> {
/// @type.symbol symbol=count type=(int32) => *Generator<int32, void, void>
/// @type.symbol symbol=count.limit source="limit: int32" type=int32
/// @resolution.name source=Generator target=async.generator.Generator

    for (let value: int32 = 0; value < limit; value += 1) {
    /// @type.symbol symbol=count.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=count.value
    /// @resolution.name source=value target=count.value
    /// @resolution.operator source="value < limit" type=boolean operator="<" kind=builtin operands=[value as int32 families=(integer), limit as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=count.value
    /// @resolution.name source=limit target=count.limit
    /// @resolution.place source=limit placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=limit root=count.limit
    /// @resolution.operator source="value += 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.pattern.assign source=value kind=place
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=value read=binding(count.value) write=binding(count.value) type=int32
    /// @resolution.access source=value root=count.value

        yield value;
        /// @resolution.name source=value target=count.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=count.value

    }
}

/// @generic.instance id="Generator<int32, void, void>" template=async.generator.Generator arguments=(int32, void, void)
"#);
}

#[test]
fn test_iterate_async_sequences_with_for_await() {
    let session = TestSession::single(
        r#"
async function* stream(): AsyncGenerator<int32, void, void> {
    yield 1;
}

async function sum(): Promise<int32> {
    let total: int32 = 0;
    for await (const value of stream()) {
        total += value;
    }

    return total;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
async function* stream(): AsyncGenerator<int32, void, void> {
    yield 1;
}

async function sum(): Promise<int32> {
    let total: int32 = 0;
    for await (const value of stream()) {
        total += value;
    }

    return total;
}

=== checked ===
async function* stream(): AsyncGenerator<int32, void, void> {
/// @type.symbol symbol=stream type=async () => *AsyncGenerator<int32, void, void>
/// @resolution.name source=AsyncGenerator target=async.generator.AsyncGenerator

    yield 1;
}

async function sum(): Promise<int32> {
/// @type.symbol symbol=sum type=async () => Promise<int32>
/// @resolution.name source=Promise target=async.promise.Promise

    let total: int32 = 0;
    /// @type.symbol symbol=sum.total source=total type=int32
    /// @resolution.pattern source=total kind=binding target=sum.total

    for await (const value of stream()) {
    /// @type.symbol symbol=sum.value source=value type=<error>
    /// @resolution.pattern source=value kind=binding target=sum.value
    /// @resolution.name source=stream target=stream
    /// @resolution.call source=stream() parameters=() return=AsyncGenerator<int32, void, void> kind=symbol target=stream

        total += value;
        /// @resolution.pattern.assign source=total kind=place
        /// @resolution.place source=total placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=total read=binding(sum.total) write=binding(sum.total) type=int32
        /// @resolution.access source=total root=sum.total
        /// @resolution.name source=value target=sum.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=sum.value

    }

    return total;
    /// @resolution.name source=total target=sum.total
    /// @resolution.place source=total placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=total root=sum.total

}

/// @generic.instance id="AsyncGenerator<int32, void, void>" template=async.generator.AsyncGenerator arguments=(int32, void, void)
/// @generic.instance id=Promise<int32> template=async.promise.Promise arguments=(int32)
"#, r#"
/// @diagnostic.error id=for-of-source-not-iterable message="for-of source must be iterable"
/// @diagnostic.label line=8 column=5 span="for await (const value of stream()) {\n        total += value;\n    }" line_source="for await (const value of stream()) {"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=8 column=5 span="for await (const value of stream()) {\n        total += value;\n    }" line_source="for await (const value of stream()) {"
/// @diagnostic.help message="annotate the type explicitly"
"#);
}
