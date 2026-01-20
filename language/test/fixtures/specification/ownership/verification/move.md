# Move Verification

## move conflicts

### use after move is rejected

> Values moved with `^` cannot be used afterwards.

```ds native=true
struct Data {
    value: int32,
}

function consume(value: ^Data): void {
    value.value;
}

function run(): void {
    let data = ^Data { value: 1 };
    consume(data);
    data.value;
}
```

- contains: use of moved value
