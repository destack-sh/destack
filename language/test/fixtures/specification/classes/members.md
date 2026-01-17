# Class Members

## instance members

### instance field access yields field type

> Instance fields are typed by their declarations.

```ds
class Counter {
    value: int32 = 0;
}

const counter = new Counter();
const value = counter.value;
value satisfies int32;
```

### instance method call yields return type

> Instance methods return their declared types.

```ds
class Counter {
    value: int32 = 0;

    increment(): int32 {
        this.value = this.value + 1;
        this.value
    }
}

const counter = new Counter();
const next = counter.increment();
next satisfies int32;
```

### instance members are not available on classes

> Instance members are not accessible from the class value.

```ds
class Counter {
    value: int32 = 0;
}

const value = Counter.value;
```

- contains: does not exist

## static members

### static field access yields field type

> Static fields are accessed on the class.

```ds
class Counter {
    static defaultValue: int32 = 0;
}

const value = Counter.defaultValue;
value satisfies int32;
```

### static fields are not available on instances

> Static fields are not accessible from instances.

```ds
class Counter {
    static defaultValue: int32 = 0;
}

const counter = new Counter();
const value = counter.defaultValue;
```

- contains: does not exist

### static method call yields return type

> Static methods return their declared types.

```ds
class Counter {
    value: int32 = 0;

    static make(value: int32): Counter {
        const counter = new Counter();
        counter.value = value;
        counter
    }
}

const counter = Counter.make(1);
counter satisfies Counter;
```

### static methods are not available on instances

> Static methods are not accessible from instances.

```ds
class Counter {
    static make(): int32 {
        1
    }
}

const counter = new Counter();
const value = counter.make();
```

- contains: does not exist
