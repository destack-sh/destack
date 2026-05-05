# Capture Annotations

`@capture` controls how a closure environment captures surrounding bindings.

## policy

### borrow capture keeps the original binding

> A borrow capture gives the closure borrowed access to the captured binding.

```ds
let count = 0;

@capture("borrow")
const next = () => {
    count += 1;
    return count;
};

next satisfies () => number;
```

### copy capture snapshots the binding value

> A copy capture gives the closure its own copied value.

```ds
let name = "Ada";

@capture("copy")
const greet = () => `hello ${name}`;

greet satisfies () => string;
```

### move capture transfers ownership into the closure

> A move capture consumes the captured binding for the closure environment.

```ds
declare function connect(): Result<Socket, IOError>;

let socket = connect()?;

@capture("move")
const send = (message: string) => socket.write(message);

send satisfies (message: string) => Result<void, IOError>;
```

### object directives can configure individual bindings

> Object form sets a default and overrides selected captures.

```ds
class Client {
    prefix: string = "client";

    make(socket: Socket, logger: Logger): (message: string) => Result<void, IOError> {
        @capture({
            default: "copy",
            socket: "move",
            logger: "borrow",
            this: "borrow",
        })
        return (message) => {
            logger.info("sending");
            return socket.write(`${this.prefix}: ${message}`);
        };
    }
}
```

### capture accepts static directives

> Capture directives can come from static values.

```ds
const policy: CaptureDirective = "copy";

@capture(policy)
const read = () => "ready";

read satisfies () => string;
```

## rejections

### capture rejects non-function targets

> `@capture` only supports declarations that create function-like values.

```ds
@capture("copy")
const value = 1;
```

- contains: capture

### capture rejects unknown policies

> Capture policies are checked through `CaptureDirective`.

```ds
@capture("maybe")
const read = () => "ready";
```

- contains: CaptureDirective

### capture rejects unknown rule values

> Per-binding rules use the same capture policy set.

```ds
let name = "Ada";

@capture({ name: "borrow" })
const read = () => name;
```

- contains: CaptureDirective
