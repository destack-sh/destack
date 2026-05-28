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
