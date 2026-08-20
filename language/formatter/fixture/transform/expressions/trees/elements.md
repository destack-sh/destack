# Tree Literals

Tree literal fixtures cover elements, generic tags, attributes, expressions, and multiline wrappers.

## Elements

### element with text

Text content stays inline when it fits.

```ds:main.ds
const node = <div>Hello</div>
```

```ds expected
const node = <div>Hello</div>;
```

### member expression element

Member expression tags keep the dotted path.

```ds:main.ds
const node = <UI.Button label="Ok" />
```

```ds expected
const node = <UI.Button label="Ok" />;
```

### text whitespace normalizes

Whitespace in tree text collapses to single spaces.

```ds:main.ds
const node = <div>  Hello   World </div>
```

```ds expected
const node = <div> Hello World </div>;
```

### whitespace expression containers normalize

Whitespace expression containers normalize to tree text spacing.

```ds:main.ds
const node = <div>{" "}Hello{" "}World{" "}</div>
```

```ds expected
const node = <div> Hello World </div>;
```

## Generic Tags

### generic tag

Generic tag parameters stay attached to the tag name.

```ds:main.ds
const node = <Component<any>></Component>
```

```ds expected
const node = <Component<any>></Component>;
```

### generic tag with children

Generic opening tags do not repeat parameters on closing tags.

```ds:main.ds
const node = <Widget<string>>Hello</Widget>
```

```ds expected
const node = <Widget<string>>Hello</Widget>;
```

### generic tag with attributes

Generic tags format with attributes normally.

```ds:main.ds
const node = <Select<Option> value={"ok"}  disabled={true}/>
```

```ds expected
const node = <Select<Option> value={"ok"} disabled={true} />;
```

### nested generic tag

Nested generic tags keep paired angle closings.

```ds:main.ds
const node = <Component<Array<string>> />
```

```ds expected
const node = <Component<Array<string>> />;
```

### multiline generic tag

Long generic tags still break like normal.

```ds:main.ds line-width=30
const node = <Panel<Props> title="Settings" description="Long description" />
```

```ds expected
const node = (
    <Panel<Props>
        title="Settings"
        description="Long description"
    />
);
```

## Attributes

### explicit boolean expression attributes

Boolean expression attribute values remain explicit.

```ds:main.ds
const node = <Button disabled={true} primary={true} />
```

```ds expected
const node = <Button disabled={true} primary={true} />;
```

### spread attributes

Spread attributes keep braces and spacing.

```ds:main.ds
const node = <Button {...props} size="large" />
```

```ds expected
const node = <Button {...props} size="large" />;
```

## Expressions

### conditional child stays inline

Conditional tree expressions stay inline when they fit.

```ds:main.ds
const node = <div>{ready && <Spinner />}</div>
```

```ds expected
const node = <div>{ready && <Spinner />}</div>;
```

### fragment children break

Fragments with multiple children break across lines.

```ds:main.ds
const node = <><Header /><Body /><Footer /></>
```

```ds expected
const node = (
    <>
        <Header />
        <Body />
        <Footer />
    </>
);
```

## Multiline Elements

### multiline element wraps in parentheses

Multiline tree in assignments is wrapped in parentheses.

```ds:main.ds line-width=30
const node = <Panel title="Settings" description="Long description" />
```

```ds expected
const node = (
    <Panel
        title="Settings"
        description="Long description"
    />
);
```
