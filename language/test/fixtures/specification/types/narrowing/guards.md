# Guard Narrowing

Guard shapes beyond nullish checks.

## Typeof Guards

### typeof guard narrows to string

```ds
const value: string | number = 0;
if (typeof value == "string") {
    value satisfies string;
} else {
    value satisfies number;
}
```

### typeof guard narrows to object

```ds
const value: { name: string } | null | string = "hello";
if (typeof value == "object") {
    value satisfies { name: string } | null;
} else {
    value satisfies string;
}
```

### typeof guard narrows to function

```ds
const value: { (): void } | string = "hello";
if (typeof value == "function") {
    value satisfies { (): void };
} else {
    value satisfies string;
}
```

### typeof guard narrows to lambda type

```ds
const value: (() => void) | string = "hello";
if (typeof value == "function") {
    value satisfies () => void;
} else {
    value satisfies string;
}
```

## Instanceof Guards

### instanceof guard narrows to class

```ds
class User {
    name: string = ""
}

const value: User | string = "hello";
if (value instanceof User) {
    value satisfies User;
} else {
    value satisfies string;
}
```

### instanceof rejects non-class targets

```ds
struct Point {
    x: int32;
    y: int32;
}

const value = Point { x: 1, y: 2 };
const ok = value instanceof Point;
```

- instanceof requires a class type

## In Guards

### in guard narrows to required key

```ds
interface WithName { name: string }
interface WithId { id: number }

function narrow(value: WithName | WithId): void {
    if ("name" in value) {
        value satisfies WithName;
    } else {
        value satisfies WithId;
    }
}
```

## Is Guards

### is guard narrows to target type

```ds
class Admin {
    name: string = ""
}

const value: Admin | string = "root";
if (value is Admin) {
    value satisfies Admin;
} else {
    value satisfies string;
}
```

### is guard yields boolean

```ds
class Admin {
    name: string = ""
}

const value: unknown = new Admin();
const ok = value is Admin;
ok satisfies boolean;
```

## Assertion Guards

### asserts guards narrow after call

> Assertion functions narrow the asserted value.

```ts
function assertString(value: unknown): asserts value is string {
    if (typeof value != "string") {
        throw 1;
    }
}

let value: string | number = 1;
assertString(value);
value satisfies string;
```
