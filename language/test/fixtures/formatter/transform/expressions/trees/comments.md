# Tree Comments

TSX comment fixtures cover comment containers, dangling comments, and arrow-expression comments.

## Comment Containers

### comment as only child

Block comments in expression containers expand the element.

```tsx:main.tsx
const node = <div>{/* TODO: add content */}</div>
```

```tsx expected
const node = <div>{/* TODO: add content */}</div>;
```

### comment between children

Comments between children stay on their own line.

```tsx:main.tsx line-width=40
const node = <div>{/* header */}<Header /><Body /></div>
```

```tsx expected
const node = (
    <div>
        {/* header */}
        <Header />
        <Body />
    </div>
);
```

### comment inside expression child

Comments inside expression containers are preserved.

```tsx:main.tsx
const node = <div>{items /* keep */ .map((item) => <Item key={item.id} />)}</div>
```

```tsx expected
const node = (
    <div>
        {items /* keep */
            .map((item) => (
                <Item key={item.id} />
            ))}
    </div>
);
```

## Dangling Comments

### fragment expression with dangling comment

Dangling comments stay inside the expression container.

```tsx:main.tsx
const node = (<>
  {
    value
    // this comment should stay here
  }
</>)
```

```tsx expected
const node = (
    <>
        {
            value
            // this comment should stay here
        }
    </>
);
```

### fragment expression without comments stays inline

Simple fragment expressions stay on one line.

```tsx:main.tsx
const node = <>{ value }</>
```

```tsx expected
const node = <>{value}</>;
```

## Arrow Expressions

### arrow expression with comment

Arrow expressions inside JSX containers break with comments preserved.

```tsx:main.tsx
const node = <>
  <div>
    {() => function A() {
      A();
    } /* comment */}
  </div>
</>
```

```tsx expected
const node = (
    <>
        <div>
            {
                () =>
                    function A() {
                        A();
                    } /* comment */
            }
        </div>
    </>
);
```

### ternary with trailing comment

Trailing comments on JSX branches stay on the same line.

```tsx:main.tsx line-width=30
const node = <div>{isVideo ? <Video /> : <Image /> // eslint-disable-line
}</div>
```

```tsx expected
const node = (
    <div>
        {
            isVideo ? (
                <Video />
            ) : (
                <Image />
            ) // eslint-disable-line
        }
    </div>
);
```

TSX comment boundary fixtures cover comments around children, ternaries, inline expressions, and call arguments.

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

### ternary branch inline comments

Inline comments inside TSX ternary branches stay attached on both sides.

```tsx:main.tsx line-width=40
const node = <div>{isVideo ? <Video /> /* keep-video */ : <Image /> /* keep-image */}</div>
```

```tsx expected
const node = (
    <div>
        {
            isVideo ? (
                <Video />
            ) : (
                /* keep-video */ <Image />
            ) /* keep-image */
        }
    </div>
);
```

### ternary alternate block comment

Block comments inside alternate TSX branches stay with the alternate branch.

```tsx:main.tsx
const Component = () => (
  <div>
    {"error" ? (
      <Error />
    ) : (
      <Success />
      /* keep-inside-branch */
    )}
  </div>
)
```

```tsx expected
const Component = () => (
    <div>
        {"error" ? (
            <Error />
        ) : (
            <Success />
            /* keep-inside-branch */
        )}
    </div>
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

## JSX Comments

### comment in expression container

Comments inside JSX use expression containers.
Block infix comments cause expansion with stable indentation.

```ds
<Container>{/* XOXO: something something add content */}</Container>
```

```ds expected
<Container>{/* XOXO: something something add content */}</Container>;
```
