# Class Inheritance

Classes support single inheritance with `extends`.

## inheritance

### children assign to parents

```ds
class Animal {
    name: string = ""
}

class Dog extends Animal {
    breed: string = ""
}

declare function getDog(): Dog;

const animal: Animal = getDog();
```

### parents do not assign to children

```ds
class Animal {
    name: string = ""
}

class Dog extends Animal {
    breed: string = ""
}

declare function getAnimal(): Animal;

const dog: Dog = getAnimal();
```

- contains: not assignable

### children satisfy parents

```ds
class Animal {
    name: string = ""
}

class Dog extends Animal {
    breed: string = ""
}

declare function getDog(): Dog;

getDog() satisfies Animal;
```

### parents do not satisfy children

```ds
class Animal {
    name: string = ""
}

class Dog extends Animal {
    breed: string = ""
}

declare function getAnimal(): Animal;

getAnimal() satisfies Dog;
```

- contains: not assignable

## multi-level inheritance

### grandchildren assign to grandparents

```ds
class Animal {
    name: string = ""
}

class Dog extends Animal {
    breed: string = ""
}

class Labrador extends Dog {
    color: string = ""
}

declare function getLabrador(): Labrador;

const animal: Animal = getLabrador();
```

### grandchildren assign to parents

```ds
class Animal {
    name: string = ""
}

class Dog extends Animal {
    breed: string = ""
}

class Labrador extends Dog {
    color: string = ""
}

declare function getLabrador(): Labrador;

const dog: Dog = getLabrador();
```

### grandparents do not assign to grandchildren

```ds
class Animal {
    name: string = ""
}

class Dog extends Animal {
    breed: string = ""
}

class Labrador extends Dog {
    color: string = ""
}

declare function getAnimal(): Animal;

const labrador: Labrador = getAnimal();
```

- contains: not assignable

## function parameters

### children pass to parent parameters

```ds
class Animal {
    name: string = ""
}

class Dog extends Animal {
    breed: string = ""
}

function acceptAnimal(a: Animal): void {}

declare function getDog(): Dog;

acceptAnimal(getDog());
```

### child parameters reject parents

```ds
class Animal {
    name: string = ""
}

class Dog extends Animal {
    breed: string = ""
}

function acceptDog(d: Dog): void {}

declare function getAnimal(): Animal;

acceptDog(getAnimal());
```

- contains: not assignable

## sibling classes

### siblings do not assign to each other

```ds
class Animal {
    name: string = ""
}

class Dog extends Animal {
    breed: string = ""
}

class Cat extends Animal {
    whiskers: number = 0
}

declare function getDog(): Dog;

const cat: Cat = getDog();
```

- contains: not assignable

## rejections

### classes reject multiple parents

```ds
class First {}
class Second {}

class Combined extends First, Second {}
```

- contains: invalid lineage

### classes reject empty extends clauses

```ds
class Counter extends {
}
```

- contains: invalid lineage

### structs reject extends

```ds
struct Base {}

struct Counter extends Base {}
```

- contains: invalid lineage
