# Type Overview

## nominal and structural types

### nominal newtypes are distinct

> Nominal newtypes do not mix even when they share a backing type.

```ds
newtype UserId = int64;
newtype OrderId = int64;

let user: UserId = UserId(1);
let order: OrderId = user;
```

- not assignable

### structs model value types

> Structs model value oriented types with named fields.

```ds
struct Point { x: float32; y: float32 }

const point = Point { x: 1.0, y: 2.0 };
point.x satisfies float32;
point.y satisfies float32;
```

### unions model one of several value shapes

> Union types admit values from each member type.

```ds
type Token = string | int32;

let left: Token = "ok";
let right: Token = 1;
```

### unions reject values outside all members

> Union types reject values that fit none of their member types.

```ds
type Token = string | int32;

let value: Token = true;
```

- not assignable

### intersections require values that satisfy all members

> Intersection types require values satisfying each intersected member.

```ds
type Named = { name: string };
type Active = { active: boolean };
type User = Named & Active;

let value: User = { name: "Ada", active: true };
```
