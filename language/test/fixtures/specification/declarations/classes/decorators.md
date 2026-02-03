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
