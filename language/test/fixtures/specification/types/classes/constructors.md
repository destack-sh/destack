# Class Constructors

Constructors allocate and initialize managed class instances.

## initialization

### required fields need initialization

```ds
class Counter {
    value: number;
}
```

- contains: property is not definitely assigned

### constructors initialize required fields

```ds
class Counter {
    value: number;

    constructor(value: number) {
        this.value = value;
    }
}
```

### optional fields do not need initialization

```ds
class Counter {
    value?: number;
}
```

### all constructor paths must initialize fields

```ds
class Counter {
    value: number;

    constructor(flag: boolean) {
        if (flag) {
            this.value = 1;
        }
    }
}
```

- contains: property is not definitely assigned

## overloads

### constructor overloads share one implementation

```ds
class Box {
    value: string | number;

    constructor(value: string);
    constructor(value: number);
    constructor(value: string | number) {
        this.value = value;
    }
}

const fromString = new Box("x");
const fromNumber = new Box(42);

fromString.value satisfies string | number;
fromNumber.value satisfies string | number;
```

### constructor overloads reject multiple implementations

```ds
class Box {
    value: string | number;

    constructor(value: string) {
        this.value = value;
    }

    constructor(value: number) {
        this.value = value;
    }
}
```

- contains: constructor

### constructor overloads require a compatible implementation

```ds
class Box {
    value: string | number;

    constructor(value: string);
    constructor(value: number);
    constructor(value: boolean) {
        this.value = value ? 1 : 0;
    }
}
```

- contains: overload

### constructor calls use overload signatures

```ds
class Box {
    value: string | number;

    constructor(value: string);
    constructor(value: number);
    constructor(value: string | number) {
        this.value = value;
    }
}

new Box(true);
```

- contains: overload

## parameter properties

### parameter properties declare fields

```ds
class Counter {
    constructor(
        public value: int32,
        readonly label: string,
    ) {}
}

const counter = new Counter(1, "label");
counter.value = 2;
counter.label satisfies string;
```

### parameter properties support defaults

```ds
class Counter {
    constructor(public value: int32 = 4) {}
}

const counter = new Counter();
counter.value satisfies int32;
```

### parameter properties reject rest parameters

```ds
class Counter {
    constructor(public ...values: int32[]) {}
}
```

- contains: parameter property

### readonly parameter properties reject writes

```ds
class Counter {
    constructor(readonly label: string) {}

    update() {
        this.label = "next";
    }
}
```

- contains: readonly

### private parameter properties are private

```ds
class Counter {
    constructor(private value: int32) {}
}

const counter = new Counter(1);
counter.value;
```

- contains: is private

### protected parameter properties are inherited

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

### protected parameter properties reject outside access

```ds
class Base {
    constructor(protected value: int32) {}
}

const base = new Base(1);
base.value;
```

- contains: is protected

### private parameter properties affect assignability

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

### protected parameter properties affect assignability

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

### parameter property fields are inherited

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

### readonly parameter properties stay readonly in subclasses

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

### parameter property modifiers only apply to constructors

```ds
class Counter {
    method(public value: int32) {}
}
```

- contains: parameter property
