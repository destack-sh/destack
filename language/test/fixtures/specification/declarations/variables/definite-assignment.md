# Variable Definite Assignment

Tests for definite assignment assertions on variable declarators.

## definite assignment assertions

### typescript variable declarators cannot use definite assignment assertions

> Definite assignment assertions are only allowed on class fields.

```ts:main.ts
let value!: string;
```

- definite assignment assertions are not valid in variable declarators

### destack variable declarators may use definite assignment assertions

> Destack bindings allow definite assignment assertions.

```ds
let value!: string;
value satisfies string;
```

### destack definite assignment assertions remain assignable after writes

> Destack definite assignment assertions can be satisfied by later writes before use.

```ds
let value!: string;
value = "ready";
value satisfies string;
```

### typescript const declarators cannot use definite assignment assertions

> Definite assignment assertions are not valid on const variable declarators in TypeScript.

```ts:main.ts
const value!: string = "ready";
```

- definite assignment assertions are not valid in variable declarators

### destack definite assignment assertions still enforce declared types

> Definite assignment assertions do not weaken declared type compatibility.

```ds
let value!: string;
value = 1;
```

- type 1 is not assignable to type string