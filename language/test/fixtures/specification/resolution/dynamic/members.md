# Member Resolution

## tests

### union member access yields shared type

> Member access on unions yields a single type when all variants agree.

```ds
struct User {
    name: string
}

struct Admin {
    name: string
}

declare function getPerson(): User | Admin;

const name = getPerson().name;
name satisfies string;
```

### union member access resolves through aliases

> Member access on unions works through type aliases.

```ds
struct User {
    id: int32
}

struct Guest {
    id: string
}

type Person = User | Guest;

declare function getPerson(): Person;

const id = getPerson().id;
id satisfies int32 | string;
```

### union member access fails through aliases

> Member access on unions fails if any aliased variant is missing the member.

```ds
struct User {
    name: string
}

struct Guest {
    id: string
}

type Person = User | Guest;

declare function getPerson(): Person;

getPerson().name;
```

- contains: does not exist

### union member access fails through nullable aliases

> Member access on unions fails when an aliased variant lacks the member.

```ds
struct User {
    name: string
}

type Person = User | null;

declare function getPerson(): Person;

getPerson().name;
```

- contains: does not exist
