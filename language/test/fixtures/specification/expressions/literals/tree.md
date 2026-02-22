# Tree Literals

Tree literals use TSX syntax.

## basic trees

### tree literal is allowed

> Tree literals are valid expressions.

```ds
declare const A: unknown;

let node = <A/>;
node;
```

### tree literal accepts attributes

> Tree literals accept attribute arguments.

```ds
declare const A: unknown;

let node = <A value={1} />;
node;
```

### tree literal accepts children

> Tree literals accept child expressions.

```ds
declare const A: unknown;

let node = <A>{1}</A>;
node;
```

### tree literal works in tsx

> TSX sources accept tree literals without Destack extensions.

```ts:main.tsx
declare const A: unknown;

const node = <A value={1} />;
node;
```

### tree literals reject duplicate attributes

> Tree literals reject duplicate attributes on the same tag.

```ds
declare const A: unknown;

const node = <A value={1} value={2} />;
node;
```

- contains: duplicate

### tree literals support nested children

> Tree literals support nested child tree expressions.

```ds
declare const A: unknown;
declare const B: unknown;

const node = <A><B value={1} /></A>;
node;
```
