# Struct Literals

Struct literal fixtures cover fields, spreads, comments, nested structs, and constructor-like forms.

## Struct Literals

### struct literal

Struct literals use the type name followed by braces with field assignments.

```tspp
Point { x : 1 , y : 2 }
```

Field colons have no space before and one space after.

```tspp expected
Point { x: 1, y: 2 };
```

### struct literal with spread

The spread operator attaches directly to the identifier with no space.

```tspp
Point { x : 1 , ... other }
```

```tspp expected
Point { x: 1, ...other };
```

### empty struct literal

Empty structs stay compact.

```tspp
Empty {  }
```

```tspp expected
Empty {};
```

### inferred struct literal

Inferred struct literals use `_` as the type marker.

```tspp
_ { x : 1 }
```

```tspp expected
_ { x: 1 };
```

### single field struct

Single fields have spacing normalized.

```tspp
Wrapper {   value : 42   }
```

```tspp expected
Wrapper { value: 42 };
```

### struct with shorthand

Shorthand fields use just the name when variable matches field.

```tspp
Point { x, y }
```

```tspp expected
Point { x, y };
```

### tail struct literal expression

Struct literals in function tail position keep expression value.

```tspp
function point(x: number, y: number): Point { Point { x, y } }
```

```tspp expected
function point(x: number, y: number): Point {
    Point { x, y }
}
```

### parenthesized statement struct literal

Parenthesized struct literal statements drop the grouping and keep the semicolon.

```tspp
function point(x: number, y: number): void { (Point { x, y }); }
```

```tspp expected
function point(x: number, y: number): void {
    Point { x, y };
}
```

## Line Breaking

### struct breaks at line width

Long structs expand to multiple lines with trailing commas.

```tspp line-width=25
Point { x: 100, y: 200, z: 300 }
```

```tspp expected
Point {
    x: 100,
    y: 200,
    z: 300,
};
```

### struct with long field names

Long field names trigger line breaking.

```tspp line-width=40
Config { firstName: "John", lastName: "Doe" }
```

```tspp expected
Config {
    firstName: "John",
    lastName: "Doe",
};
```

## Comments

### struct field sibling comments

Trailing field comments and next field leading comments keep separate ownership.

```tspp
Point { x: first, // x field
// y field
y: second }
```

```tspp expected
Point {
    x: first, // x field
    // y field
    y: second,
};
```

### struct shorthand comments

Comments around shorthand fields stay inside the struct literal.

```tspp
Point { /* x */ x, /* y */ y }
```

```tspp expected
Point { /* x */ x, /* y */ y };
```

## Nested Structs

### nested struct literal

Nested structs stay on one line if short.

```tspp
Outer { inner: Inner { value: 1 } }
```

```tspp expected
Outer { inner: Inner { value: 1 } };
```

### deeply nested struct

Deep nesting expands outer levels while keeping inner compact.

```tspp line-width=40
Level1 { level2: Level2 { level3: Level3 { value: 1 } } }
```

```tspp expected
Level1 {
    level2: Level2 {
        level3: Level3 { value: 1 },
    },
};
```

## Struct with Expressions

### struct with computed values

Expressions can be struct field values.

```tspp
Point { x: a + b, y: c * d }
```

```tspp expected
Point { x: a + b, y: c * d };
```

### struct with function calls

Function calls can be struct field values.

```tspp
Config { value: getValue(), name: getName() }
```

```tspp expected
Config { value: getValue(), name: getName() };
```

### struct with method call result

Method call results can be struct field values.

```tspp
Result { data: source.transform() }
```

```tspp expected
Result { data: source.transform() };
```

## Struct as Arguments

### struct in function call

Structs can be passed directly as arguments.

```tspp
process(Point { x: 1, y: 2 })
```

```tspp expected
process(Point { x: 1, y: 2 });
```

### struct argument breaks

Long struct arguments expand with hugging.

```tspp line-width=30
process(Config { name: "test", value: 42 })
```

```tspp expected
process(Config {
    name: "test",
    value: 42,
});
```

## Struct Construction

### newtype scalar construction

Newtype wrappers use tuple-like syntax.

```tspp
UserId(42)
```

```tspp expected
UserId(42);
```

### newtype tuple construction

Tuple structs use parentheses.

```tspp
Point(1.0, 2.0)
```

```tspp expected
Point(1.0, 2.0);
```

### struct with mixed content

Complex structs with nested types expand when they exceed line width.

```tspp line-width=60
Entity { id: UserId(1), position: Point { x: 0, y: 0 }, active: true }
```

```tspp expected
Entity {
    id: UserId(1),
    position: Point { x: 0, y: 0 },
    active: true,
};
```

## Struct in Collections

### array of structs

Short arrays of structs stay inline when they fit.

```tspp
[Point { x: 1, y: 1 }, Point { x: 2, y: 2 }]
```

```tspp expected
[Point { x: 1, y: 1 }, Point { x: 2, y: 2 }];
```

### array of structs breaks

Long arrays of structs break with one per line.

```tspp line-width=40
[Point { x: 1, y: 1 }, Point { x: 2, y: 2 }, Point { x: 3, y: 3 }]
```

```tspp expected
[
    Point { x: 1, y: 1 },
    Point { x: 2, y: 2 },
    Point { x: 3, y: 3 },
];
```

## Variable Binding

### struct literal in const

Structs can be assigned to constants.

```tspp
const p = Point { x: 1, y: 2 }
```

```tspp expected
const p = Point { x: 1, y: 2 };
```

### struct with type annotation

Explicit type annotations work with struct literals.

```tspp
const p: Point = Point { x: 1, y: 2 }
```

```tspp expected
const p: Point = Point { x: 1, y: 2 };
```
