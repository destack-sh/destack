# Tree Attributes

Tree attribute fixtures cover spacing, expressions, comments, spread attributes, and wrapping.

## Attribute Forms

### spacing and boolean expression attributes

Attribute spacing is normalized without rewriting expression attribute values.

```tspp:main.tspp
const node = <Button disabled={true} count={ 1 } label="Ok" />
```

```tspp expected
const node = <Button disabled={true} count={1} label="Ok" />;
```

### string expression attributes

String literal expression containers keep their braces.

```tspp:main.tspp
const node = <div title={"Hello"} className={"card"} />
```

```tspp expected
const node = <div title={"Hello"} className={"card"} />;
```

### mixed attribute kinds

Mixed attribute types stay ordered and normalized.

```tspp:main.tspp
const node = <Input name="search" value={query} onChange={(e) => setQuery(e.target.value)} />
```

```tspp expected
const node = <Input name="search" value={query} onChange={(e) => setQuery(e.target.value)} />;
```

### expression attribute with call

Complex expression attributes remain wrapped in braces.

```tspp:main.tspp
const node = <div className={cx("a", { b: cond })} />
```

```tspp expected
const node = <div className={cx("a", { b: cond })} />;
```

### template attribute interpolation comments

Template attribute interpolations hug their braces while inner comments stay indented from the template segment.

```tspp:main.tspp
const node = <Panel className={`
  color: ${theme?.activeColor[
    // selected mode
    mode === "dark" ? "dark" : "light"
  ]};
`} />
```

```tspp expected
const node = (
    <Panel
        className={`
  color: ${theme?.activeColor[
      // selected mode
      mode === "dark" ? "dark" : "light"
  ]};
`}
    />
);
```

### attributes with trailing comments

Trailing attribute comments stay attached to the same attributes.

```tspp:main.tspp
const node = <div
  key={formMessageId} // key-tail
  initial={{ opacity: 0, y: -5, height: 0 }} // initial-tail
  style={{ /* before */ overflow: "hidden" /* after */  }}
></div>
```

```tspp expected
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

```tspp:main.tspp
const node = <Widget
  title="Settings" // title-tail
/>
```

```tspp expected
const node = (
    <Widget
        title="Settings" // title-tail
    />
);
```

### tag name comment before attributes

Comments after the tag name stay before the first attribute.

```tspp:main.tspp
const node = <Widget
  /* tag-tail */
  title="Settings"
/>
```

```tspp expected
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

```tspp:main.tspp
const node = <Widget {...props} kind="primary" {...extra} />
```

```tspp expected
const node = <Widget {...props} kind="primary" {...extra} />;
```

### spread attributes with comments

Spread attribute comments stay attached to the same attribute boundaries.

```tspp:main.tspp
const node = <Widget
  // props-leading
  {...props} // props-tail
  kind="primary"
  // extra-leading
  {...extra}
/>
```

```tspp expected
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

```tspp:main.tspp line-width=40
const node = <Panel title="Settings" description="Long description" icon={settingsIcon} />
```

```tspp expected
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

```tspp:main.tspp line-width=45
const node = <Panel options={{ label: "Settings", description: "Long description" }} />
```

```tspp expected
const node = (
    <Panel
        options={{
            label: "Settings",
            description: "Long description",
        }}
    />
);
```

### tree callback attribute breaks around return element

Tree-returning callback attributes break the element and callback body vertically.

```tspp:main.tspp
const node = <List renderItem={(item) => <Item key={item.id}>{item.name}</Item>} />
```

```tspp expected
const node = (
    <List
        renderItem={(item) => (
            <Item key={item.id}>{item.name}</Item>
        )}
    />
);
```
