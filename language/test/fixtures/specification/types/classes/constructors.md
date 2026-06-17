# Class Constructors

Constructors allocate and initialize managed class instances.

## initialization

### classes can use implicit constructors

A class without an explicit constructor has an implicit constructor.
Base classes get an empty constructor, and derived classes forward to the base constructor.

```ds
class Counter {
    value: number = 0;
}

class Session {
    id: number;

    constructor(id: number) {
        this.id = id;
    }
}

class UserSession extends Session {}

const counter = new Counter();
const session = new UserSession(1);
```

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

### constructor overloads use real implementations

Each overload has its own body.

```ds
class Box {
    value: string | number;

    constructor(value: string) {
        this.value = value as string | number;
    }

    constructor(value: number) {
        this.value = value as string | number;
    }
}

const fromString = new Box("x");
const fromNumber = new Box(42);

fromString.value satisfies string | number;
fromNumber.value satisfies string | number;
```

### constructor declarations need declaration context

Bodyless constructor declarations belong in declaration contexts.

```ds
declare class Box {
    value: string | number;

    constructor(value: string);
    constructor(value: number);
}
```

### concrete constructors require bodies

Concrete classes cannot use TypeScript-style hidden implementation signatures.

```ds
class Box {
    value: string | number;

    constructor(value: string);
    constructor(value: number);
}
```

- contains: constructor

### constructor calls use overload implementations

Calls resolve against the overload bodies.

```ds
class Box {
    value: string | number;

    constructor(value: string) {
        this.value = value as string | number;
    }

    constructor(value: number) {
        this.value = value as string | number;
    }
}

new Box(true);
```

- contains: overload
