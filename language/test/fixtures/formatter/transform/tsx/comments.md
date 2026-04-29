# TSX Comments

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
