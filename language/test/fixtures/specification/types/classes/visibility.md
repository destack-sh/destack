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

### private shorthand fields are accessible within the class

> Private shorthand fields are accessible inside the declaring class.

```ds
class Counter {
    #value: int32 = 0

    get(): int32 { this.#value }
}

const counter = new Counter();
const out = counter.get();
```

### private shorthand fields are inaccessible in subclasses

> Private shorthand fields are not accessible from subclasses.

```ds
class Base {
    #value: int32 = 0
}

class Child extends Base {
    get(): int32 { this.#value }
}
```

- contains: is private

### private shorthand methods are accessible within the class

> Private shorthand methods are callable inside the declaring class.

```ds
class Counter {
    #next(): int32 { 1 }

    get(): int32 { this.#next() }
}

const counter = new Counter();
const out = counter.get();
```

### private shorthand methods are inaccessible outside the class

> Private shorthand methods are not callable outside the declaring class.

```ds
class Counter {
    #next(): int32 { 1 }
}

const counter = new Counter();
const out = counter.#next();
```

- contains: is private

### static private fields are accessible within the class

> Static private fields are accessible inside the declaring class.

```ds
class Counter {
    static #value: int32 = 1;

    static get(): int32 {
        Counter.#value
    }
}

Counter.get() satisfies int32;
```

### static private fields are inaccessible outside the class

> Static private fields are not accessible from the outside.

```ds
class Counter {
    static #value: int32 = 1;
}

Counter.#value;
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

- '<anonymous>' is protected
