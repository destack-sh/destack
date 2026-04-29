# TSX Attributes

Tests for TSX attribute formatting and layout behavior.

## Basic Attributes

### spacing and boolean shorthand

Attribute spacing is normalized and boolean attributes use shorthand.

```tsx:main.tsx
const node = <Button disabled={true} count={ 1 } label="Ok" />
```

```tsx expected
const node = <Button disabled={true} count={1} label="Ok" />;
```

### string expression attribute collapses

String literal expression containers collapse to plain string attributes.

```tsx:main.tsx
const node = <div title={"Hello"} className={'card'} />
```

```tsx expected
const node = <div title={"Hello"} className={"card"} />;
```

### mixed attribute kinds

Mixed attribute types stay ordered and normalized.

```tsx:main.tsx
const node = <Input name="search" value={query} onChange={(e) => setQuery(e.target.value)} />
```

```tsx expected
const node = <Input name="search" value={query} onChange={(e) => setQuery(e.target.value)} />;
```

### expression attribute with call

Complex expression attributes remain wrapped in braces.

```tsx:main.tsx
const node = <div className={cx("a", { b: cond })} />
```

```tsx expected
const node = <div className={cx("a", { b: cond })} />;
```

### attributes with trailing comments

Trailing attribute comments stay attached to the same attributes.

```tsx:main.tsx
const node = <div
  key={formMessageId} // key-tail
  initial={{ opacity: 0, y: -5, height: 0 }} // initial-tail
  style={{ /* before */ overflow: "hidden" /* after */  }}
></div>
```

```tsx expected
const node = (
    <div
        key={formMessageId} // key-tail
        initial={{ opacity: 0, y: -5, height: 0 }} // initial-tail
        style={{ /* before */ overflow: "hidden" /* after */ }}
    ></div>
);
```

## Spread Attributes

### spread attributes keep order

Spread attributes preserve their position in the attribute list.

```tsx:main.tsx
const node = <Widget {...props} kind="primary" {...extra} />
```

```tsx expected
const node = <Widget {...props} kind="primary" {...extra} />;
```

### spread attributes with comments

Spread attribute comments stay attached to the same attribute boundaries.

```tsx:main.tsx
const node = <Widget
  // props-leading
  {...props} // props-tail
  kind="primary"
  // extra-leading
  {...extra}
/>
```

```tsx expected
const node = (
    <Widget
        // props-leading
        {...props} // props-tail
        kind="primary"
        // extra-leading
        {...extra}
    />
);
```

## Line Breaking

### attributes break when too long

Long attribute lists break across lines.

```tsx:main.tsx line-width=40
const node = <Panel title="Settings" description="Long description" icon={settingsIcon} />
```

```tsx expected
const node = (
    <Panel
        title="Settings"
        description="Long description"
        icon={settingsIcon}
    />
);
```

### bracket same line option

For self-closing elements, `bracket_same_line` still leaves `/>` on its own line.

```tsx:main.tsx line-width=40 bracket-same-line=true
const node = <Panel title="Settings" description="Long description" icon={settingsIcon} />
```

```tsx expected
const node = (
    <Panel
        title="Settings"
        description="Long description"
        icon={settingsIcon}
    />
);
```

### single attribute per line option

When `single_attribute_per_line` is true, each attribute is on its own line for expanded elements.

```tsx:main.tsx single-attribute-per-line=true
const node = <Button variant="primary" size="large" disabled />
```

```tsx expected
const node = (
    <Button
        variant="primary"
        size="large"
        disabled
    />
);
```
