# Readonly

`Readonly<T>` is the standard mapped utility for shallow readonly fields.

## properties

### readonly keeps field types

Only mutability changes.

```ds
interface Person {
    name: string;
    age: number;
}

type Frozen = Readonly<Person>;

const ok: Frozen = { name: "Ada", age: 42 };
ok satisfies Frozen;
```

### readonly preserves optional fields

Optionality survives.

```ds
interface Person {
    name?: string;
}

type Frozen = Readonly<Person>;

const ok: Frozen = {};
ok satisfies Frozen;
```

### readonly rejects property writes

The write surface is gone.

```ds
interface Person {
    name: string;
    age: number;
}

type Frozen = Readonly<Person>;

const frozen: Frozen = { name: "Ada", age: 42 };
frozen.name = "Grace";
```

- contains: readonly

### readonly keeps nested object fields mutable

The mapped utility follows TypeScript and only changes immediate fields.

```ds
interface Person {
    profile: {
        name: string;
    };
}

type Frozen = Readonly<Person>;

const frozen: Frozen = { profile: { name: "Ada" } };
frozen.profile.name = "Grace";
frozen.profile.name satisfies string;
```
