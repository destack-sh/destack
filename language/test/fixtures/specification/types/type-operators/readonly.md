# Readonly Types

`readonly T` is a deep read-only view of `T`.

## objects

### readonly objects reject field writes

```ds
type User = {
    name: string;
};

declare const user: readonly User;
user.name = "Grace";
```

- contains: readonly

### readonly objects reject nested field writes

```ds
type User = {
    profile: {
        name: string;
    };
};

declare const user: readonly User;
user.profile.name = "Grace";
```

- contains: readonly

### readonly objects can be read

```ds
type User = {
    profile: {
        name: string;
    };
};

declare const user: readonly User;
user.profile.name satisfies string;
```

## structs

### readonly structs reject field writes

```ds
struct User {
    name: string;
}

declare const user: readonly User;
user.name = "Grace";
```

- contains: readonly

### readonly structs reject nested field writes

```ds
struct Profile {
    name: string;
}

struct User {
    profile: Profile;
}

declare const user: readonly User;
user.profile.name = "Grace";
```

- contains: readonly

## arrays

### readonly arrays accept mutable arrays

```ds
declare let values: number[];
let frozen: readonly number[] = values;
frozen satisfies readonly number[];
```

### readonly arrays reject mutable assignment

```ds
declare let frozen: readonly number[];
let bad: number[] = frozen;
```

- contains: not assignable

### readonly arrays reject index writes

```ds
declare const values: readonly number[];
values[0] = 1;
```

- contains: readonly

### readonly arrays reject nested element writes

```ds
type User = {
    name: string;
};

declare const users: readonly User[];
users[0].name = "Grace";
```

- contains: readonly

### readonly arrays reject mutation methods

```ds
declare const values: readonly number[];
values.push(1);
```

- contains: push

## slices

### readonly slices reject index writes

```ds
declare const values: readonly [number];
values[0] = 1;
```

- contains: readonly

### readonly slices reject nested element writes

```ds
struct User {
    name: string;
}

declare const users: readonly [User];
users[0].name = "Grace";
```

- contains: readonly

## fixed arrays

### readonly fixed arrays reject index writes

```ds
declare const values: readonly [number; 3];
values[0] = 1;
```

- contains: readonly

### readonly fixed arrays reject nested element writes

```ds
struct User {
    name: string;
}

declare const users: readonly [User; 2];
users[0].name = "Grace";
```

- contains: readonly

## tuples

### readonly tuples accept mutable tuples

```ds
type Pair = (number, string);
type ReadonlyPair = readonly (number, string);

declare let pair: Pair;
let frozen: ReadonlyPair = pair;
frozen satisfies ReadonlyPair;
```

### readonly tuples reject mutable assignment

```ds
type Pair = (number, string);
type ReadonlyPair = readonly (number, string);

declare let frozen: ReadonlyPair;
let bad: Pair = frozen;
```

- contains: not assignable

### readonly tuples reject element writes

```ds
declare const pair: readonly (number, string);
pair[0] = 1;
```

- contains: readonly

### readonly tuples reject nested element writes

```ds
type User = {
    name: string;
};

declare const pair: readonly (User, number);
pair[0].name = "Grace";
```

- contains: readonly
