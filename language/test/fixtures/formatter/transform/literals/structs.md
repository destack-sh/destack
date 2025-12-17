# Struct Literals

Tests for Destack struct literal formatting.

## Basic Structs

### simple struct literal

Struct literals use the type name followed by braces with field assignments.

```ds
Point { x : 1 , y : 2 }
```

Field colons have no space before and one space after.

```ds expected
Point { x: 1, y: 2 };
```

### struct literal with spread

The spread operator attaches directly to the identifier with no space.

```ds
Point { x : 1 , ... other }
```

```ds expected
Point { x: 1, ...other };
```

### empty struct literal

Empty structs preserve internal spacing.

```ds
Empty {  }
```

```ds expected
Empty { };
```

### single field struct

Single fields have spacing normalized.

```ds
Wrapper {   value : 42   }
```

```ds expected
Wrapper { value: 42 };
```

### struct with shorthand

Shorthand fields use just the name when variable matches field.

```ds
Point { x, y }
```

```ds expected
Point { x, y };
```

## Line Breaking

### struct breaks at line width

Long structs expand to multiple lines with trailing commas.

```ds line-width=25
Point { x: 100, y: 200, z: 300 }
```

```ds expected
Point {
    x: 100,
    y: 200,
    z: 300,
};
```

### struct with long field names

Long field names trigger line breaking.

```ds line-width=40
Config { firstName: "John", lastName: "Doe" }
```

```ds expected
Config {
    firstName: "John",
    lastName: "Doe",
};
```

## Nested Structs

### _nested struct literal

Nested structs stay on one line if short.

```ds
Outer { inner: Inner { value: 1 } }
```

```ds expected
Outer { inner: Inner { value: 1 } };
```

### deeply nested struct

Deep nesting expands outer levels while keeping inner compact.

```ds line-width=40
Level1 { level2: Level2 { level3: Level3 { value: 1 } } }
```

```ds expected
Level1 {
    level2: Level2 {
        level3: Level3 { value: 1 },
    },
};
```

## Struct with Expressions

### _struct with computed values

Expressions can be struct field values.

```ds
Point { x: a + b, y: c * d }
```

```ds expected
Point { x: a + b, y: c * d };
```

### _struct with function calls

Function calls can be struct field values.

```ds
Config { value: getValue(), name: getName() }
```

```ds expected
Config { value: getValue(), name: getName() };
```

### _struct with method call result

Method call results can be struct field values.

```ds
Result { data: source.transform() }
```

```ds expected
Result { data: source.transform() };
```

## Struct as Arguments

### struct in function call

Structs can be passed directly as arguments.

```ds
process(Point { x: 1, y: 2 })
```

```ds expected
process(Point { x: 1, y: 2 });
```

### struct argument breaks

Long struct arguments cause both to break.

```ds line-width=30
process(Config { name: "test", value: 42 })
```

```ds expected
process(
    Config {
        name: "test",
        value: 42,
    },
);
```

## Struct Construction

### newtype scalar construction

Newtype wrappers use tuple-like syntax.

```ds
UserId(42)
```

```ds expected
UserId(42);
```

### newtype tuple construction

Tuple structs use parentheses.

```ds
Point(1.0, 2.0)
```

```ds expected
Point(1.0, 2.0);
```

### struct with mixed content

Complex structs with nested types expand.

```ds
Entity { id: UserId(1), position: Point { x: 0, y: 0 }, active: true }
```

```ds expected
Entity {
    id: UserId(1),
    position: Point { x: 0, y: 0 },
    active: true,
};
```

## Struct in Collections

### _array of structs

Arrays of structs stay on one line if short.

```ds
[Point { x: 1, y: 1 }, Point { x: 2, y: 2 }]
```

```ds expected
[Point { x: 1, y: 1 }, Point { x: 2, y: 2 }];
```

### array of structs breaks

Long arrays of structs break with one per line.

```ds line-width=40
[Point { x: 1, y: 1 }, Point { x: 2, y: 2 }, Point { x: 3, y: 3 }]
```

```ds expected
[
    Point { x: 1, y: 1 },
    Point { x: 2, y: 2 },
    Point { x: 3, y: 3 },
];
```

## Variable Binding

### struct literal in const

Structs can be assigned to constants.

```ds
const p = Point { x: 1, y: 2 }
```

```ds expected
const p = Point { x: 1, y: 2 };
```

### struct with type annotation

Explicit type annotations work with struct literals.

```ds
const p: Point = Point { x: 1, y: 2 }
```

```ds expected
const p: Point = Point { x: 1, y: 2 };
```
