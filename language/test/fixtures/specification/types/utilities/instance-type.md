# InstanceType

`InstanceType` extracts the instance type created by a constructor.

### InstanceType extracts class instances

```ts libs=es5
class User {
    name: string = "";
}

type Value = InstanceType<typeof User>;

const ok: Value = new User();
ok.name satisfies string;
```

### InstanceType rejects unrelated instances

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
