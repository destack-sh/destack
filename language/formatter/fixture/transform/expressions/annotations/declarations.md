# Declaration Annotations

## Declaration Forms

### ambient declaration forms

Ambient declaration forms are parsed and formatted.

```tspp:main.tspp
declare type A = true;
declare function b(): "hello";
declare const foo: "bar";
declare let qux: boolean;
declare enum Kind {}
declare interface Shape {}
declare class Box {}
```

```tspp expected
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

```tspp
module {
    const tree = HtmlTree
}
```

```tspp expected
module {
    const tree = HtmlTree;
}
```

### overload signatures with optional and rest parameters

Overload signatures with optional and rest parameters are parsed.

```tspp:main.tspp
function fn4a(x?: number, y: string)
function fn4a() {}

function fn5(x: string, y: string, ...rest: any[])
function fn5() {}
```

```tspp expected
function fn4a(x?: number, y: string);
function fn4a() {}

function fn5(x: string, y: string, ...rest: any[]);
function fn5() {}
```
