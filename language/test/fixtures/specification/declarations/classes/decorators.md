# Class Decorators

Tests for decorators on classes and members.

## class decorators

### class decorator allows declaration

> Class decorators are accepted on class declarations.

```ds
@deprecated("use new")
class User {
    name: string = "";
}

declare const user: User;
user.name satisfies string;
```

## member decorators

### property decorator allows declaration

> Property decorators are accepted on class fields.

```ds
class User {
    @deprecated
    name: string = "";
}

declare const user: User;
user.name satisfies string;
```

### method decorator allows declaration

> Method decorators are accepted on class methods.

```ds
class User {
    @deprecated
    greet(): string {
        return "hi";
    }
}

declare const user: User;
user.greet() satisfies string;
```

### static method decorator allows declaration

> Decorators are accepted on static class methods.

```ds
class User {
    @deprecated
    static create(): User {
        User { name: "Ada" }
    }

    name: string = "";
}

const user = User.create();
user.name satisfies string;
```

### class field decorators reject conflicting likely and unlikely

> likely and unlikely decorators cannot be combined on class fields.

```ds
class User {
    @likely
    @unlikely
    name: string = "";
}
```

- invalid well-known decorator: likely and unlikely decorators cannot be combined