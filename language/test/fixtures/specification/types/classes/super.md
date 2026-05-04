# Super Inheritance Semantics

`super` constructor and member behavior in derived classes.

## Constructor Calls

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

> Super constructor calls are only valid in derived constructors.

```ds
class Base {}

class Derived extends Base {
    update() {
        super();
    }
}
```

- super calls are only valid in constructors of derived classes

### super call is rejected in non derived constructors

> Super constructor calls are invalid when the class has no base type.

```ds
class Base {
    constructor() {
        super();
    }
}
```

- super calls are only valid in constructors of derived classes
- calling non-callable

### super call is rejected in nested constructor functions

> Super constructor calls are invalid inside nested functions, even in derived constructors.

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

- super calls are only valid in constructors of derived classes

### super call is rejected in static methods

> Super constructor calls are invalid in static methods.

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

- super calls are only valid in constructors of derived classes

## Member Access

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

- optional chaining cannot be applied to super

### super optional chaining is rejected through members

> Optional chaining is invalid when any optional segment is rooted in super.

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

- optional chaining cannot be applied to super

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
