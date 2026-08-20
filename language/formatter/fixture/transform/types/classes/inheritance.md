# Class Inheritance

## Inheritance

### class extends

Subclasses use `extends` to inherit from a base class.

```ds
class Dog extends Animal { bark() { } }
```

```ds expected
class Dog extends Animal {
    bark() {}
}
```

### class implements

Classes use `implements` to satisfy interface contracts.

```ds
class Dog implements Animal { makeSound() { } }
```

```ds expected
class Dog implements Animal {
    makeSound() {}
}
```

### class extends and implements

A class can both extend a base class and implement interfaces.

```ds
class Dog extends Pet implements Animal, Named { name: string }
```

```ds expected
class Dog extends Pet implements Animal, Named {
    name: string;
}
```

### class with super call

Simple single-statement constructor bodies stay on one line.

```ds
class Dog extends Animal { constructor() { super() } }
```

```ds expected
class Dog extends Animal {
    constructor() {
        super();
    }
}
```
