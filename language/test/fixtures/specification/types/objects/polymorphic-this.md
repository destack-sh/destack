# This Type

## polymorphic this

### class instance this return keeps subclass receiver

When an instance method returns `this`, calls through subclasses should preserve the subclass receiver type.

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

### class static this return keeps subclass constructor type

When a static method returns `this`, calls through subclasses should preserve the subclass constructor type.

```ts
class Base {
    static make(this: new () => Base): Base {
        return new this();
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

### this parameters reject incompatible receiver arguments

Functions with explicit `this` parameters should reject calls that provide an incompatible receiver.

```ts
function use(this: { kind: "ok" }, value: number): number {
    return value;
}

use.call({ kind: "bad" }, 1);
```

- contains: not assignable

### extracted methods preserve this parameter requirements

Extracting a method value should still enforce its explicit `this` parameter requirement when invoked.

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
{ "compilerOptions": { "allowTs": true, "checkTs": true, "noImplicitThis": true } }
```
