# Export Statements

Export fixtures cover named exports, default exports, re-exports, and export comments.

## Named Exports

### export braces have internal spacing

Export braces get internal spacing, like imports.

```tspp
export {foo,bar,baz}
```

Spaces are added after `{` and before `}`.

```tspp expected
export { bar, baz, foo };
```

### single named export

Single exports also get internal spacing.

```tspp
export { foo }
```

```tspp expected
export { foo };
```

### export with alias

Exports can rename values using `as`.

```tspp
export { foo as bar }
```

```tspp expected
export { foo as bar };
```

### multiple exports with aliases

Multiple exports can each have aliases.

```tspp
export { foo as f, bar as b, baz as z }
```

```tspp expected
export { bar as b, foo as f, baz as z };
```

### export as default

Values can be exported as the default export.

```tspp
export { foo as default }
```

```tspp expected
export { foo as default };
```

### export all from module

Re-exporting all keeps spacing and semicolons.

```tspp
export * from "module"
```

```tspp expected
export * from "module";
```

### export namespace from module

Namespace re-exports keep spacing and semicolons.

```tspp:main.tspp
export * as Utils from "module"
```

```tspp expected
export * as Utils from "module";
```

## Inline Exports

### export const

Constants can be exported inline.

```tspp
export const x = 1
```

```tspp expected
export const x = 1;
```

### export let

Mutable variables can be exported inline.

```tspp
export let y = 2
```

```tspp expected
export let y = 2;
```

### export function

Functions can be exported inline.

```tspp
export function foo() { }
```

```tspp expected
export function foo() {}
```

### export class

Classes can be exported inline.

```tspp
export class Foo { }
```

```tspp expected
export class Foo {}
```

### export interface

Interfaces with members expand to multiple lines.

```tspp
export interface Foo { x: number }
```

```tspp expected
export interface Foo {
    x: number;
}
```

### export type alias

Type aliases can be exported inline.

```tspp
export type Foo = number
```

```tspp expected
export type Foo = number;
```

### export struct

Structs with fields expand to multiple lines.

```tspp
export struct Point { x: number; y: number }
```

```tspp expected
export struct Point {
    x: number;
    y: number;
}
```

### export enum

Enums expand to multiple lines with trailing commas on variants.

```tspp
export enum Status { Active; Inactive }
```

```tspp expected
export enum Status {
    Active,
    Inactive,
}
```

## Default Exports

### export default function

Functions can be the default export.

```tspp
export default function handler() { }
```

```tspp expected
export default function handler() {}
```

### export default class

Classes can be the default export.

```tspp
export default class Handler { }
```

```tspp expected
export default class Handler {}
```

### export default expression

Expressions can be the default export.

```tspp
export default 42
```

```tspp expected
export default 42;
```

### export default object

Object literals can be the default export.

```tspp
export default { x: 1, y: 2 }
```

```tspp expected
export default { x: 1, y: 2 };
```

### export default arrow function

Arrow functions can be the default export.

```tspp
export default (x) => x * 2
```

```tspp expected
export default (x) => x * 2;
```

## Line Breaking

### long export breaks

When exports exceed the line width, they break to multiple lines.

```tspp line-width=40
export { veryLongName, anotherLongName, thirdLongName }
```

Each export goes on its own line with a trailing comma.

```tspp expected
export {
    anotherLongName,
    thirdLongName,
    veryLongName,
};
```

### many exports break

When exports exceed the line width, they break to multiple lines.

```tspp line-width=30
export { a, b, c, d, e, f, g }
```

```tspp expected
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
