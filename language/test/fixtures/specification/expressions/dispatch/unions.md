# Union Dispatch

## fields

### member access returns the shared field type

Union member access returns a single type when all variants agree.

```ds
struct User {
    name: string;
}

struct Admin {
    name: string;
}

declare function getPerson(): User | Admin;

const name = getPerson().name;
name satisfies string;
```

### aliases preserve union member access

Type aliases do not change union member access.

```ds
struct User {
    id: int32;
}

struct Guest {
    id: string;
}

type Person = User | Guest;

declare function getPerson(): Person;

const id = getPerson().id;
id satisfies int32 | string;
```

### missing members are rejected

Every union variant must expose the accessed member.

```ds
struct User {
    name: string;
}

struct Guest {
    id: string;
}

type Person = User | Guest;

declare function getPerson(): Person;

getPerson().name;
```

- contains: does not exist

### nullable variants reject member access

Nullable variants do not expose ordinary members.

```ds
struct User {
    name: string;
}

type Person = User | null;

declare function getPerson(): Person;

getPerson().name;
```

- contains: does not exist

### optional members stay optional

Optional members keep `undefined` in the result type.

```ds
struct User {
    nickname?: string;
}

struct Admin {
    nickname?: string;
}

declare function getPerson(): User | Admin;

const nickname = getPerson().nickname;
nickname satisfies string | undefined;
```

### nested access still checks every variant

Nested member access is rejected when a nested variant is missing the member.

```ds
struct User {
    profile: { displayName: string };
}

struct Guest {
    profile: { id: int32 };
}

declare function getPerson(): User | Guest;

getPerson().profile.displayName;
```

- contains: property 'displayName' does not exist on type { displayName: string } | { id: int32 }

## methods

### union method calls select compatible overloads

Overload selection uses signatures compatible with the argument.

```ds
struct Cat {
    speak(volume: string): string {
        volume
    }

    speak(volume: int32): int32 {
        volume
    }
}

struct Dog {
    speak(volume: string): int32 {
        1
    }

    speak(volume: int32): string {
        "woof"
    }
}

declare function getPet(): Cat | Dog;

const sound = getPet().speak("loud");
sound satisfies string | int32;
```

### union method calls honor overload order

Overloads resolve in declaration order for each union variant.

```ds
struct Cat {
    speak(volume: string): string {
        volume
    }

    speak(volume: "loud"): int32 {
        1
    }
}

struct Dog {
    speak(volume: string): string {
        volume
    }
}

declare function getPet(): Cat | Dog;

const sound = getPet().speak("loud");
sound satisfies string;
```

### union method calls include extension members

Member lookup accounts for inherent and extension members together.

```ds
struct Cat {
    speak(): string {
        "meow"
    }
}

struct Dog {
    name: string;
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

### union method calls reject unmatched union arguments

Union arguments must match a single compatible overload.

```ds
struct Cat {
    speak(volume: string): string {
        volume
    }

    speak(volume: int32): int32 {
        volume
    }
}

struct Dog {
    speak(volume: string): string {
        "woof"
    }

    speak(volume: int32): string {
        "woof"
    }
}

declare function getPet(): Cat | Dog;

const volume: string | int32 = "loud";

getPet().speak(volume);
```

- contains: no matching overload

### union method calls do not distribute arguments

Union arguments must be accepted by a single overload.

```ds
struct Cat {
    speak(volume: string): string {
        volume
    }

    speak(volume: int32): string {
        "meow"
    }
}

struct Dog {
    speak(volume: string): int32 {
        1
    }

    speak(volume: int32): int32 {
        2
    }
}

declare function getPet(): Cat | Dog;

const volume: string | int32 = "loud";

getPet().speak(volume);
```

- contains: no matching overload

### union method calls accept union overloads

Union arguments are allowed when an overload accepts the union.

```ds
struct Cat {
    speak(volume: string | int32): string {
        "meow"
    }
}

struct Dog {
    speak(volume: string | int32): int32 {
        1
    }
}

declare function getPet(): Cat | Dog;

const volume: string | int32 = "loud";

const sound = getPet().speak(volume);
sound satisfies string | int32;
```

### extension methods return unions

Extension overloads contribute their selected return types.

```ds
struct Cat {
    name: string;
}
struct Dog {
    name: string;
}

extension of Cat {
    speak(): string {
        "meow"
    }
}

extension of Dog {
    speak(): int32 {
        1
    }
}

declare function getPet(): Cat | Dog;

const sound = getPet().speak();
sound satisfies string | int32;
```
