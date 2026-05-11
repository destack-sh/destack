# Pin

## roots

### boxes can be pinned

Pinned boxes own stable heap storage and prevent safe code from moving the boxed value out.

```ds
class User {
    name: string = "";
}

let user: ^User = new User();
let pinned = Box.pin(user);

pinned satisfies Pin<Box<User>>;
```

### pinned roots can be borrowed by address-sensitive code

Borrowing through a pinned root keeps the address-sensitive owner in place.

```ds
class User {
    name: string = "";
}

let user: ^User = new User();
let pinned = Box.pin(user);
let name = &readonly pinned.name;

name satisfies &readonly string;
```

### movable pinned roots can be unwrapped

`Unpin` is the explicit capability for moving a value out of `Pin`.

```ds
struct Token implements Unpin {
    value: int32;
}

let token = Pin.new(Token { value: 1 });
let value = token.intoInner();

value satisfies Token;
```

### address-sensitive roots can opt out of unpin

Types that store or expose self-references can explicitly reject `Unpin`.

```ds
struct IntrusiveNode implements !Unpin {
    next: *IntrusiveNode;
}

let node = Box.pin(IntrusiveNode { next: null });

node satisfies Pin<Box<IntrusiveNode>>;
```

### pinned exclusive access stays pinned

Exclusive access to a pinned non-`Unpin` value preserves the pin.

```ds
struct IntrusiveNode implements !Unpin {
    next: *IntrusiveNode;
}

let node = Box.pin(IntrusiveNode { next: null });
let projected = node.asPinnedExclusive();

projected satisfies Pin<&exclusive Box<IntrusiveNode>>;
```

## self borrows

### inline self borrows are rejected

Inline owned storage can move with the containing value, so a stored borrow into that storage is not address-stable.

```ds
class User {
    name: string = "";
}

struct NameView<L: Lifetime> {
    user: ^User;
    name: Borrowed<string, L>;
}

function makeView<L: Lifetime>(): NameView<L> {
    let user: ^User = new User();
    return NameView {
        user,
        name: &user.name,
    };
}
```

- contains: borrow
