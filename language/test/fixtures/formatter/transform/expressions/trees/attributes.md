# TSX Attributes

TSX attribute fixtures cover attribute spacing, expressions, comments, spread attributes, and wrapping options.

## Attribute Forms

### spacing and boolean expression attributes

Attribute spacing is normalized without rewriting expression attribute values.

```tsx:main.tsx
const node = <Button disabled={true} count={ 1 } label="Ok" />
```

```tsx expected
const node = <Button disabled={true} count={1} label="Ok" />;
```

### string expression attribute quote style

String literal expression containers keep their braces while quote style is normalized.

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

### template attribute interpolation comments

Template attributes keep multiline interpolation comments indented from the template segment.

```tsx:main.tsx
const node = <Panel className={`
  color: ${theme?.activeColor[
    // selected mode
    mode === "dark" ? "dark" : "light"
  ]};
`} />
```

```tsx expected
const node = (
    <Panel
        className={`
  color: ${
      theme?.activeColor[
          // selected mode
          mode === "dark" ? "dark" : "light"
      ]
  };
`}
    />
);
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

### self closing final attribute comment

Trailing comments after the final attribute keep the closing bracket on the next line.

```tsx:main.tsx
const node = <Widget
  title="Settings" // title-tail
/>
```

```tsx expected
const node = (
    <Widget
        title="Settings" // title-tail
    />
);
```

### tag name comment before attributes

Comments after the tag name stay before the first attribute.

```tsx:main.tsx
const node = <Widget
  /* tag-tail */
  title="Settings"
/>
```

```tsx expected
const node = (
    <Widget
        /* tag-tail */
        title="Settings"
    />
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

### expression attribute value breaks internally

Long expression attribute values break inside the expression container.

```tsx:main.tsx line-width=45
const node = <Panel options={{ label: "Settings", description: "Long description" }} />
```

```tsx expected
const node = (
    <Panel
        options={{
            label: "Settings",
            description: "Long description",
        }}
    />
);
```

### jsx callback attribute breaks around return element

JSX-returning callback attributes break the element and callback body vertically.

```tsx:main.tsx
const node = <List renderItem={(item) => <Item key={item.id}>{item.name}</Item>} />
```

```tsx expected
const node = (
    <List
        renderItem={(item) => (
            <Item key={item.id}>{item.name}</Item>
        )}
    />
);
```
