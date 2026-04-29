# TSX Comment Boundaries

Tests for TSX comment attachment boundaries.

## Child Containers

### comment as only child

Comment only child containers stay on their own line.

```tsx:main.tsx
const node = <div>{/* only-child */}</div>
```

```tsx expected
const node = <div>{/* only-child */}</div>;
```

### comment between sibling children

Comments between sibling children stay between the same children.

```tsx:main.tsx line-width=40
const node = <div><A />{/* between */}<B /></div>
```

```tsx expected
const node = (
    <div>
        <A />
        {/* between */}
        <B />
    </div>
);
```

## Ternaries

### ternary branch block comments

Block comments inside ternary branches are preserved.

```tsx:main.tsx
const node = (
  <div>
    {isVideo ? <Video /> /* video-comment */ : <Image /> /* image-comment */}
  </div>
)
```

```tsx expected
const node = <div>{isVideo ? <Video /> : /* video-comment */ <Image /> /* image-comment */}</div>;
```

### ternary alternate trailing line comment

Trailing line comments on alternate branches stay with the alternate branch.

```tsx:main.tsx
const node = (
  <>
    {x ? <A /> : // alt-line
    <B />}
  </>
)
```

```tsx expected
const node = (
    <>
        {x ? (
            <A /> // alt-line
        ) : (
            <B />
        )}
    </>
);
```

## Inline Expressions

### map callback with inline comment

Inline comments in TSX expression callbacks stay attached to the callback body.

```tsx:main.tsx
const node = <div>{items.map((item) => item /* map-inline */)}</div>
```

```tsx expected
const node = <div>{items.map((item) => item /* map-inline */)}</div>;
```

### logical expression with trailing comment

Trailing comments in logical TSX expressions stay on the same logical line.

```tsx:main.tsx
const node = <div>{ready && <Body /> // logical-tail
}</div>
```

```tsx expected
const node = (
    <div>
        {
            ready && <Body /> // logical-tail
        }
    </div>
);
```

## Call Arguments

### jsx first argument trailing comment

Trailing comments on JSX first arguments stay attached to that argument.

```tsx:main.tsx
send(
  <Card />, // jsx-first
  options,
)
```

```tsx expected
send(
    <Card />, // jsx-first
    options,
);
```

### jsx generic element with trailing comment

Trailing comments on generic JSX element heads stay attached to the same JSX argument.

```tsx:main.tsx line-width=36
send(
  <Card<T> value={value} />, // jsx-generic
  options,
)
```

```tsx expected
send(
    <Card<T> value={value} />, // jsx-generic
    options,
);
```

### jsx argument followed by line comments

Line comments after a JSX argument stay in the following argument position.

```tsx:main.tsx
someFunction(
  <Component
    value1={{
      foo: "bar",
    }}
  />,
  // option stays after jsx argument
)
```

```tsx expected
someFunction(
    <Component
        value1={{
            foo: "bar",
        }}
    />,
    // option stays after jsx argument
);
```
