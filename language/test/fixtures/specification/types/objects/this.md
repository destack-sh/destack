# This

`this` names the current receiver type inside receiver-shaped declarations.
Explicit `this` parameters use the ordinary TS call, bind, and apply rules in `.ts` sources.

## receiver types

### methods returning this keep the receiver type

Methods returning `this` preserve the receiver type.

```ds
interface Builder {
    build(): this
}

declare const builder: Builder;

let built = builder.build();
built satisfies Builder;
```

### this in properties resolves to the receiver type

Property types may refer to the receiver type.

```ds
interface Node {
    parent: this | null
}

declare const node: Node;

let parent = node.parent;
parent satisfies Node | null;
```

### this inside type arguments resolves to the receiver type

Parameterized types may use `this` where a type argument is expected.

```ds
interface Box<T> { value: T }

interface Builder {
    box(): Box<this>
}

declare const builder: Builder;

let boxed = builder.box();
boxed.value satisfies Builder;
```

### chained calls keep the receiver type

Method chains preserve the receiver type through each call.

```ds
interface Chain {
    set(value: int32): this
}

declare const chain: Chain;

let chained = chain.set(1).set(2);
chained satisfies Chain;
```

### this parameters require receiver-compatible arguments

Parameters typed as `this` require values compatible with the receiver type.

```ds
interface Builder {
    merge(other: this): this
}

declare const builder: Builder;

builder.merge(builder);
builder.merge("nope");
```

- contains: not assignable

## explicit receivers

### callback declarations can include explicit this receivers

Callback signatures may declare the receiver expected by the callback body.

```ts
declare function invoke(callback: (this: { tag: "ok" }, value: number) => number): number;

const value = invoke(function (this: { tag: "ok" }, current: number): number {
    return current;
});

value satisfies number;
```

### bind accepts compatible this arguments

`bind` accepts a receiver compatible with the function's explicit `this` parameter.

```ts
function use(this: { tag: "ok" }, value: number): number {
    return value;
}

const bound = use.bind({ tag: "ok" });
bound(1) satisfies number;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "noImplicitThis": true, "strictBindCallApply": true } }
```

### bind rejects incompatible this arguments

`bind` rejects receivers that do not satisfy the function's explicit `this` parameter.

```ts
function use(this: { tag: "ok" }, value: number): number {
    return value;
}

use.bind({ tag: "bad" });
```

- contains: not assignable

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "noImplicitThis": true, "strictBindCallApply": true } }
```

### extracted methods keep this parameter requirements

Extracting a method value keeps its explicit `this` parameter requirement.

```ts
const tool = {
    run(this: { tag: "tool" }, value: number): number {
        return value;
    }
};

const run = tool.run;
run.call({ tag: "bad" }, 1);
```

- contains: not assignable

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "noImplicitThis": true, "strictBindCallApply": true } }
```

## polymorphic this

### class instance methods keep subclass receivers

Instance methods returning `this` preserve subclass receiver types.

```ts
class Base {
    set(value: string): this {
        return this;
    }
}

class Child extends Base {
    childOnly(): "child" {
        return "child";
    }
}

const child = new Child().set("ok");
child.childOnly() satisfies "child";
```

### static methods keep subclass constructors

Static methods using `this` as the constructor receiver preserve subclass instance types.

```ts
class Base {
    static make<T extends typeof Base>(this: T): InstanceType<T> {
        return new this() as InstanceType<T>;
    }
}

class Child extends Base {
    childOnly(): "child" {
        return "child";
    }
}

const child = Child.make();
child.childOnly() satisfies "child";
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "noImplicitThis": true, "strictBindCallApply": true } }
```
