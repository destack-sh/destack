# Tree Tags

Tree tags resolve either as values or as configured intrinsic tags.

## value tags

### structural component tag is allowed

> Value tags can be resolved from structural component shape.

```ds
declare function Button(props: { kind: string, children?: unknown[] }): unknown;

const node = <Button kind="primary">ok</Button>;
node;
```

### key and ref are not language reserved

> The language does not reserve key and ref prop names globally.

```ds
declare function Button(props: { key: string, ref: string, children?: unknown[] }): unknown;

const node = <Button key="k1" ref="r1" />;
node;
```

## intrinsic tags

### lowercase intrinsic requires configured TreeTagBuilder

> Lowercase tags require an active TreeTagBuilder.

```ds
const node = <div className="card" />;
node;
```

- contains: intrinsic

### fragment requires configured TreeTagBuilder

> Fragments resolve through TreeTagBuilder and fail without one.

```ds
declare function Button(props: { children?: unknown[] }): unknown;

const node = <>
    <Button />
</>;
node;
```

- Fragment

### namespaced tags route as intrinsic string names

> XML namespaced tags are routed as intrinsic string names.

```ds
const node = <svg:path />;
node;
```

- contains: intrinsic
- svg:path

## attributes and spread

### explicit props override spread props by source order

> Later explicit props override earlier spread props.

```ds
declare function Button(props: { a: number, b: number, children?: unknown[] }): unknown;

const base = { a: 1, b: 2 };
const node = <Button {...base} b={3} />;
node;
```

### open shape spread is rejected

> Open shape spreads are not allowed in tree literals.

```ds
declare function Button(props: { children?: unknown[] }): unknown;
declare const dynamicProps: Record<string, unknown>;

const node = <Button {...dynamicProps} />;
node;
```

- spread
- dynamic
