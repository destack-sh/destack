# Tree Literals

Tree literal fixtures cover elements, generic tags, attributes, expressions, and multiline wrappers.

## Elements

### element with text

Text content stays inline when it fits.

```tspp:main.tspp
const node = <div>Hello</div>
```

```tspp expected
const node = <div>Hello</div>;
```

### member expression element

Member expression tags keep the dotted path.

```tspp:main.tspp
const node = <UI.Button label="Ok" />
```

```tspp expected
const node = <UI.Button label="Ok" />;
```

### text whitespace normalizes

Whitespace in tree text collapses to single spaces.

```tspp:main.tspp
const node = <div>  Hello   World </div>
```

```tspp expected
const node = <div> Hello World </div>;
```

### whitespace expression containers normalize

Whitespace expression containers normalize to tree text spacing.

```tspp:main.tspp
const node = <div>{" "}Hello{" "}World{" "}</div>
```

```tspp expected
const node = <div> Hello World </div>;
```

## Generic Tags

### generic tag

Generic tag parameters stay attached to the tag name.

```tspp:main.tspp
const node = <Component<any>></Component>
```

```tspp expected
const node = <Component<any>></Component>;
```

### generic tag with children

Generic opening tags do not repeat parameters on closing tags.

```tspp:main.tspp
const node = <Widget<string>>Hello</Widget>
```

```tspp expected
const node = <Widget<string>>Hello</Widget>;
```

### generic tag with attributes

Generic tags format with attributes normally.

```tspp:main.tspp
const node = <Select<Option> value={"ok"}  disabled={true}/>
```

```tspp expected
const node = <Select<Option> value={"ok"} disabled={true} />;
```

### nested generic tag

Nested generic tags keep paired angle closings.

```tspp:main.tspp
const node = <Component<Array<string>> />
```

```tspp expected
const node = <Component<Array<string>> />;
```

### multiline generic tag

Long generic tags still break like normal.

```tspp:main.tspp line-width=30
const node = <Panel<Props> title="Settings" description="Long description" />
```

```tspp expected
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

```tspp:main.tspp
const node = <Button disabled={true} primary={true} />
```

```tspp expected
const node = <Button disabled={true} primary={true} />;
```

### spread attributes

Spread attributes keep braces and spacing.

```tspp:main.tspp
const node = <Button {...props} size="large" />
```

```tspp expected
const node = <Button {...props} size="large" />;
```

## Expressions

### conditional child stays inline

Conditional tree expressions stay inline when they fit.

```tspp:main.tspp
const node = <div>{ready && <Spinner />}</div>
```

```tspp expected
const node = <div>{ready && <Spinner />}</div>;
```

### fragment children break

Fragments with multiple children break across lines.

```tspp:main.tspp
const node = <><Header /><Body /><Footer /></>
```

```tspp expected
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

```tspp:main.tspp line-width=30
const node = <Panel title="Settings" description="Long description" />
```

```tspp expected
const node = (
    <Panel
        title="Settings"
        description="Long description"
    />
);
```
