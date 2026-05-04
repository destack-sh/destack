# Calls

## union methods

### union method call selects compatible overloads

> Overload selection uses signatures compatible with the argument.

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

### union method call honors overload declaration order

> Overloads resolve in declaration order for each union variant.

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

### union method call resolves inherent and extension members

> Member lookup accounts for inherent and extension members together.

```ds
struct Cat {
    speak(): string {
        "meow"
    }
}

struct Dog {
    name: string
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

### union method call rejects union arguments without compatible overload

> Union arguments must match a single compatible overload.

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

### union method call rejects union arguments with only per-overload coverage

> Union arguments must be accepted by a single overload.

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

### union method call accepts union argument with union overload

> Union arguments are allowed when an overload accepts the union.

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

### union method call returns union for extension overloads

> Extension overloads participate in dynamic call resolution.

```ds
struct Cat { name: string }
struct Dog { name: string }

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
