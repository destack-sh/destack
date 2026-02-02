# Variable Declarations

Tests for variable declaration validation.

## definite assignment assertions

### typescript variable declarators cannot use definite assignment assertions

> Definite assignment assertions are only allowed on class fields.

```ts:main.ts
let value!: string;
```

- contains: definite assignment assertions are not valid in variable declarators

### destack variable declarators may use definite assignment assertions

> Destack bindings allow definite assignment assertions.

```ds
let value!: string;
value satisfies string;
```
