use crate::tests::{DirRows, TestSession};

/// A union binding accepts a value of each member type.
#[test]
fn test_union_accepts_each_member() {
    let session = TestSession::single(
        r#"
let value: string | int32 = "hello";
value = 42;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
let value: string | int32 = "hello" as string | int32;
value = 42 as string | int32;

=== dir ===
let value: string | int32 = "hello";
/// @type.symbol symbol=value source=value type=string | int32
/// @resolution.pattern source=value kind=binding target=value

value = 42;
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=string | int32
"#,
    );
}

/// A union binding accepts a value declared at one member type.
#[test]
fn test_union_accepts_declared_member() {
    let session = TestSession::single(
        r#"
const text: string = "hello";
const value: string | int32 = text;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const text: string = "hello";
const value: string | int32 = text as string | int32;

=== dir ===
const text: string = "hello";
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text

const value: string | int32 = text;
/// @type.symbol symbol=value source=value type=string | int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=text target=text
/// @resolution.place source=text placement="local" lifetime="static" access="immutable"
/// @resolution.access source=text root=text
"#,
    );
}

/// A union flattens the members of a nested union alias.
#[test]
fn test_union_flattens_nested_aliases() {
    let session = TestSession::single(
        r#"
type A = { a: int32 } | { b: string };
type B = A | { c: boolean };

const value: B = { c: true };
value satisfies { a: int32 } | { b: string } | { c: boolean };
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type A = { a: int32 } | { b: string };
type B = A | { c: boolean };

const value: B = { c: true } as B;
value satisfies { a: int32 } | { b: string } | { c: boolean };

=== dir ===
type A = { a: int32 } | { b: string };
/// @type.symbol symbol=A source="type A = { a: int32 } | { b: string }" type={ a: int32 } | { b: string }
/// @definition.type symbol=A source="type A = { a: int32 } | { b: string }" value={ a: int32 } | { b: string }
/// @type.symbol symbol=A.a source="a: int32" type=int32
/// @type.symbol symbol=A.b source="b: string" type=string

type B = A | { c: boolean };
/// @type.symbol symbol=B source="type B = A | { c: boolean }" type={ a: int32 } | { b: string } | { c: boolean }
/// @definition.type symbol=B source="type B = A | { c: boolean }" value=A | { c: boolean }
/// @resolution.name source=A target=A
/// @type.symbol symbol=B.c source="c: boolean" type=boolean

const value: B = { c: true };
/// @type.symbol symbol=value source=value type=B
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=B target=B

value satisfies { a: int32 } | { b: string } | { c: boolean };
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @type.symbol symbol=a source="a: int32" type=int32
/// @type.symbol symbol=b source="b: string" type=string
/// @type.symbol symbol=c source="c: boolean" type=boolean
"#,
    );
}

/// A union drops its never members.
#[test]
fn test_union_removes_never() {
    let session = TestSession::single(
        r#"
type A = never | string;

const value: A = "hello";
value satisfies string;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type A = never | string;

const value: A = "hello";
value satisfies string;

=== dir ===
type A = never | string;
/// @type.symbol symbol=A source="type A = never | string" type=string
/// @definition.type symbol=A source="type A = never | string" value=string

const value: A = "hello";
/// @type.symbol symbol=value source=value type=A
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=A target=A

value satisfies string;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

/// A union of void and never keeps void.
#[test]
fn test_void_union_with_never_keeps_void() {
    let session = TestSession::single(
        r#"
type Value = void | never;

const value: Value = ();
value satisfies void;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = void | never;

const value: Value = ();
value satisfies void;

=== dir ===
type Value = void | never;
/// @type.symbol symbol=Value source="type Value = void | never" type=void
/// @definition.type symbol=Value source="type Value = void | never" value=void

const value: Value = ();
/// @type.symbol symbol=value source=value type=Value
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

value satisfies void;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

/// A member call on a union selects the method of each variant.
#[test]
fn test_union_member_call_selects_each_variant_method() {
    let session = TestSession::single(
        r#"
struct Rectangle {
    draw(): void {}
}

struct Circle {
    draw(): void {}
}

let shape: Rectangle | Circle = Rectangle {};
shape.draw();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Rectangle {
    draw(): void {}
}

struct Circle {
    draw(): void {}
}

let shape: Rectangle | Circle = Rectangle {} as Rectangle | Circle;
shape.draw();

=== dir ===
struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.method symbol=Rectangle.draw source="draw(): void {}" slot=draw type=<Rectangle.draw.'a>(this: &Rectangle.draw.'a readonly Rectangle) => void

    draw(): void {}
    /// @generic.template symbol=Rectangle.draw parameters=('a)
    /// @type.symbol symbol=Rectangle.draw source="draw(): void {}" type=<Rectangle.draw.'a>(this: &Rectangle.draw.'a readonly Rectangle) => void
    /// @type.symbol symbol=Rectangle.draw.this type=&Rectangle.draw.'a readonly Rectangle

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.method symbol=Circle.draw source="draw(): void {}" slot=draw type=<Circle.draw.'a>(this: &Circle.draw.'a readonly Circle) => void

    draw(): void {}
    /// @generic.template symbol=Circle.draw parameters=('a)
    /// @type.symbol symbol=Circle.draw source="draw(): void {}" type=<Circle.draw.'a>(this: &Circle.draw.'a readonly Circle) => void
    /// @type.symbol symbol=Circle.draw.this type=&Circle.draw.'a readonly Circle

}

let shape: Rectangle | Circle = Rectangle {};
/// @type.symbol symbol=shape source=shape type=Rectangle | Circle
/// @resolution.pattern source=shape kind=binding target=shape
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle
/// @resolution.name source=Rectangle target=Rectangle

shape.draw();
/// @resolution.name source=shape target=shape
/// @resolution.member source=shape.draw type=<Rectangle.draw.'a>(this: &Rectangle.draw.'a readonly Rectangle) => void | <Circle.draw.'a>(this: &Circle.draw.'a readonly Circle) => void kind=union arms=[receiver=Rectangle, target=Rectangle.draw, type=<Rectangle.draw.'a>(this: &Rectangle.draw.'a readonly Rectangle) => void, receiver=Circle, target=Circle.draw, type=<Circle.draw.'a>(this: &Circle.draw.'a readonly Circle) => void]
/// @resolution.call source=shape.draw() return=void kind=union arms=[Rectangle.draw(parameters=(), arguments=(), return=void, regions=("static" & "local")), Circle.draw(parameters=(), arguments=(), return=void, regions=("static" & "local"))]
/// @resolution.place source=shape placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=shape root=shape
/// @generic.instantiation id="Circle.draw<\"static\" & \"local\">" template=Circle.draw arguments=("static" & "local")
/// @generic.instantiation id="Rectangle.draw<\"static\" & \"local\">" template=Rectangle.draw arguments=("static" & "local")
/// @generic.instance id="Circle.draw<\"bound0\" & \"local\">" template=Circle.draw arguments=("bound0" & "local")
/// @generic.instance id="Rectangle.draw<\"bound0\" & \"local\">" template=Rectangle.draw arguments=("bound0" & "local")
"#);
}

/// A member call on a union alias parameter dispatches to each variant.
#[test]
fn test_union_alias_parameter_dispatches_each_variant() {
    let session = TestSession::single(
        r#"
struct Rectangle {
    draw(): void {}
}

struct Circle {
    draw(): void {}
}

type Shape = Rectangle | Circle;

function draw(shape: Shape): void {
    shape.draw();
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Rectangle {
    draw(): void {}
}

struct Circle {
    draw(): void {}
}

type Shape = Rectangle | Circle;

function draw(shape: Shape): void {
    shape.draw();
}

=== dir ===
struct Rectangle {
/// @type.symbol symbol=Rectangle type=Rectangle
/// @definition.struct symbol=Rectangle
/// @definition.method symbol=Rectangle.draw source="draw(): void {}" slot=draw type=<Rectangle.draw.'a>(this: &Rectangle.draw.'a readonly Rectangle) => void

    draw(): void {}
    /// @generic.template symbol=Rectangle.draw parameters=('a)
    /// @type.symbol symbol=Rectangle.draw source="draw(): void {}" type=<Rectangle.draw.'a>(this: &Rectangle.draw.'a readonly Rectangle) => void
    /// @type.symbol symbol=Rectangle.draw.this type=&Rectangle.draw.'a readonly Rectangle

}

struct Circle {
/// @type.symbol symbol=Circle type=Circle
/// @definition.struct symbol=Circle
/// @definition.method symbol=Circle.draw source="draw(): void {}" slot=draw type=<Circle.draw.'a>(this: &Circle.draw.'a readonly Circle) => void

    draw(): void {}
    /// @generic.template symbol=Circle.draw parameters=('a)
    /// @type.symbol symbol=Circle.draw source="draw(): void {}" type=<Circle.draw.'a>(this: &Circle.draw.'a readonly Circle) => void
    /// @type.symbol symbol=Circle.draw.this type=&Circle.draw.'a readonly Circle

}

type Shape = Rectangle | Circle;
/// @type.symbol symbol=Shape source="type Shape = Rectangle | Circle" type=Rectangle | Circle
/// @definition.type symbol=Shape source="type Shape = Rectangle | Circle" value=Rectangle | Circle
/// @resolution.name source=Rectangle target=Rectangle
/// @resolution.name source=Circle target=Circle

function draw(shape: Shape): void {
/// @type.symbol symbol=draw type=(Shape) => void
/// @type.symbol symbol=draw.shape source="shape: Shape" type=Shape
/// @resolution.name source=Shape target=Shape

    shape.draw();
    /// @resolution.name source=shape target=draw.shape
    /// @resolution.member source=shape.draw type=<Rectangle.draw.'a>(this: &Rectangle.draw.'a readonly Rectangle) => void | <Circle.draw.'a>(this: &Circle.draw.'a readonly Circle) => void kind=union arms=[receiver=Rectangle, target=Rectangle.draw, type=<Rectangle.draw.'a>(this: &Rectangle.draw.'a readonly Rectangle) => void, receiver=Circle, target=Circle.draw, type=<Circle.draw.'a>(this: &Circle.draw.'a readonly Circle) => void]
    /// @resolution.call source=shape.draw() return=void kind=union arms=[Rectangle.draw(parameters=(), arguments=(), return=void, regions=("frame" & "local")), Circle.draw(parameters=(), arguments=(), return=void, regions=("frame" & "local"))]
    /// @resolution.place source=shape placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=shape root=draw.shape
    /// @generic.instantiation id="Circle.draw<\"frame\" & \"local\">" template=Circle.draw arguments=("frame" & "local")
    /// @generic.instantiation id="Rectangle.draw<\"frame\" & \"local\">" template=Rectangle.draw arguments=("frame" & "local")
    /// @generic.instance id="Circle.draw<\"bound0\" & \"local\">" template=Circle.draw arguments=("bound0" & "local")
    /// @generic.instance id="Rectangle.draw<\"bound0\" & \"local\">" template=Rectangle.draw arguments=("bound0" & "local")

}
"#);
}

/// A member call on a union selects one overload per variant.
#[test]
fn test_union_member_call_selects_one_overload_per_variant() {
    let session = TestSession::single(
        r#"
struct Left {
    parse(value: string): "left-string" {
        return "left-string";
    }

    parse(value: int32): "left-integer" {
        return "left-integer";
    }
}

struct Right {
    parse(value: string): "right-string" {
        return "right-string";
    }

    parse(value: int32): "right-integer" {
        return "right-integer";
    }
}

declare const parser: Left | Right;
const result = parser.parse(1);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Left {
    parse(value: string): "left-string" {
        return "left-string";
    }

    parse(value: int32): "left-integer" {
        return "left-integer";
    }
}

struct Right {
    parse(value: string): "right-string" {
        return "right-string";
    }

    parse(value: int32): "right-integer" {
        return "right-integer";
    }
}

declare const parser: Left | Right;
const result: "left-integer" | "right-integer" = parser.parse(1);

=== dir ===
struct Left {
/// @type.symbol symbol=Left type=Left
/// @definition.struct symbol=Left
/// @definition.method symbol=Left.parse#1 slot=parse type=<Left.parse#1.'a>(this: &Left.parse#1.'a readonly Left, string) => "left-string"
/// @definition.method symbol=Left.parse#2 slot=parse type=<Left.parse#2.'a>(this: &Left.parse#2.'a readonly Left, int32) => "left-integer"

    parse(value: string): "left-string" {
    /// @generic.template symbol=Left.parse#1 parameters=('a)
    /// @type.symbol symbol=Left.parse#1 type=<Left.parse#1.'a>(this: &Left.parse#1.'a readonly Left, string) => "left-string"
    /// @type.symbol symbol=Left.parse.this#1 type=&Left.parse#1.'a readonly Left
    /// @type.symbol symbol=Left.parse.value#1 source="value: string" type=string

        return "left-string";
    }

    parse(value: int32): "left-integer" {
    /// @generic.template symbol=Left.parse#2 parameters=('a)
    /// @type.symbol symbol=Left.parse#2 type=<Left.parse#2.'a>(this: &Left.parse#2.'a readonly Left, int32) => "left-integer"
    /// @type.symbol symbol=Left.parse.this#2 type=&Left.parse#2.'a readonly Left
    /// @type.symbol symbol=Left.parse.value#2 source="value: int32" type=int32

        return "left-integer";
    }
}

struct Right {
/// @type.symbol symbol=Right type=Right
/// @definition.struct symbol=Right
/// @definition.method symbol=Right.parse#1 slot=parse type=<Right.parse#1.'a>(this: &Right.parse#1.'a readonly Right, string) => "right-string"
/// @definition.method symbol=Right.parse#2 slot=parse type=<Right.parse#2.'a>(this: &Right.parse#2.'a readonly Right, int32) => "right-integer"

    parse(value: string): "right-string" {
    /// @generic.template symbol=Right.parse#1 parameters=('a)
    /// @type.symbol symbol=Right.parse#1 type=<Right.parse#1.'a>(this: &Right.parse#1.'a readonly Right, string) => "right-string"
    /// @type.symbol symbol=Right.parse.this#1 type=&Right.parse#1.'a readonly Right
    /// @type.symbol symbol=Right.parse.value#1 source="value: string" type=string

        return "right-string";
    }

    parse(value: int32): "right-integer" {
    /// @generic.template symbol=Right.parse#2 parameters=('a)
    /// @type.symbol symbol=Right.parse#2 type=<Right.parse#2.'a>(this: &Right.parse#2.'a readonly Right, int32) => "right-integer"
    /// @type.symbol symbol=Right.parse.this#2 type=&Right.parse#2.'a readonly Right
    /// @type.symbol symbol=Right.parse.value#2 source="value: int32" type=int32

        return "right-integer";
    }
}

declare const parser: Left | Right;
/// @type.symbol symbol=parser source=parser type=Left | Right
/// @resolution.pattern source=parser kind=binding target=parser
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

const result = parser.parse(1);
/// @type.symbol symbol=result source=result type="left-integer" | "right-integer"
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=parser target=parser
/// @resolution.member source=parser.parse type=<Left.parse#1.'a>(this: &Left.parse#1.'a readonly Left, string) => "left-string" & <Left.parse#2.'a>(this: &Left.parse#2.'a readonly Left, int32) => "left-integer" | <Right.parse#1.'a>(this: &Right.parse#1.'a readonly Right, string) => "right-string" & <Right.parse#2.'a>(this: &Right.parse#2.'a readonly Right, int32) => "right-integer" kind=union arms=[receiver=Left, target=Left.parse#1 | Left.parse#2, type=<Left.parse#1.'a>(this: &Left.parse#1.'a readonly Left, string) => "left-string" & <Left.parse#2.'a>(this: &Left.parse#2.'a readonly Left, int32) => "left-integer", receiver=Right, target=Right.parse#1 | Right.parse#2, type=<Right.parse#1.'a>(this: &Right.parse#1.'a readonly Right, string) => "right-string" & <Right.parse#2.'a>(this: &Right.parse#2.'a readonly Right, int32) => "right-integer"]
/// @resolution.call source=parser.parse(1) return="left-integer" | "right-integer" kind=union arms=[Left.parse#2(parameters=(int32), arguments=(provided(1) as int32), return="left-integer", regions=("static" & "local")), Right.parse#2(parameters=(int32), arguments=(provided(1) as int32), return="right-integer", regions=("static" & "local"))]
/// @resolution.place source=parser placement="local" lifetime="static" access="immutable"
/// @resolution.access source=parser root=parser
/// @generic.instantiation id="Left.parse#2<\"static\" & \"local\">" template=Left.parse#2 arguments=("static" & "local")
/// @generic.instantiation id="Right.parse#2<\"static\" & \"local\">" template=Right.parse#2 arguments=("static" & "local")
/// @generic.instance id="Left.parse#2<\"bound0\" & \"local\">" template=Left.parse#2 arguments=("bound0" & "local")
/// @generic.instance id="Right.parse#2<\"bound0\" & \"local\">" template=Right.parse#2 arguments=("bound0" & "local")
"#,
    );
}

/// A subscript on a union selects the index conformance of each variant.
#[test]
fn test_union_subscript_selects_each_protocol_call() {
    let session = TestSession::single(
        r#"
declare const values: int32[] | string[];
const first = values[0];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: int32[] | string[];
const first: int32 | string = values[0];

=== dir ===
declare const values: int32[] | string[];
/// @type.symbol symbol=values source=values type=int32[] | string[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=Array<string> template=Array arguments=(string)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<string>> template=sliceAssumeInit arguments=(MaybeUninit<string>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<string>> template=sliceUninit arguments=(MaybeUninit<string>)

const first = values[0];
/// @type.symbol symbol=first source=first type=int32 | string
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[0] type=int32 | string kind=union arms=[index#2(parameters=(isize), arguments=(provided(0) as isize), return=int32, regions=("managed" & "local")), index#2(parameters=(isize), arguments=(provided(0) as isize), return=string, regions=("managed" & "local"))]
/// @generic.instantiation id="index#2<int32, \"managed\" & \"local\">" template=index#2 arguments=(int32, "managed" & "local")
/// @generic.instantiation id="index#2<string, \"managed\" & \"local\">" template=index#2 arguments=(string, "managed" & "local")
/// @generic.instance id="index#2<int32, \"bound0\" & \"local\">" template=index#2 arguments=(int32, "bound0" & "local")
/// @generic.instance id="index#2<string, \"bound0\" & \"local\">" template=index#2 arguments=(string, "bound0" & "local")
"#,
    );
}
