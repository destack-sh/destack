# Object Literals

Tests for object literal formatting.

## Spacing

### object literals have internal spacing

Object braces get internal spacing, unlike arrays.

```ds
const x = {a:1,b:2}
```

Spaces are added after `{` and before `}`.

```ds expected
const x = { a: 1, b: 2 };
```

### object property colon has trailing space only

Property colons have no space before and one space after.

```ds
const x = { a : 1 }
```

```ds expected
const x = { a: 1 };
```

### short objects stay on one line

Short object literals remain on a single line.

```ds
const x = { a: 1, b: 2 }
```

```ds expected
const x = { a: 1, b: 2 };
```

### empty object

Empty objects stay compact.

```ds
const x = {  }
```

```ds expected
const x = {};
```

### single property object

Single properties have spacing normalized.

```ds
const x = {   a : 1   }
```

```ds expected
const x = { a: 1 };
```

## Shorthand Properties

### shorthand property

Shorthand properties use the variable name as both key and value.

```ds
const x = { a, b, c }
```

```ds expected
const x = { a, b, c };
```

### mixed shorthand and regular

Shorthand and regular properties can be mixed.

```ds
const x = { a, b: 2, c }
```

```ds expected
const x = { a, b: 2, c };
```

### shorthand with method

Objects with methods expand to multiple lines.

```ds
const x = { a, method() { return 1 } }
```

```ds expected
const x = {
    a,
    method() {
        return 1;
    },
};
```

## Computed Properties

### computed property name

Computed properties use brackets around the key expression.

```ds
const x = { [key]: value }
```

```ds expected
const x = { [key]: value };
```

### computed property with expression

Any expression can be used as a computed key.

```ds
const x = { [a + b]: value }
```

```ds expected
const x = { [a + b]: value };
```

### computed property with template literal

Template literals can be computed keys.

```ds
const x = { [`prefix_${name}`]: value }
```

```ds expected
const x = { [`prefix_${name}`]: value };
```

## Quoted Properties

### quoted property names

Property names that require quotes stay quoted.

```ds
const x = { "data-id": 1, "default": 2 }
```

```ds expected
const x = { "data-id": 1, default: 2 };
```

### mixed quoted and unquoted

Only properties that require quotes stay quoted.

```ds
const x = { normal: 1, "needs-quotes": 2 }
```

```ds expected
const x = { normal: 1, "needs-quotes": 2 };
```

### typescript quote props as needed

TypeScript removes quotes when they are not required.

```ts:main.ts
const x = { "data-id": 1, "default": 2, "normal": 3 }
```

```ts expected
const x = { "data-id": 1, default: 2, normal: 3 };
```

### typescript quote props consistent

Consistent quote props quotes all keys when any require quotes.

```ts:main.ts quote-props=consistent
const x = { a: 1, "needs-quotes": 2, "default": 3 }
```

```ts expected
const x = { "a": 1, "needs-quotes": 2, "default": 3 };
```

### destack quote props consistent

Consistent quote props applies to shared object expression syntax.

```ds quote-props=consistent
const x = { a: 1, "needs-quotes": 2, "default": 3 }
```

```ds expected
const x = { "a": 1, "needs-quotes": 2, "default": 3 };
```

### typescript quote props preserve

Preserve keeps original quoting.

```ts:main.ts quote-props=preserve
const x = { "normal": 1, "needs-quotes": 2, default: 3 }
```

```ts expected
const x = { "normal": 1, "needs-quotes": 2, default: 3 };
```

## Methods

### method in object

Methods expand the object to multiple lines.

```ds
const x = { foo() { return 1 } }
```

```ds expected
const x = {
    foo() {
        return 1;
    },
};
```

### method with parameters

Method parameters follow function formatting rules.

```ds
const x = { add(a, b) { return a + b } }
```

```ds expected
const x = {
    add(a, b) {
        return a + b;
    },
};
```

### async method

Async methods use the `async` keyword before the name.

```ds
const x = { async fetch() { return await data } }
```

```ds expected
const x = {
    async fetch() {
        return await data;
    },
};
```

### generator method

Generator methods use `*` before the name.

```ds
const x = { *items() { yield 1; yield 2 } }
```

```ds expected
const x = {
    *items() {
        yield 1;
        yield 2;
    },
};
```

### getter

Getters use `get` keyword before the property name.

```ds
const x = { get value() { return this._value } }
```

```ds expected
const x = {
    get value() {
        return this._value;
    },
};
```

### setter

Setters use `set` keyword and take one parameter.

```ds
const x = { set value(v) { this._value = v } }
```

```ds expected
const x = {
    set value(v) {
        this._value = v;
    },
};
```

## Nested Objects

### nested object

Nested objects stay on one line if short.

```ds
const x = { a: { b: 1 } }
```

```ds expected
const x = { a: { b: 1 } };
```

### deeply nested object

Any depth of nesting is preserved if short.

```ds
const x = { a: { b: { c: { d: 1 } } } }
```

```ds expected
const x = { a: { b: { c: { d: 1 } } } };
```

### object with array value

Arrays can be object property values.

```ds
const x = { items: [1, 2, 3] }
```

```ds expected
const x = { items: [1, 2, 3] };
```

## Spread

### spread in object

Spread copies properties from another object.

```ds
const x = { ...other }
```

```ds expected
const x = { ...other };
```

### multiple spreads

Multiple spreads merge properties in order.

```ds
const x = { ...a, ...b, ...c }
```

```ds expected
const x = { ...a, ...b, ...c };
```

### spread between properties

Spread can appear between regular properties.

```ds
const x = { a: 1, ...middle, b: 2 }
```

```ds expected
const x = { a: 1, ...middle, b: 2 };
```

### spread with shorthand

Spread and shorthand properties can mix.

```ds
const x = { a, ...rest, b: 2 }
```

```ds expected
const x = { a, ...rest, b: 2 };
```

## Complex Objects

### config object pattern

Config objects expand to multiple lines when exceeding width.

```ds line-width=50
const config = { host: "localhost", port: 3000, debug: true, timeout: 5000 }
```

```ds expected
const config = {
    host: "localhost",
    port: 3000,
    debug: true,
    timeout: 5000,
};
```

### object with mixed content

Mixed properties and methods expand to multiple lines.

```ds line-width=40
const x = { name: "test", items: [1, 2], handler() { return this.name } }
```

```ds expected
const x = {
    name: "test",
    items: [1, 2],
    handler() {
        return this.name;
    },
};
```

### object as argument

Objects can be passed directly as function arguments.

```ds
foo({ a: 1, b: 2 })
```

```ds expected
foo({ a: 1, b: 2 });
```

## Type Annotations

### object with type annotation

Object type annotations use the same brace syntax.

```ds
const x: { a: number } = { a: 1 }
```

```ds expected
const x: { a: number } = { a: 1 };
```

### object satisfies type

`satisfies` checks type without changing inference.

```ds
const x = { a: 1 } satisfies Record<string, number>
```

```ds expected
const x = { a: 1 } satisfies Record<string, number>;
```

## As Const

### object as const

`as const` keeps the object literal inline when it fits.

```ts:main.ts
const settings = { retries: 3, verbose: false } as const
```

```ts expected
const settings = { retries: 3, verbose: false } as const;
```

## Unicode Keys (TypeScript)

### object with unicode keys

Unicode keys that are not identifiers stay quoted and normalize quotes.

```ts:main.ts
x = { 'x・': 0, 'x･': 1 }
```

```ts expected
x = { "x・": 0, "x･": 1 };
```
