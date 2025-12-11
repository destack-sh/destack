# Class Inheritance (extends)

Tests for class inheritance using `extends`.

## Basic Inheritance

### child assignable to parent

> A child class instance is assignable to a parent class type.

```ds
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}

declare function getDog(): Dog;

const animal: Animal = getDog();
```

### parent not assignable to child

> A parent class instance is not assignable to a child class type.

```ds
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}

declare function getAnimal(): Animal;

const dog: Dog = getAnimal();
```

- type Animal is not assignable to type Dog

### child satisfies parent

> A child class satisfies the parent class type.

```ds
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}

declare function getDog(): Dog;

getDog() satisfies Animal;
```

### parent does not satisfy child

> A parent class does not satisfy a child class type.

```ds
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}

declare function getAnimal(): Animal;

getAnimal() satisfies Dog;
```

- expected Dog, found Animal

## Multi-level Inheritance

### grandchild assignable to grandparent

> A grandchild class is assignable to a grandparent class.

```ds
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}

class Labrador extends Dog {
    color: string
}

declare function getLabrador(): Labrador;

const animal: Animal = getLabrador();
```

### grandchild assignable to parent

> A grandchild class is assignable to its direct parent.

```ds
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}

class Labrador extends Dog {
    color: string
}

declare function getLabrador(): Labrador;

const dog: Dog = getLabrador();
```

### grandparent not assignable to grandchild

> A grandparent class is not assignable to a grandchild class.

```ds
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}

class Labrador extends Dog {
    color: string
}

declare function getAnimal(): Animal;

const labrador: Labrador = getAnimal();
```

- type Animal is not assignable to type Labrador

## Function Parameters

### child passed to parent parameter

> A child class can be passed where a parent class is expected.

```ds
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}

function acceptAnimal(a: Animal): void {}

declare function getDog(): Dog;

acceptAnimal(getDog());
```

### parent rejected for child parameter

> A parent class cannot be passed where a child class is expected.

```ds
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}

function acceptDog(d: Dog): void {}

declare function getAnimal(): Animal;

acceptDog(getAnimal());
```

- type Animal is not assignable to type Dog

## Sibling Classes

### sibling not assignable

> Sibling classes (same parent) are not assignable to each other.

```ds
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}

class Cat extends Animal {
    whiskers: number
}

declare function getDog(): Dog;

const cat: Cat = getDog();
```

- type Dog is not assignable to type Cat
