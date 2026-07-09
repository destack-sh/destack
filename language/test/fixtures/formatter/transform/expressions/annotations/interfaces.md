# Interface Annotations

## Interfaces and Method Types

### interface method parameter trailing comment

Trailing comments inside method parameter lists stay attached to the same parameter.

```ds:main.ds
interface Worker {
  run(
    value: string, // value-tail
  ): number
}
```

```ds expected
interface Worker {
    run(
        value: string, // value-tail
    ): number;
}
```

### interface method return boundary comment

Boundary comments around method return types stay attached to the same method signature.

```ds:main.ds
interface Worker {
  run(): // return-tail
  Promise<void>
}
```

```ds expected
interface Worker {
    run(): // return-tail
    Promise<void>;
}
```
