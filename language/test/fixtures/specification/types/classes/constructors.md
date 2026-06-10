# Class Constructors

Constructors allocate and initialize managed class instances.

## initialization

### required fields need initialization

Fields must be definitely assigned.

```ds
class Counter {
    value: number;
}
```

- contains: property is not definitely assigned

### constructors initialize required fields

Constructor assignment satisfies the requirement.

```ds
class Counter {
    value: number;

    constructor(value: number) {
        this.value = value;
    }
}
```

### optional fields do not need initialization

Optional means possibly absent.

```ds
class Counter {
    value?: number;
}
```

### all constructor paths must initialize fields

Every path through the constructor must assign.

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

Overload signatures sit above one body.

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

Only one body is allowed.

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

The body must serve every signature.

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

Calls resolve against the signatures, not the body.

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
