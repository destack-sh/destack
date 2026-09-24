use crate::tests::{DirRows, TestSession};

#[test]
fn test_accept_an_exact_value_for_a_consumed_parameter() {
    let session = TestSession::single(
        r#"
type Buffer<const N: uint> = [uint8; N];

declare const buffer: Buffer<1024>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
type Buffer<const N: uint> = [uint8; N];

declare const buffer: Buffer<1024>;

=== dir ===
type Buffer<const N: uint> = [uint8; N];

declare const buffer: Buffer<1024>;
"#,
        r#"
"#,
    );
}

#[test]
fn test_accept_a_static_computation_over_a_fixed_parameter() {
    let session = TestSession::single(
        r#"
type Buffer<const N: uint> = [uint8; N];

type Halved<const M: uint> = Buffer<M / 2>;

declare const halved: Halved<1024>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
type Buffer<const N: uint> = [uint8; N];

type Halved<const M: uint> = Buffer<M / 2>;

declare const halved: Halved<1024>;

=== dir ===
type Buffer<const N: uint> = [uint8; N];

type Halved<const M: uint> = Buffer<M / 2>;

declare const halved: Halved<1024>;
"#,
        r#"
"#,
    );
}

#[test]
fn test_reject_an_argument_that_leaves_a_consumed_parameter_open() {
    let session = TestSession::single(
        r#"
type Buffer<const N: uint> = [uint8; N];

declare const buffer: Buffer<uint>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
type Buffer<const N: uint> = [uint8; N];

declare const buffer: Buffer<uint64>;

=== dir ===
type Buffer<const N: uint> = [uint8; N];

declare const buffer: Buffer<uint>;
"#,
        r#"
/// @diagnostic.error id=argument-not-exact-value message="type 'uint64' does not fix const parameter 'N' to one exact value"
/// @diagnostic.label line=4 column=30 span="uint" line_source="declare const buffer: Buffer<uint>;"
"#,
    );
}

#[test]
fn test_chain_a_consumed_parameter_through_an_alias() {
    let session = TestSession::single(
        r#"
type Double<const N: uint> = [uint8; N];

type Quad<const M: uint> = Double<M>;

declare const quad: Quad<uint>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
type Double<const N: uint> = [uint8; N];

type Quad<const M: uint> = Double<M>;

declare const quad: Quad<uint64>;

=== dir ===
type Double<const N: uint> = [uint8; N];

type Quad<const M: uint> = Double<M>;

declare const quad: Quad<uint>;
"#,
        r#"
/// @diagnostic.error id=argument-not-exact-value message="type 'uint64' does not fix const parameter 'M' to one exact value"
/// @diagnostic.label line=6 column=26 span="uint" line_source="declare const quad: Quad<uint>;"
"#,
    );
}

/// A body read of a const parameter fixes it, every instantiation then supplying one exact value.
#[test]
fn test_fix_a_const_parameter_read_in_the_body_at_instantiation() {
    let session = TestSession::single(
        r#"
function count<const N: usize>(): usize {
    let total: usize = 0;

    for (let lane: usize = 0; lane < N; lane += 1) {
        total += 1;
    }

    total
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
function count<const N: usize>(): usize {
    let total: usize = 0;

    for (let lane: usize = 0; lane < N; lane += 1) {
        total += 1;
    }

    total
}

=== dir ===
function count<const N: usize>(): usize {
    let total: usize = 0;

    for (let lane: usize = 0; lane < N; lane += 1) {
        total += 1;
    }

    total
}
"#,
        r#"
"#,
    );
}

#[test]
fn test_read_a_signature_fixed_parameter_in_the_body() {
    let session = TestSession::single(
        r#"
struct Block<const N: usize> {
    data: [uint8; N];

    get length(): usize {
        N
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
struct Block<const N: usize> {
    data: [uint8; N];

    get length(): usize {
        N
    }
}

=== dir ===
struct Block<const N: usize> {
    data: [uint8; N];

    get length(): usize {
        N
    }
}
"#,
        r#"
"#,
    );
}

/// A value parameter typed by a const parameter demands one exact argument.
#[test]
fn test_reject_a_widened_argument_at_a_const_typed_parameter() {
    let session = TestSession::single(
        r#"
enum Ordering { Relaxed, Acquire }

declare function primitive(order: Ordering): void;

function load<const Order: Ordering = Ordering.Acquire>(order?: Order): void {
    primitive(Order);
}

declare const runtime: Ordering;

load();
load(Ordering.Relaxed);
load(runtime);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
enum Ordering {
    Relaxed,
    Acquire,
}

declare function primitive(order: Ordering): void;

function load<const Order: Ordering = Ordering.Acquire>(order?: Order): void {
    primitive(Order);
}

declare const runtime: Ordering;

load();
load(Ordering.Relaxed);
load<Ordering>(runtime as Ordering | undefined);

=== dir ===
enum Ordering { Relaxed, Acquire }

declare function primitive(order: Ordering): void;

function load<const Order: Ordering = Ordering.Acquire>(order?: Order): void {
    primitive(Order);
}

declare const runtime: Ordering;

load();
load(Ordering.Relaxed);
load(runtime);
"#,
        r#"
/// @diagnostic.error id=argument-not-exact-value message="type 'Ordering' does not fix const parameter 'Order' to one exact value"
/// @diagnostic.label line=14 column=1 span="load(runtime)" line_source="load(runtime);"
"#,
    );
}
