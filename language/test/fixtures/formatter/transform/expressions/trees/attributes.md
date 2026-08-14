# Tree Attributes

Tree attribute fixtures cover spacing, expressions, comments, spread attributes, and wrapping.

## Attribute Forms

### spacing and boolean expression attributes

Attribute spacing is normalized without rewriting expression attribute values.

```ds:main.ds
const node = <Button disabled={true} count={ 1 } label="Ok" />
```

```ds expected
const node = <Button disabled={true} count={1} label="Ok" />;
```

### string expression attributes

String literal expression containers keep their braces.

```ds:main.ds
const node = <div title={"Hello"} className={"card"} />
```

```ds expected
const node = <div title={"Hello"} className={"card"} />;
```

### mixed attribute kinds

Mixed attribute types stay ordered and normalized.

```ds:main.ds
const node = <Input name="search" value={query} onChange={(e) => setQuery(e.target.value)} />
```

```ds expected
const node = <Input name="search" value={query} onChange={(e) => setQuery(e.target.value)} />;
```

### expression attribute with call

Complex expression attributes remain wrapped in braces.

```ds:main.ds
const node = <div className={cx("a", { b: cond })} />
```

```ds expected
const node = <div className={cx("a", { b: cond })} />;
```

### template attribute interpolation comments

Template attribute interpolations hug their braces while inner comments stay indented from the template segment.

```ds:main.ds
const node = <Panel className={`
  color: ${theme?.activeColor[
    // selected mode
    mode === "dark" ? "dark" : "light"
  ]};
`} />
```

```ds expected
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

```ds:main.ds
const node = <div
  key={formMessageId} // key-tail
  initial={{ opacity: 0, y: -5, height: 0 }} // initial-tail
  style={{ /* before */ overflow: "hidden" /* after */  }}
></div>
```

```ds expected
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

```ds:main.ds
const node = <Widget
  title="Settings" // title-tail
/>
```

```ds expected
const node = (
    <Widget
        title="Settings" // title-tail
    />
);
```

### tag name comment before attributes

Comments after the tag name stay before the first attribute.

```ds:main.ds
const node = <Widget
  /* tag-tail */
  title="Settings"
/>
```

```ds expected
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

```ds:main.ds
const node = <Widget {...props} kind="primary" {...extra} />
```

```ds expected
const node = <Widget {...props} kind="primary" {...extra} />;
```

### spread attributes with comments

Spread attribute comments stay attached to the same attribute boundaries.

```ds:main.ds
const node = <Widget
  // props-leading
  {...props} // props-tail
  kind="primary"
  // extra-leading
  {...extra}
/>
```

```ds expected
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

```ds:main.ds line-width=40
const node = <Panel title="Settings" description="Long description" icon={settingsIcon} />
```

```ds expected
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

```ds:main.ds line-width=45
const node = <Panel options={{ label: "Settings", description: "Long description" }} />
```

```ds expected
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

```ds:main.ds
const node = <List renderItem={(item) => <Item key={item.id}>{item.name}</Item>} />
```

```ds expected
const node = (
    <List
        renderItem={(item) => (
            <Item key={item.id}>{item.name}</Item>
        )}
    />
);
```
