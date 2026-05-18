# Class Members

## instance members

### instance fields keep declared types

```ds
class Counter {
    value: int32 = 0;
}

const counter = new Counter();
const value = counter.value;
value satisfies int32;
```

### instance methods keep return types

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

### virtual methods type check as instance methods

```ds
class Counter {
    value: int32 = 0;

    virtual increment(): int32 {
        this.value = this.value + 1;
        this.value
    }
}

const counter = new Counter();
const next = counter.increment();
next satisfies int32;
```

### fields and methods can share names

Property access and method calls use different member projections.

```ds
class Counter {
    count: int32 = 0;

    count(): string {
        "count"
    }
}

const counter = new Counter();

counter.count satisfies int32;
counter.count() satisfies string;
```

### callable fields can be selected explicitly

Parentheses select the field before calling it.

```ds
class Runner {
    run: () => "field" = () => "field";

    run(): "method" {
        "method"
    }
}

const runner = new Runner();

runner.run() satisfies "method";
(runner.run)() satisfies "field";
```

### instance members are unavailable on class values

```ds
class Counter {
    value: int32 = 0;
}

const value = Counter.value;
```

- contains: does not exist

## static members

### static fields live on class values

```ds
class Counter {
    static defaultValue: int32 = 0;
}

const value = Counter.defaultValue;
value satisfies number;
```

### static fields infer from initializers

```ds
class Counter {
    static defaultValue = 1;
}

const value = Counter.defaultValue;
value satisfies int32;
```

### static fields are unavailable on instances

```ds
class Counter {
    static defaultValue: int32 = 0;
}

const counter = new Counter();
const value = counter.defaultValue;
```

- contains: does not exist

### static methods keep return types

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

### static methods are unavailable on instances

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

### static blocks can access static members

```ds
class Counter {
    static value: int32 = 1;

    static {
        Counter.value = 2;
    }
}

Counter.value satisfies int32;
```

## accessors

### accessors expose property types

```ds
class Counter {
    private value: int32 = 0;

    get count(): int32 {
        this.value
    }

    set count(next: int32) {
        this.value = next;
    }
}

const counter = new Counter();
counter.count satisfies int32;
counter.count = 1;
```

### accessors check assignments

```ds
class Counter {
    private value: int32 = 0;

    get count(): int32 {
        this.value
    }

    set count(next: int32) {
        this.value = next;
    }
}

const counter = new Counter();
counter.count = "bad";
```

- contains: not assignable

### fields and accessors share property names

Fields and accessors cannot both answer the same property access.

```ds
class Counter {
    count: int32 = 0;

    get count(): int32 {
        this.count
    }
}
```

- contains: duplicate member

## rejections

### abstract fields cannot have initializers

```ds
abstract class Counter {
    abstract value: int32 = 1;
}
```

- contains: invalid member modifier

### readonly fields reject assignment

```ds
class Counter {
    readonly value: int32 = 0;

    update() {
        this.value = 1;
    }
}
```

- contains: readonly

### declare fields cannot include initializers

```ds
class Counter {
    declare value: int32 = 1;
}
```

- contains: invalid member modifier

### constructors reject override

```ds
class Base {}

class Counter extends Base {
    override constructor() {}
}
```

- contains: invalid constructor

### static methods cannot be abstract

```ds
abstract class Counter {
    static abstract increment(): void;
}
```

- contains: invalid member modifier

### constructors cannot have generic parameters

```ds
class Counter {
    constructor<T>(value: T) {}
}
```

- contains: invalid constructor

### declare cannot apply to accessors

```ds
class Counter {
    declare get value(): number;
}
```

- contains: invalid member modifier

### declare cannot combine with override

```ds
class Base {
    greet(): void {}
}

class Counter extends Base {
    declare override greet(): void;
}
```

- contains: invalid member modifier
