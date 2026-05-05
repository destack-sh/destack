# Readonly

`Readonly<T>` is the named utility alias for deep `readonly T`.

### readonly keeps field types

```ds
interface Person {
    name: string
    age: number
}

type Frozen = Readonly<Person>;

const ok: Frozen = { name: "Ada", age: 42 };
ok satisfies Frozen;
```

### readonly preserves optional fields

```ds
interface Person {
    name?: string
}

type Frozen = Readonly<Person>;

const ok: Frozen = {};
ok satisfies Frozen;
```

### readonly rejects property writes

```ds
interface Person {
    name: string
    age: number
}

type Frozen = Readonly<Person>;

const frozen: Frozen = { name: "Ada", age: 42 };
frozen.name = "Grace";
```

- contains: read-only

### readonly rejects nested property writes

```ds
interface Person {
    profile: {
        name: string
    }
}

type Frozen = Readonly<Person>;

const frozen: Frozen = { profile: { name: "Ada" } };
frozen.profile.name = "Grace";
```

- contains: read-only
