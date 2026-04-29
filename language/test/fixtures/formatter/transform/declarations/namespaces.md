# Namespace Declarations

Namespace fixtures cover namespace, module, and global declaration bodies.

## Namespace Forms

### namespace declaration

Namespaces format like other declaration blocks.

```ts:main.ts
namespace   Foo{const x=1}
```

```ts expected
namespace Foo {
    const x = 1;
}
```

### exported namespace

Exported namespaces keep the export keyword.

```ts:main.ts
export   namespace  Foo { }
```

```ts expected
export namespace Foo {}
```

### declared namespace

Declared namespaces keep the declare keyword.

```ts:main.d.ts
declare namespace Foo {
    export const version: string
}
```

```ts expected
declare namespace Foo {
    export const version: string;
}
```

## Module Alias

### string literal ambient module keeps declare module

String-literal ambient declarations keep the `declare module` form.

```ts:main.ts
declare module   "Bar" { export const value:number }
```

```ts expected
declare module "Bar" {
    export const value: number;
}
```

## Where Clauses

### namespace with where clause

Where clauses stay attached to the namespace header.

```ds
namespace Foo where Guard: Limit { const x = 1 }
```

```ds expected
namespace Foo where Guard: Limit {
    const x = 1;
}
```
