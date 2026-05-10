# Export Statements

Export fixtures cover named exports, default exports, re-exports, and export comments.

## Named Exports

### export braces have internal spacing

Export braces get internal spacing, like imports.

```ds
export {foo,bar,baz}
```

Spaces are added after `{` and before `}`.

```ds expected
export { bar, baz, foo };
```

### single named export

Single exports also get internal spacing.

```ds
export { foo }
```

```ds expected
export { foo };
```

### export with alias

Exports can rename values using `as`.

```ds
export { foo as bar }
```

```ds expected
export { foo as bar };
```

### multiple exports with aliases

Multiple exports can each have aliases.

```ds
export { foo as f, bar as b, baz as z }
```

```ds expected
export { bar as b, foo as f, baz as z };
```

### export as default

Values can be exported as the default export.

```ds
export { foo as default }
```

```ds expected
export { foo as default };
```

### export all from module

Re-exporting all keeps spacing and semicolons.

```ds
export * from "module"
```

```ds expected
export * from "module";
```

### export namespace from module

Namespace re-exports keep spacing and semicolons.

```ts:main.ts
export * as Utils from "module"
```

```ts expected
export * as Utils from "module";
```

## Inline Exports

### export const

Constants can be exported inline.

```ds
export const x = 1
```

```ds expected
export const x = 1;
```

### export let

Mutable variables can be exported inline.

```ds
export let y = 2
```

```ds expected
export let y = 2;
```

### export function

Functions can be exported inline.

```ds
export function foo() { }
```

```ds expected
export function foo() {}
```

### export class

Classes can be exported inline.

```ds
export class Foo { }
```

```ds expected
export class Foo {}
```

### export interface

Interfaces with members expand to multiple lines.

```ds
export interface Foo { x: number }
```

```ds expected
export interface Foo {
    x: number;
}
```

### export type alias

Type aliases can be exported inline.

```ds
export type Foo = number
```

```ds expected
export type Foo = number;
```

### export struct

Structs with fields expand to multiple lines.

```ds
export struct Point { x: number; y: number }
```

```ds expected
export struct Point {
    x: number;
    y: number;
}
```

### export enum

Enums expand to multiple lines with trailing commas on variants.

```ds
export enum Status { Active; Inactive }
```

```ds expected
export enum Status {
    Active,
    Inactive,
}
```

## Default Exports

### export default function

Functions can be the default export.

```ds
export default function handler() { }
```

```ds expected
export default function handler() {}
```

### export default class

Classes can be the default export.

```ds
export default class Handler { }
```

```ds expected
export default class Handler {}
```

### export default expression

Expressions can be the default export.

```ds
export default 42
```

```ds expected
export default 42;
```

### export default object

Object literals can be the default export.

```ds
export default { x: 1, y: 2 }
```

```ds expected
export default { x: 1, y: 2 };
```

### export default arrow function

Arrow functions can be the default export.

```ds
export default (x) => x * 2
```

```ds expected
export default (x) => x * 2;
```

## Type Exports

### type-only export

Type-only exports use `export type`.

```ds
export type { Foo }
```

```ds expected
export type { Foo };
```

### mixed type and value exports

Type and value exports can be mixed using `type` modifier on individual items.

```ds
export { type Foo, bar }
```

```ds expected
export { type Foo, bar };
```

### multiple type exports

Multiple types can be exported together.

```ds
export { type Foo, type Bar, type Baz }
```

```ds expected
export { type Bar, type Baz, type Foo };
```

## Line Breaking

### long export breaks

When exports exceed the line width, they break to multiple lines.

```ds line-width=40
export { veryLongName, anotherLongName, thirdLongName }
```

Each export goes on its own line with a trailing comma.

```ds expected
export {
    anotherLongName,
    thirdLongName,
    veryLongName,
};
```

### many exports break

When exports exceed the line width, they break to multiple lines.

```ds line-width=30
export { a, b, c, d, e, f, g }
```

```ds expected
export {
    a,
    b,
    c,
    d,
    e,
    f,
    g,
};
```
