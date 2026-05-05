# Readonly

`Readonly` is a standard utility type.

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

### readonly is shallow

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
