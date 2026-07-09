# Tree Comments

Tree comment fixtures cover comment containers, dangling comments, and arrow-expression comments.

## Comment Containers

### comment as only child

Block comments remain inside their expression containers.

```ds:main.ds
const node = <div>{/* TODO: add content */}</div>
```

```ds expected
const node = <div>{/* TODO: add content */}</div>;
```

### comment between children

Comments between children stay on their own line.

```ds:main.ds line-width=40
const node = <div>{/* header */}<Header /><Body /></div>
```

```ds expected
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

```ds:main.ds
const node = <div>{items /* keep */ .map((item) => <Item key={item.id} />)}</div>
```

```ds expected
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

```ds:main.ds
const node = (<>
  {
    value
    // this comment should stay here
  }
</>)
```

```ds expected
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

```ds:main.ds
const node = <>{ value }</>
```

```ds expected
const node = <>{value}</>;
```

## Arrow Expressions

### arrow expression with comment

Arrow expressions inside tree containers break with comments preserved.

```ds:main.ds
const node = <>
  <div>
    {() => function A() {
      A();
    } /* comment */}
  </div>
</>
```

```ds expected
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

```ds:main.ds line-width=30
const node = <div>{isVideo ? <Video /> : <Image /> // eslint-disable-line
}</div>
```

```ds expected
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

```ds:main.ds line-width=40
const node = <div><A />{/* between */}<B /></div>
```

```ds expected
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

```ds:main.ds
const node = (
  <div>
    {isVideo ? <Video /> /* video-comment */ : <Image /> /* image-comment */}
  </div>
)
```

```ds expected
const node = <div>{isVideo ? <Video /> /* video-comment */ : <Image /> /* image-comment */}</div>;
```

### ternary alternate trailing line comment

Trailing line comments on alternate branches stay with the alternate branch.

```ds:main.ds
const node = (
  <>
    {x ? <A /> : // alt-line
    <B />}
  </>
)
```

```ds expected
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

```ds:main.ds line-width=40
const node = <div>{isVideo ? <Video /> /* keep-video */ : <Image /> /* keep-image */}</div>
```

```ds expected
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

```ds:main.ds
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

```ds expected
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

```ds:main.ds
const node = <div>{items.map((item) => item /* map-inline */)}</div>
```

```ds expected
const node = <div>{items.map((item) => item /* map-inline */)}</div>;
```

### logical expression with trailing comment

Trailing comments in logical tree expressions stay on the same logical line.

```ds:main.ds
const node = <div>{ready && <Body /> // logical-tail
}</div>
```

```ds expected
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

```ds:main.ds
send(
  <Card />, // tree-first
  options,
)
```

```ds expected
send(
    <Card />, // tree-first
    options,
);
```

### tree generic element with trailing comment

Trailing comments on generic tree element heads stay attached to the same tree argument.

```ds:main.ds line-width=36
send(
  <Card<T> value={value} />, // tree-generic
  options,
)
```

```ds expected
send(
    <Card<T> value={value} />, // tree-generic
    options,
);
```

### tree argument followed by line comments

Line comments after a tree argument stay in the following argument position.

```ds:main.ds
someFunction(
  <Component
    value1={{
      foo: "bar",
    }}
  />,
  // option stays after tree argument
)
```

```ds expected
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

```ds
<Container>{/* XOXO: something something add content */}</Container>
```

```ds expected
<Container>{/* XOXO: something something add content */}</Container>;
```
