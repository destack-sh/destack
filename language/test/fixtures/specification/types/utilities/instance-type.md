# InstanceType

`InstanceType` extracts the instance type created by a constructor.

## constructors

### InstanceType extracts class instances

The constructor type yields its instance type.

```ds
class User {
    name: string = "";
}

type Value = InstanceType<typeof User>;

const ok: Value = new User();
ok.name satisfies string;
```

### InstanceType rejects unrelated instances

The extracted type is exact.

```ds
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
