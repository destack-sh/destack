# Class Visibility

Member visibility is enforced, not advisory.

## private fields

### private fields reject outside access

`private` members are class-internal.

```ds
class Counter {
    private value: int32 = 0;

    get(): int32 {
        this.value
    }
}

const counter = new Counter();
const out = counter.value;
```

- contains: is private

### static private fields allow class access

Statics see their own private statics.

```ds
class Counter {
    private static value: int32 = 1;

    static get(): int32 {
        Counter.value
    }
}

Counter.get() satisfies int32;
```

### static private fields reject outside access

Private statics stay internal.

```ds
class Counter {
    private static value: int32 = 1;
}

Counter.value;
```

- contains: is private

## protected fields

### protected fields allow subclass access

`protected` members extend to subclasses.

```ds
class Base {
    protected value: int32 = 0;
}

class Child extends Base {
    get(): int32 {
        return this.value;
    }
}
```

### protected fields reject outside access

Outside the hierarchy, protected is private.

```ds
class Base {
    protected value: int32 = 0;
}

class Child extends Base {
    get(): int32 {
        return this.value;
    }
}

const child = new Child();
const out = child.value;
```

- contains: '<anonymous>' is protected
