# Readonly

`Readonly` is a standard TypeScript utility type.

## cases

### readonly keeps field types

> `Readonly` preserves the field types.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type Frozen = Readonly<Person>;

const ok: Frozen = { name: "Ada", age: 42 };
ok satisfies Frozen;
```

### readonly preserves optional fields

> `Readonly` keeps optional fields optional.

```ds libs=es5
interface Person {
    name?: string
}

type Frozen = Readonly<Person>;

const ok: Frozen = {};
ok satisfies Frozen;
```

### readonly rejects property writes

> Readonly fields cannot be assigned through the readonly type.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type Frozen = Readonly<Person>;

const frozen: Frozen = { name: "Ada", age: 42 };
frozen.name = "Grace";
```

- contains: read-only
