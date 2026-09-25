# Tree Comments

Tree comment fixtures cover comment containers, dangling comments, and arrow-expression comments.

## Comment Containers

### comment as only child

Block comments remain inside their expression containers.

```tspp:main.tspp
const node = <div>{/* TODO: add content */}</div>
```

```tspp expected
const node = <div>{/* TODO: add content */}</div>;
```

### comment between children

Comments between children stay on their own line.

```tspp:main.tspp line-width=40
const node = <div>{/* header */}<Header /><Body /></div>
```

```tspp expected
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

```tspp:main.tspp
const node = <div>{items /* keep */ .map((item) => <Item key={item.id} />)}</div>
```

```tspp expected
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

```tspp:main.tspp
const node = (<>
  {
    value
    // this comment should stay here
  }
</>)
```

```tspp expected
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

```tspp:main.tspp
const node = <>{ value }</>
```

```tspp expected
const node = <>{value}</>;
```

## Arrow Expressions

### arrow expression with comment

Arrow expressions inside tree containers break with comments preserved.

```tspp:main.tspp
const node = <>
  <div>
    {() => function A() {
      A();
    } /* comment */}
  </div>
</>
```

```tspp expected
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

Trailing comments on tree branches stay on the same line.

```tspp:main.tspp line-width=30
const node = <div>{isVideo ? <Video /> : <Image /> // eslint-disable-line
}</div>
```

```tspp expected
const node = (
    <div>
        {
            isVideo ? (
                <Video />
            ) : (
                <Image /> // eslint-disable-line
            )
        }
    </div>
);
```

Tree comment boundaries cover children, ternaries, inline expressions, and call arguments.

## Sibling Comments

### comment between sibling children

Comments between sibling children stay between the same children.

```tspp:main.tspp line-width=40
const node = <div><A />{/* between */}<B /></div>
```

```tspp expected
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

```tspp:main.tspp
const node = (
  <div>
    {isVideo ? <Video /> /* video-comment */ : <Image /> /* image-comment */}
  </div>
)
```

```tspp expected
const node = <div>{isVideo ? <Video /> /* video-comment */ : <Image /> /* image-comment */}</div>;
```

### ternary alternate trailing line comment

Trailing line comments on alternate branches stay with the alternate branch.

```tspp:main.tspp
const node = (
  <>
    {x ? <A /> : // alt-line
    <B />}
  </>
)
```

```tspp expected
const node = (
    <>
        {x ? (
            <A />
        ) : (
            // alt-line
            <B />
        )}
    </>
);
```

### ternary branch inline comments

Inline comments inside tree ternary branches stay attached on both sides.

```tspp:main.tspp line-width=40
const node = <div>{isVideo ? <Video /> /* keep-video */ : <Image /> /* keep-image */}</div>
```

```tspp expected
const node = (
    <div>
        {isVideo ? (
            <Video /> /* keep-video */
        ) : (
            <Image /> /* keep-image */
        )}
    </div>
);
```

### ternary alternate block comment

Block comments inside alternate tree branches stay with the alternate branch.

```tspp:main.tspp
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

```tspp expected
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

Inline comments in tree expression callbacks stay attached to the callback body.

```tspp:main.tspp
const node = <div>{items.map((item) => item /* map-inline */)}</div>
```

```tspp expected
const node = <div>{items.map((item) => item /* map-inline */)}</div>;
```

### logical expression with trailing comment

Trailing comments in logical tree expressions stay on the same logical line.

```tspp:main.tspp
const node = <div>{ready && <Body /> // logical-tail
}</div>
```

```tspp expected
const node = (
    <div>
        {
            ready && <Body /> // logical-tail
        }
    </div>
);
```

## Call Arguments

### tree first argument trailing comment

Trailing comments on tree first arguments stay attached to that argument.

```tspp:main.tspp
send(
  <Card />, // tree-first
  options,
)
```

```tspp expected
send(
    <Card />, // tree-first
    options,
);
```

### tree generic element with trailing comment

Trailing comments on generic tree element heads stay attached to the same tree argument.

```tspp:main.tspp line-width=36
send(
  <Card<T> value={value} />, // tree-generic
  options,
)
```

```tspp expected
send(
    <Card<T> value={value} />, // tree-generic
    options,
);
```

### tree argument followed by line comments

Line comments after a tree argument stay in the following argument position.

```tspp:main.tspp
someFunction(
  <Component
    value1={{
      foo: "bar",
    }}
  />,
  // option stays after tree argument
)
```

```tspp expected
someFunction(
    <Component
        value1={{
            foo: "bar",
        }}
    />,
    // option stays after tree argument
);
```

## Inline Comments

### comment in expression container

Comments inside tree literals use expression containers.
Comments remain inline when the element fits.

```tspp
<Container>{/* XOXO: something something add content */}</Container>
```

```tspp expected
<Container>{/* XOXO: something something add content */}</Container>;
```
