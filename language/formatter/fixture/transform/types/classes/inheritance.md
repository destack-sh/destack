# Class Inheritance

## Inheritance

### class extends

Subclasses use `extends` to inherit from a base class.

```tspp
class Dog extends Animal { bark() { } }
```

```tspp expected
class Dog extends Animal {
    bark() {}
}
```

### class implements

Classes use `implements` to satisfy interface contracts.

```tspp
class Dog implements Animal { makeSound() { } }
```

```tspp expected
class Dog implements Animal {
    makeSound() {}
}
```

### class extends and implements

A class can both extend a base class and implement interfaces.

```tspp
class Dog extends Pet implements Animal, Named { name: string }
```

```tspp expected
class Dog extends Pet implements Animal, Named {
    name: string;
}
```

### class with super call

Simple single-statement constructor bodies stay on one line.

```tspp
class Dog extends Animal { constructor() { super() } }
```

```tspp expected
class Dog extends Animal {
    constructor() {
        super();
    }
}
```
