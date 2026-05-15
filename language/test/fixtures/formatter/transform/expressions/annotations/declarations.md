# Declaration Annotations

## Declaration Forms

### constructor parameter modifiers

Constructor parameter modifiers are parsed and formatted.

```ts:main.ts
class C {
  constructor(readonly x: number) {}
}

class D {
  constructor(public readonly x: number) {}
}

class E {
  constructor(private readonly x: number) {}
}
```

```ts expected
class C {
    constructor(readonly x: number) {}
}

class D {
    constructor(public readonly x: number) {}
}

class E {
    constructor(private readonly x: number) {}
}
```

### ambient declaration forms

Ambient declaration forms are parsed and formatted.

```ts:main.ts
declare type A = true;
declare function b(): "hello";
declare const foo: "bar";
declare let qux: boolean;
declare enum Kind {}
declare interface Shape {}
declare class Box {}
```

```ts expected
declare type A = true;
declare function b(): "hello";
declare const foo: "bar";
declare let qux: boolean;
declare enum Kind {}
declare interface Shape {}
declare class Box {}
```

### module declaration bodies

Module declaration bodies are parsed and formatted.

```ds
module {
    tree: HtmlTree
}
```

```ds expected
module {
    tree: HtmlTree;
}
```

### overload signatures with optional and rest parameters

Overload signatures with optional and rest parameters are parsed.

```ts:main.ts
function fn4a(x?: number, y: string)
function fn4a() {}

function fn5(x: string, y: string, ...rest: any[])
function fn5() {}
```

```ts expected
function fn4a(x?: number, y: string);
function fn4a() {}

function fn5(x: string, y: string, ...rest: any[]);
function fn5() {}
```
