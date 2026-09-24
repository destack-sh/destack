use crate::tests::{DirRows, TestSession};

/// Instance arguments follow the parameter declaration order.
#[test]
fn test_record_instance_arguments_in_declaration_order() {
    let session = TestSession::single(
        r#"
function pick<A, B>(second: B, first: A): A {
    return first;
}

function use(): string {
    return pick(1, "x");
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
function pick<A, B>(second: B, first: A): A {
    return first;
}

function use(): string {
    return pick<string, int64>(1, "x");
}

=== dir ===
function pick<A, B>(second: B, first: A): A {
/// @generic.template symbol=pick parameters=(A, B)
/// @type.symbol symbol=pick type=<A, B>(B, A) => A
/// @type.symbol symbol=pick.A source=A type=A
/// @type.symbol symbol=pick.B source=B type=B
/// @type.symbol symbol=pick.second source="second: B" type=B
/// @resolution.name source=B target=pick.B
/// @type.symbol symbol=pick.first source="first: A" type=A
/// @resolution.name source=A target=pick.A
/// @resolution.name source=A target=pick.A

    return first;
    /// @resolution.name source=first target=pick.first
    /// @resolution.place source=first placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=first root=pick.first

}

function use(): string {
/// @type.symbol symbol=use type=() => string

    return pick(1, "x");
    /// @resolution.name source=pick target=pick
    /// @resolution.call source="pick(1, \"x\")" parameters=(int64, string) arguments=(provided(1) as int64, provided("x") as string) return=string kind=symbol target=pick instance="pick<string, int64>"
    /// @generic.instantiation id="pick<string, int64>" template=pick arguments=(string, int64)
    /// @generic.instance id="pick<string, int64>" template=pick arguments=(string, int64)

}
"#);
}

/// Record outer extension arguments before nested method arguments.
#[test]
fn test_record_outer_extension_arguments_before_method_arguments() {
    let session = TestSession::single(
        r#"
struct Box<T> {
    value: T;
}

extension<T> of Box<T> {
    swap<U>(other: U): U {
        return other;
    }
}

function use(box: Box<int32>): string {
    return box.swap("x");
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Box<out T> {
    value: T;
}

extension<T> of Box<T> {
    swap<U>(other: U): U {
        return other;
    }
}

function use(box: Box<int32>): string {
    return box.swap<int32, "frame", string>("x");
}

=== dir ===
struct Box<T> {
/// @generic.template symbol=Box parameters=(out T#1)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

extension<T> of Box<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.method symbol=swap slot=swap type=<swap.'a, U>(this: &swap.'a readonly Box<T#2>, U) => U
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T

    swap<U>(other: U): U {
    /// @generic.template symbol=swap parent=template#1 parameters=('a, U)
    /// @type.symbol symbol=swap type=<swap.'a, U>(this: &swap.'a readonly Box<T#2>, U) => U
    /// @type.symbol symbol=swap.this type=&swap.'a readonly Box<T#2>
    /// @type.symbol symbol=swap.U source=U type=U
    /// @type.symbol symbol=swap.other source="other: U" type=U
    /// @resolution.name source=U target=swap.U
    /// @resolution.name source=U target=swap.U

        return other;
        /// @resolution.name source=other target=swap.other
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=swap.other

    }
}

function use(box: Box<int32>): string {
/// @type.symbol symbol=use type=(Box<int32>) => string
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @type.symbol symbol=use.box source="box: Box<int32>" type=Box<int32>
/// @resolution.name source=Box target=Box

    return box.swap("x");
    /// @resolution.name source=box target=use.box
    /// @resolution.member source=box.swap receiver=Box<int32> type=<swap.'a, U>(this: &swap.'a readonly Box<int32>, U) => U kind=symbol target_receiver=Box<int32> target=swap
    /// @resolution.call source="box.swap(\"x\")" parameters=(string) arguments=(provided("x") as string) return=string regions=("frame" & "local") kind=symbol target=swap receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#1>.swap<\"frame\" & \"local\", string>"
    /// @resolution.place source=box placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=box root=use.box
    /// @generic.instantiation id="swap<int32, \"frame\" & \"local\", string>" template=swap arguments=(int32, "frame" & "local", string)
    /// @generic.instantiation id=swap<int32> template=swap arguments=(int32)
    /// @generic.instance id="swap<int32, \"bound0\" & \"local\", string>" template=swap arguments=(int32, "bound0" & "local", string)

}
"#);
}
