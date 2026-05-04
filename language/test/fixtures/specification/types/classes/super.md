# Super

`super` constructor and member use in derived classes.

## constructor calls

### super call accepts base constructor parameters

> A derived constructor can call the base constructor with matching arguments.

```ds
class Base {
    constructor(value: int32) {}
}

class Derived extends Base {
    constructor() {
        super(1);
    }
}

new Derived();
```

### super call accepts implicit base constructors

> A derived constructor can call super when the base constructor is implicit.

```ds
class Base {}

class Derived extends Base {
    constructor() {
        super();
    }
}

new Derived();
```

### super call validates constructor overloads

> A derived constructor must match the base constructor signature.

```ds
class Base {
    constructor(value: int32) {}
}

class Derived extends Base {
    constructor() {
        super("invalid");
    }
}
```

- contains: not assignable

### super call is rejected outside constructors

> Super constructor calls are only allowed in derived constructors.

```ds
class Base {}

class Derived extends Base {
    update() {
        super();
    }
}
```

- contains: super calls are only valid in constructors of derived classes

### super call is rejected in non derived constructors

> Super constructor calls are rejected when the class has no base type.

```ds
class Base {
    constructor() {
        super();
    }
}
```

- contains: super calls are only valid in constructors of derived classes
- contains: calling non-callable

### super call is rejected in nested constructor functions

> Super constructor calls are rejected inside nested functions, even in derived constructors.

```ds
class Base {
    constructor() {}
}

class Derived extends Base {
    constructor() {
        let invoke = () => {
            super();
        };

        invoke();
    }
}
```

- contains: super calls are only valid in constructors of derived classes

### super call is rejected in static methods

> Super constructor calls are rejected in static methods.

```ds
class Base {
    constructor() {}
}

class Derived extends Base {
    static boot() {
        super();
    }
}
```

- contains: super calls are only valid in constructors of derived classes

## member access

### super optional chaining is rejected

> Optional chaining cannot target a super reference.

```ds
class Base {
    value(): int32 {
        1
    }
}

class Derived extends Base {
    inspectBase(): int32 {
        super?.value()
    }
}
```

- contains: optional chaining cannot be applied to super

### super optional chaining is rejected through members

> Optional chaining is rejected when any optional segment is rooted in super.

```ds
class Base {
    value(): int32 {
        1
    }
}

class Derived extends Base {
    inspectBase(): int32 | undefined {
        super.value?.()
    }
}
```

- contains: optional chaining cannot be applied to super

### super member calls resolve the base implementation

> Super member dispatch resolves members on the base class.

```ds
class Base {
    label(): "base" {
        "base"
    }
}

class Derived extends Base {
    fromSuper(): "base" {
        super.label()
    }
}
```
