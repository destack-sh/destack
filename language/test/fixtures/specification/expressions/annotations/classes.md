# Class Annotations

Class annotations are resolved expressions on class and member declarations.

## metadata

### class metadata preserves the constructor type

> Metadata on a class does not change the class constructor or instance shape.

```ds
newtype Label = { value: string };

function label(value: string): Label {
    Label({ value })
}

@label("user")
class User {
    name: string = "";
}

declare const user: User;
user.name satisfies string;

const factory = User;
factory satisfies typeof User;
```

### member metadata preserves member types

> Metadata on fields and methods does not wrap or replace the member.

```ds
newtype Label = { value: string };

function label(value: string): Label {
    Label({ value })
}

class User {
    @label("stored")
    name: string = "";

    @label("display")
    display(): string {
        return this.name;
    }
}

declare const user: User;

user.name satisfies string;
user.display() satisfies string;
```

### static member metadata preserves static member types

> Metadata on static members does not rewrite the static side.

```ds
newtype Label = { value: string };

function label(value: string): Label {
    Label({ value })
}

class User {
    constructor(public name: string) {}

    @label("factory")
    static create(name: string): User {
        return new User(name);
    }
}

const user = User.create("Ada");
user.name satisfies string;
```

## resolution

### unresolved class annotations are rejected

> The expression after `@` must resolve.

```ds
@missing
class User {}
```

- contains: missing

## rejections

### conflicting member hints are rejected

> One target cannot carry contradictory branch hints.

```ds
class User {
    @likely
    @unlikely
    name: string = "";
}
```

- contains: likely and unlikely
