# Class Visibility

## private fields

### private fields reject outside access

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

```ds
class Counter {
    private static value: int32 = 1;
}

Counter.value;
```

- contains: is private

## protected fields

### protected fields allow subclass access

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
