# Super

`super` constructor and member use in derived classes.

## constructor calls

### super calls accept base constructor parameters

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

### super calls accept implicit base constructors

```ds
class Base {}

class Derived extends Base {
    constructor() {
        super();
    }
}

new Derived();
```

### super calls check constructor parameters

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

### super calls reject non-constructors

```ds
class Base {}

class Derived extends Base {
    update() {
        super();
    }
}
```

- contains: super calls are only valid in constructors of derived classes

### super calls require base classes

```ds
class Base {
    constructor() {
        super();
    }
}
```

- contains: super calls are only valid in constructors of derived classes
- contains: calling non-callable

### super calls reject nested functions

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

### super calls reject static methods

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

### super rejects optional chaining

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

### super rejects optional member calls

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

### super member calls use base implementations

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
