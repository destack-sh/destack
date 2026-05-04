# Member Access

## unions

### union member access yields common type

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

> Member access on unions uses type aliases.

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

### union member access rejects missing aliased members

> Member access on unions is rejected if any aliased variant is missing the member.

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

### union member access rejects nullable aliased members

> Member access on unions is rejected when an aliased variant lacks the member.

```ds
struct User {
    name: string
}

type Person = User | null;

declare function getPerson(): Person;

getPerson().name;
```

- contains: does not exist

### union member access preserves optional member types

> Member access on unions preserves optional member unions when all variants define the member.

```ds
struct User {
    nickname?: string
}

struct Admin {
    nickname?: string
}

declare function getPerson(): User | Admin;

const nickname = getPerson().nickname;
nickname satisfies string | undefined;
```

### union member access rejects properties missing from one variant

> Member access on unions rejects properties absent from any union variant.

```ds
struct User {
    profile: { displayName: string }
}

struct Guest {
    profile: { id: int32 }
}

declare function getPerson(): User | Guest;

getPerson().profile.displayName;
```

- contains: property 'displayName' does not exist on type { displayName: string } | { id: int32 }
