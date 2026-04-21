# Dynamic Resolution

Tests for dynamic resolution on union receivers.

## union member access

### union member access returns a union type

> Member access on unions yields a union of member types.

```ds
struct User {
    id: int32
}
struct Guest {
    id: string
}

declare function getPerson(): User | Guest;

const id = getPerson().id;
id satisfies int32 | string;
```

### union member access requires members on all variants

> Member access on unions fails if any variant is missing the member.

```ds
struct User {
    name: string
}
struct Guest {
    id: string
}

declare function getPerson(): User | Guest;

getPerson().name;
```

- property 'speak' does not exist on type Cat | Dog

### union member access fails on nullable variants

> Member access on unions fails if any variant lacks the member.

```ds
struct User {
    name: string
}

declare function getPerson(): User | null;

getPerson().name;
```

- property 'speak' does not exist on type Cat | Dog

## union method calls

### union method call returns a union type

> Method calls on unions yield a union of return types.

```ds
struct Cat {
    speak(): string {
        "meow"
    }
}

struct Dog {
    speak(): int32 {
        1
    }
}

declare function getPet(): Cat | Dog;

const sound = getPet().speak();
sound satisfies string | int32;
```

### union method call returns a shared type

> Method calls on unions yield the shared return type when all variants agree.

```ds
struct Cat {
    speak(): string {
        "meow"
    }
}

struct Dog {
    speak(): string {
        "woof"
    }
}

declare function getPet(): Cat | Dog;

const sound = getPet().speak();
sound satisfies string;
```

### union method call rejects incompatible arguments

> Method calls on unions require arguments that satisfy all candidates.

```ds
struct Cat {
    speak(volume: int32): string {
        "meow"
    }
}

struct Dog {
    speak(volume: string): string {
        "woof"
    }
}

declare function getPet(): Cat | Dog;

getPet().speak(1);
```

- contains: no matching overload

### union method call resolves extension members

> Extension methods participate in union member resolution.

```ds
struct Cat { name: string }
struct Dog { name: string }

extension of Cat {
    speak(): string {
        "meow"
    }
}

extension of Dog {
    speak(): string {
        "woof"
    }
}

declare function getPet(): Cat | Dog;

const sound = getPet().speak();
sound satisfies string;
```

### union method call requires members on all variants

> Method calls on unions fail if any variant is missing the member.

```ds
struct Cat {
    speak(): string {
        "meow"
    }
}

struct Dog {
    name: string
}

declare function getPet(): Cat | Dog;

getPet().speak();
```

- contains: does not exist

### union method call fails when a nullable variant is present

> Method calls on unions fail when any variant lacks the member.

```ds
struct Cat {
    speak(): string {
        "meow"
    }
}

declare function getPet(): Cat | null;

getPet().speak();
```

- contains: does not exist

### union method call rejects missing extension member

> Extension members must exist on all union variants.

```ds
struct Cat { name: string }
struct Dog { name: string }

extension of Cat {
    speak(): string {
        "meow"
    }
}

declare function getPet(): Cat | Dog;

getPet().speak();
```

- contains: does not exist
