# New

## ownership

### new creates managed values for managed destinations

> `new` follows the destination ownership form.

```ds
class Box {
    value: int32 = 0;
}

let value: Box = new Box();
value satisfies Box;
```

### new creates owned values for owned destinations

> Owned destinations receive owned heap values.

```ds
class Box {
    value: int32 = 0;
}

let value: ^Box = new Box();
value satisfies ^Box;
```

### new creates owned shared values for owned shared destinations

> `new` follows ownership and space in the destination type.

```ds
class Box {
    value: int32 = 0;
}

let value: ^shared Box = new Box();
value satisfies ^shared Box;
```

## space

### new creates shared values for shared destinations

> Shared destinations allocate in shared space.

```ds
class Box {
    value: int32 = 0;
}

let value: shared Box = new Box();
value satisfies shared Box;
```

### new places ambient fields in the destination space

> Ambient fields follow the placement of the constructed value.

```ds
class Header {
    id: int32 = 0;
}

class Payload {
    value: int32 = 0;
}

class Request<T> {
    header: Header = new Header();
    body: T;

    constructor(body: T) {
        this.body = body;
    }
}

let request: shared Request<Payload> = new Request(new Payload());
request.header satisfies shared Header;
request.body satisfies shared Payload;
```
