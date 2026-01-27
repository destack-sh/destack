# Class Visibility

## private fields

### private fields are inaccessible outside the class

> Private members are only accessible inside the declaring class.

```ds
class Counter {
    private value: int32 = 0

    get(): int32 { this.value }
}

const counter = new Counter();
const out = counter.value;
```

- contains: is private

### private shorthand fields are inaccessible outside the class

> The `#field` shorthand is private.

```ds
class Counter {
    #value: int32 = 0

    get(): int32 { this.#value }
}

const counter = new Counter();
const out = counter.#value;
```

- contains: is private

## protected fields

### protected fields are accessible in subclasses

> Protected members are accessible in subclasses.

```ds
class Base {
    protected value: int32 = 0;
}

class Child extends Base {
    get(): int32 { 
        return this.value;
    }
}
```

### protected fields are inaccessible outside subclasses

> Protected members are not accessible from outside the class hierarchy.

```ds
class Base {
    protected value: int32 = 0;
}

class Child extends Base {
    get(): int32 { 
        return this.value;
    }
}

const child = new Child();
const out = child.value;
```

- contains: is protected
