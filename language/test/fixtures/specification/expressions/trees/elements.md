# Tree Elements

Tree elements use JSX-shaped element syntax.

## elements

### tree elements are expressions

> Tree elements are accepted expressions.

```ds
declare const A: unknown;

let node = <A/>;
node;
```

### tree elements accept attributes

> Tree elements accept attribute arguments.

```ds
declare const A: unknown;

let node = <A value={1} />;
node;
```

### tree elements accept children

> Tree elements accept child expressions.

```ds
declare const A: unknown;

let node = <A>{1}</A>;
node;
```

### tsx sources accept tree elements

> `.tsx` sources accept tree elements without extensions.

```tsx:main.tsx
declare const A: unknown;

const node = <A value={1} />;
node;
```

### tree elements reject duplicate attributes

> Tree elements reject duplicate attributes on the same tag.

```ds
declare const A: unknown;

const node = <A value={1} value={2} />;
node;
```

- contains: duplicate

### tree elements support nested children

> Tree elements support nested child tree expressions.

```ds
declare const A: unknown;
declare const B: unknown;

const node = <A><B value={1} /></A>;
node;
```
