# Readonly Types

`readonly T` is a deep read-only view of `T`.

## objects

### readonly objects reject field writes

> Readonly object fields cannot be assigned through a readonly view.

```ds
type User = {
    name: string;
};

declare const user: readonly User;
user.name = "Grace";
```

- contains: readonly

### readonly objects reject nested field writes

> Readonly reaches through nested object fields.

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

> Readonly views keep field types for reads.

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

> Readonly struct fields cannot be assigned through a readonly view.

```ds
struct User {
    name: string;
}

declare const user: readonly User;
user.name = "Grace";
```

- contains: readonly

### readonly structs reject nested field writes

> Readonly reaches through nested struct fields.

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

> Mutable arrays are assignable to readonly arrays.

```ds
declare let values: number[];
let frozen: readonly number[] = values;
frozen satisfies readonly number[];
```

### readonly arrays reject mutable assignment

> Readonly arrays are not assignable to mutable arrays.

```ds
declare let frozen: readonly number[];
let bad: number[] = frozen;
```

- contains: not assignable

### readonly arrays reject index writes

> Readonly arrays cannot be assigned through indexed access.

```ds
declare const values: readonly number[];
values[0] = 1;
```

- contains: readonly

### readonly arrays reject nested element writes

> Readonly reaches through array elements.

```ds
type User = {
    name: string;
};

declare const users: readonly User[];
users[0].name = "Grace";
```

- contains: readonly

### readonly arrays reject mutation methods

> Readonly arrays do not expose mutating array methods.

```ds
declare const values: readonly number[];
values.push(1);
```

- contains: push

## slices

### readonly slices reject index writes

> Readonly slices cannot be assigned through indexed access.

```ds
declare const values: readonly [number];
values[0] = 1;
```

- contains: readonly

### readonly slices reject nested element writes

> Readonly reaches through slice elements.

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

> Readonly fixed arrays cannot be assigned through indexed access.

```ds
declare const values: readonly [number; 3];
values[0] = 1;
```

- contains: readonly

### readonly fixed arrays reject nested element writes

> Readonly reaches through fixed array elements.

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

> Mutable tuples are assignable to readonly tuples.

```ds
type Pair = (number, string);
type ReadonlyPair = readonly (number, string);

declare let pair: Pair;
let frozen: ReadonlyPair = pair;
frozen satisfies ReadonlyPair;
```

### readonly tuples reject mutable assignment

> Readonly tuples are not assignable to mutable tuples.

```ds
type Pair = (number, string);
type ReadonlyPair = readonly (number, string);

declare let frozen: ReadonlyPair;
let bad: Pair = frozen;
```

- contains: not assignable

### readonly tuples reject element writes

> Readonly tuples cannot be assigned through indexed access.

```ds
declare const pair: readonly (number, string);
pair[0] = 1;
```

- contains: readonly

### readonly tuples reject nested element writes

> Readonly reaches through tuple elements.

```ds
type User = {
    name: string;
};

declare const pair: readonly (User, number);
pair[0].name = "Grace";
```

- contains: readonly
