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

### static field inference uses initializer

> Static fields without annotations infer from their initializer.

```ds
class Counter {
    static defaultValue = 1;
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

### static blocks can access static members

> Static blocks can reference class statics.

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

> Accessors define a property type for reads and writes.

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

### accessors reject incompatible assignments

> Accessor setters enforce the declared parameter type.

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

## parameter properties

### parameter properties declare and initialize members

> Parameter properties create fields and initialize them from constructor arguments.

```ds
class Counter {
    constructor(public value: int32, readonly label: string) {}
}

const counter = new Counter(1, "label");
counter.value = 2;
counter.label satisfies string;
```

### parameter properties require identifiers

> Parameter properties cannot use binding patterns.

```ds
class Counter {
    constructor(public {}: {}) {}
}
```

- contains: parameter property

### parameter properties cannot be rest parameters

> Parameter properties cannot be variadic.

```ds
class Counter {
    constructor(public ...values: int32[]) {}
}
```

- contains: parameter property

### readonly parameter properties reject assignment

> Readonly parameter properties cannot be reassigned.

```ds
class Counter {
    constructor(readonly label: string) {}

    update() {
        this.label = "next";
    }
}
```

- contains: readonly

### private parameter properties are inaccessible outside the class

> Private parameter properties follow class visibility rules.

```ds
class Counter {
    constructor(private value: int32) {}
}

const counter = new Counter(1);
counter.value;
```

- contains: is private

### protected parameter properties are accessible in subclasses

> Protected parameter properties can be used in derived classes.

```ds
class Base {
    constructor(protected value: int32) {}
}

class Derived extends Base {
    read(): int32 {
        this.value
    }
}

const derived = new Derived(1);
derived.read() satisfies int32;
```

### protected parameter properties are inaccessible outside subclasses

> Protected parameter properties are not visible from outside the hierarchy.

```ds
class Base {
    constructor(protected value: int32) {}
}

const base = new Base(1);
base.value;
```

- contains: is protected

### parameter properties support default values

> Constructor parameter property defaults initialize fields when omitted.

```ds
class Counter {
    constructor(public value: int32 = 4) {}
}

const counter = new Counter();
counter.value satisfies int32;
```

### parameter property modifiers are only valid on constructors

> Non-constructor methods cannot use parameter property modifiers.

```ds
class Counter {
    method(public value: int32) {}
}
```

- contains: parameter property

### private parameter properties keep classes nominally distinct

> Classes with different private parameter properties are not mutually assignable.

```ds
class Left {
    constructor(private value: int32) {}
}

class Right {
    constructor(private value: int32) {}
}

declare const right: Right;
const left: Left = right;
```

- contains: not assignable

### protected parameter properties keep classes nominally distinct

> Classes with different protected parameter properties are not mutually assignable.

```ds
class Left {
    constructor(protected value: int32) {}
}

class Right {
    constructor(protected value: int32) {}
}

declare const right: Right;
const left: Left = right;
```

- contains: not assignable

### parameter property fields are inherited by subclasses

> Subclasses inherit parameter-property fields from base constructors.

```ds
class Base {
    constructor(public value: int32) {}
}

class Derived extends Base {
    constructor(value: int32) {
        super(value);
    }
}

const derived = new Derived(1);
derived.value satisfies int32;
```

### readonly parameter properties cannot be reassigned in subclasses

> Readonly parameter-property fields remain readonly in derived methods.

```ds
class Base {
    constructor(readonly label: string) {}
}

class Derived extends Base {
    update() {
        this.label = "next";
    }
}
```

- contains: readonly

## invalid members

### abstract fields cannot have initializers

> Abstract fields are declarations only and cannot include initializers.

```ds
abstract class Counter {
    abstract value: int32 = 1;
}
```

- invalid member modifier

### readonly does not apply to methods

> Readonly modifiers are only valid on fields.

```ds
class Counter {
    readonly increment(): int32 {
        1
    }
}
```

- invalid member modifier

### readonly fields reject assignment

> Readonly fields cannot be assigned after initialization.

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

> Declared fields cannot include initializers.

```ds
class Counter {
    declare value: int32 = 1;
}
```

- invalid member modifier

### override constructors are invalid

> Constructors cannot use override modifiers.

```ds
class Base {}

class Counter extends Base {
    override constructor() {}
}
```

- invalid constructor

### abstract methods cannot have bodies

> Abstract methods are declarations only.

```ds
abstract class Counter {
    abstract increment(): void {}
}
```

- invalid abstract method

### static abstract methods are invalid

> Static methods cannot be abstract.

```ds
abstract class Counter {
    static abstract increment(): void;
}
```

- invalid member modifier

### constructors cannot have generic parameters

> Constructors cannot declare generic parameters.

```ds
class Counter {
    constructor<T>(value: T) {}
}
```

- invalid constructor

### declare methods cannot have bodies

> Declared methods cannot include bodies.

```ds
class Counter {
    declare increment(): void {}
}
```

- invalid member modifier

### declare accessors are invalid

> Declared members cannot use accessors.

```ds
class Counter {
    declare get value(): number;
}
```

- invalid member modifier

### declare override is invalid

> Declared members cannot be overrides.

```ds
class Base {
    greet(): void {}
}

class Counter extends Base {
    declare override greet(): void;
}
```

- invalid member modifier

### index signatures cannot use modifiers

> Index signatures cannot use visibility or static modifiers.

```ds
class Counter {
    public [key: string]: int32;
}
```

- invalid member modifier

### static blocks cannot use modifiers

> Static class blocks cannot be modified.

```ds
class Counter {
    public static {
        const value = 1;
        value;
    }
}
```

- static class blocks cannot have any modifier
