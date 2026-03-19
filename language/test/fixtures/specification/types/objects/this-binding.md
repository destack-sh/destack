# This Binding Edge Cases

## strict bind call apply

### bind accepts compatible this arguments

> Under strict `bind/call/apply`, rebinding should succeed when the provided receiver satisfies the declared `this` type.

```ts
function use(this: { tag: "ok" }, value: number): number {
    return value;
}

const bound = use.bind({ tag: "ok" });
bound(1) satisfies number;
```

### bind rejects incompatible this arguments

> Rebinding should fail when the provided receiver is incompatible with the method's declared `this` parameter.

```ts
function use(this: { tag: "ok" }, value: number): number {
    return value;
}

use.bind({ tag: "bad" });
```

- contains: not assignable

## callbacks

### callback declarations can include explicit this receivers

> Callback signatures with explicit `this` parameters should contextualize receiver typing at call sites.

```ts
declare function invoke(callback: (this: { tag: "ok" }, value: number) => number): number;

const value = invoke(function (this: { tag: "ok" }, current: number): number {
    return current;
});

value satisfies number;
```

## polymorphic static this

### static polymorphic this can preserve subclass instance returns

> Polymorphic static `this` should preserve subclass instance return types when called through derived constructors.

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
