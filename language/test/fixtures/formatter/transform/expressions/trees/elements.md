# TSX Tree Literals

TSX tree fixtures cover elements, generic tags, attributes, expressions, and multiline wrappers.

## Elements

### element with text

Text content stays inline when it fits.

```tsx:main.tsx
const node = <div>Hello</div>
```

```tsx expected
const node = <div>Hello</div>;
```

### member expression element

Member expression tags keep the dotted path.

```tsx:main.tsx
const node = <UI.Button label="Ok" />
```

```tsx expected
const node = <UI.Button label="Ok" />;
```

### text whitespace normalizes

Whitespace in JSX text collapses to single spaces.

```tsx:main.tsx
const node = <div>  Hello   World </div>
```

```tsx expected
const node = <div> Hello World </div>;
```

### whitespace expression containers are preserved

Whitespace expression containers stay as string literals.

```tsx:main.tsx
const node = <div>{" "}Hello{" "}World{" "}</div>
```

```tsx expected
const node = <div> Hello World </div>;
```

## Generic Tags

### generic tag

Generic tag parameters stay attached to the tag name.

```tsx:main.tsx
const node = <Component<any>></Component>
```

```tsx expected
const node = <Component<any>></Component>;
```

### generic tag with children

Generic opening tags do not repeat parameters on closing tags.

```tsx:main.tsx
const node = <Widget<string>>Hello</Widget>
```

```tsx expected
const node = <Widget<string>>Hello</Widget>;
```

### generic tag with attributes

Generic tags format with attributes normally.

```tsx:main.tsx
const node = <Select<Option> value={"ok"}  disabled={true}/>
```

```tsx expected
const node = <Select<Option> value={"ok"} disabled={true} />;
```

### nested generic tag

Nested generic tags keep paired angle closings.

```tsx:main.tsx
const node = <Component<Array<string>> />
```

```tsx expected
const node = <Component<Array<string>> />;
```

### multiline generic tag

Long generic tags still break like normal.

```tsx:main.tsx line-width=30
const node = <Panel<Props> title="Settings" description="Long description" />
```

```tsx expected
const node = (
    <Panel<Props>
        title="Settings"
        description="Long description"
    />
);
```

## Attributes

### boolean attribute shorthand

Boolean attributes omit `={true}`.

```tsx:main.tsx
const node = <Button disabled={true} primary={true} />
```

```tsx expected
const node = <Button disabled={true} primary={true} />;
```

### spread attributes

Spread attributes keep braces and spacing.

```tsx:main.tsx
const node = <Button {...props} size="large" />
```

```tsx expected
const node = <Button {...props} size="large" />;
```

## Expressions

### conditional child stays inline

Conditional JSX expressions stay inline when they fit.

```tsx:main.tsx
const node = <div>{ready && <Spinner />}</div>
```

```tsx expected
const node = <div>{ready && <Spinner />}</div>;
```

### fragment children break

Fragments with multiple children break across lines.

```tsx:main.tsx
const node = <><Header /><Body /><Footer /></>
```

```tsx expected
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

Multiline JSX in assignments is wrapped in parentheses.

```tsx:main.tsx line-width=30
const node = <Panel title="Settings" description="Long description" />
```

```tsx expected
const node = (
    <Panel
        title="Settings"
        description="Long description"
    />
);
```
