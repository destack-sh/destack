# InstanceType

`InstanceType` extracts the instance type created by a constructor.

## cases

### instancetype extracts class instances

> Class constructors resolve to their instance type.

```ts libs=es5
class User {
    name: string = "";
}

type Value = InstanceType<typeof User>;

const ok: Value = new User();
ok.name satisfies string;
```

### instancetype rejects unrelated instances

> Extracted instance types keep the class identity.

```ts libs=es5
class User {
    name: string = "";
}

class Project {
    title: string = "";
}

type Value = InstanceType<typeof User>;

const bad: Value = new Project();
```

- contains: not assignable
