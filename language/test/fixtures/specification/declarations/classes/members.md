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

> Static methods are not accessible from instances.-æ

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

## invalid members

### abstract fields cannot have initializers

> Abstract fields are declarations only and cannot include initializers.

```ds
abstract class Counter {
    abstract value: int32 = 1;
}
```

- contains: invalid member modifier

### readonly does not apply to methods

> Readonly modifiers are only valid on fields.

```ds
class Counter {
    readonly increment(): int32 {
        1
    }
}
```

- contains: invalid member modifier

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

- contains: invalid member modifier

### override constructors are invalid

> Constructors cannot use override modifiers.

```ds
class Base {}

class Counter extends Base {
    override constructor() {}
}
```

- contains: invalid constructor

### abstract methods cannot have bodies

> Abstract methods are declarations only.

```ds
abstract class Counter {
    abstract increment(): void {}
}
```

- contains: invalid abstract method

### abstract methods require abstract classes

> Abstract methods can only appear in abstract classes.

```json:dsconfig.json
{ "compilerOptions": { "strictPropertyInitialization": false } }
```

```ds
class Counter {
    abstract increment(): void;
}
```

- contains: invalid abstract method

### static abstract methods are invalid

> Static methods cannot be abstract.

```ds
abstract class Counter {
    static abstract increment(): void;
}
```

- contains: invalid member modifier

### constructors cannot have static parameters

> Constructors cannot declare static parameters.

```ds
class Counter {
    constructor<T>(value: T) {}
}
```

- contains: invalid constructor

### declare methods cannot have bodies

> Declared methods cannot include bodies.

```ds
class Counter {
    declare increment(): void {}
}
```

- contains: invalid member modifier

### declare accessors are invalid

> Declared members cannot use accessors.

```ds
class Counter {
    declare get value(): number;
}
```

- contains: invalid member modifier

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

- contains: invalid member modifier

### abstract fields require abstract classes

> Abstract fields only appear in abstract classes.

```json:dsconfig.json
{ "compilerOptions": { "strictPropertyInitialization": false } }
```

```ds
class Counter {
    abstract value: int32;
}
```

- contains: invalid member modifier

### private abstract fields are invalid

> Private fields cannot be abstract.

```json:dsconfig.json
{ "compilerOptions": { "strictPropertyInitialization": false } }
```

```ds
abstract class Counter {
    abstract #value: int32;
}
```

- contains: invalid member modifier

### auto accessors cannot be combined with readonly

> Auto accessors cannot combine with readonly modifiers.

```json:dsconfig.json
{ "compilerOptions": { "strictPropertyInitialization": false } }
```

```ds
class Counter {
    accessor readonly value: int32;
}
```

- contains: invalid member modifier

### declare abstract fields are invalid

> Declared fields cannot be abstract.

```json:dsconfig.json
{ "compilerOptions": { "strictPropertyInitialization": false } }
```

```ds
abstract class Counter {
    declare abstract value: int32;
}
```

- contains: invalid member modifier

### index signatures cannot use modifiers

> Index signatures cannot use visibility or static modifiers.

```ds
class Counter {
    public [key: string]: int32;
}
```

- contains: invalid member modifier

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

- contains: static class blocks cannot have any modifier
