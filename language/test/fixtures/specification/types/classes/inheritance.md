# Class Inheritance

Classes support single inheritance with `extends`.

## inheritance

### children assign to parents

Subclass instances are instances of the parent.

```ds
class Animal {
    name: string = "";
}

class Dog extends Animal {
    breed: string = "";
}

declare function getDog(): Dog;

const animal: Animal = getDog();
```

### final classes construct normally

`final` only forbids extension.

```ds
final class Session {
    id: string = "";
}

const session = new Session();
session satisfies Session;
```

### final classes reject inheritance

A `final` class is a leaf.

```ds
final class Session {
    id: string = "";
}

class DerivedSession extends Session {}
```

- contains: cannot extend final class

### parents do not assign to children

Upcasting does not reverse.

```ds
class Animal {
    name: string = "";
}

class Dog extends Animal {
    breed: string = "";
}

declare function getAnimal(): Animal;

const dog: Dog = getAnimal();
```

- contains: not assignable

### children satisfy parents

`satisfies` follows the same direction.

```ds
class Animal {
    name: string = "";
}

class Dog extends Animal {
    breed: string = "";
}

declare function getDog(): Dog;

getDog() satisfies Animal;
```

### parents do not satisfy children

A parent is not a child.

```ds
class Animal {
    name: string = "";
}

class Dog extends Animal {
    breed: string = "";
}

declare function getAnimal(): Animal;

getAnimal() satisfies Dog;
```

- contains: not assignable

## multi-level inheritance

### grandchildren assign to grandparents

Assignability is transitive.

```ds
class Animal {
    name: string = "";
}

class Dog extends Animal {
    breed: string = "";
}

class Labrador extends Dog {
    color: string = "";
}

declare function getLabrador(): Labrador;

const animal: Animal = getLabrador();
```

### grandchildren assign to parents

Each ancestor accepts the descendant.

```ds
class Animal {
    name: string = "";
}

class Dog extends Animal {
    breed: string = "";
}

class Labrador extends Dog {
    color: string = "";
}

declare function getLabrador(): Labrador;

const dog: Dog = getLabrador();
```

### grandparents do not assign to grandchildren

Transitivity does not reverse.

```ds
class Animal {
    name: string = "";
}

class Dog extends Animal {
    breed: string = "";
}

class Labrador extends Dog {
    color: string = "";
}

declare function getAnimal(): Animal;

const labrador: Labrador = getAnimal();
```

- contains: not assignable

## function parameters

### children pass to parent parameters

Parameter positions accept descendants.

```ds
class Animal {
    name: string = "";
}

class Dog extends Animal {
    breed: string = "";
}

function acceptAnimal(a: Animal): void {}

declare function getDog(): Dog;

acceptAnimal(getDog());
```

### child parameters reject parents

Parameter positions do not accept ancestors.

```ds
class Animal {
    name: string = "";
}

class Dog extends Animal {
    breed: string = "";
}

function acceptDog(d: Dog): void {}

declare function getAnimal(): Animal;

acceptDog(getAnimal());
```

- contains: not assignable

## sibling classes

### siblings do not assign to each other

Sharing a parent relates neither sibling.

```ds
class Animal {
    name: string = "";
}

class Dog extends Animal {
    breed: string = "";
}

class Cat extends Animal {
    whiskers: number = 0;
}

declare function getDog(): Dog;

const cat: Cat = getDog();
```

- contains: not assignable
