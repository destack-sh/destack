# Call Resolution

## union method call selects per-variant overloads

> Overload selection happens per union variant.

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

## union method call honors overload declaration order

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

## union method call resolves inherent and extension members

> Dynamic resolution accounts for inherent and extension members together.

```ds
struct Cat {
    speak(): string {
        "meow"
    }
}

struct Dog {
    name: string
}

extension for Dog {
    speak(): string {
        "woof"
    }
}

declare function getPet(): Cat | Dog;

const sound = getPet().speak();
sound satisfies string;
```

## union method call rejects union argument without matching overload

> Calls require a matching overload for every union variant.

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

## union method call accepts union arguments with full coverage

> Union arguments are valid when every variant has matching overloads.

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

const sound = getPet().speak(volume);
sound satisfies string | int32;
```

## union method call returns union for extension overloads

> Extension overloads participate in dynamic call resolution.

```ds
struct Cat { name: string }
struct Dog { name: string }

extension for Cat {
    speak(): string {
        "meow"
    }
}

extension for Dog {
    speak(): int32 {
        1
    }
}

declare function getPet(): Cat | Dog;

const sound = getPet().speak();
sound satisfies string | int32;
```
