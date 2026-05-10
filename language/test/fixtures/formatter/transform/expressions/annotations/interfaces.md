# Interface Annotations

## Interfaces and Method Types

### interface method parameter trailing comment

Trailing comments inside method parameter lists stay attached to the same parameter.

```ts:main.ts
interface Worker {
  run(
    value: string, // value-tail
  ): number
}
```

```ts expected
interface Worker {
    run(
        value: string, // value-tail
    ): number;
}
```

### interface method return boundary comment

Boundary comments around method return types stay attached to the same method signature.

```ts:main.ts
interface Worker {
  run(): // return-tail
  Promise<void>
}
```

```ts expected
interface Worker {
    run(): // return-tail
    Promise<void>;
}
```
