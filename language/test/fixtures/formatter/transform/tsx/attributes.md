# TSX Attributes

Tests for TSX attribute formatting and layout behavior.

## Basic Attributes

### spacing and boolean shorthand

Attribute spacing is normalized and boolean attributes use shorthand.

```tsx:main.tsx
const node = <Button disabled={true} count={ 1 } label="Ok" />
```

```tsx expected
const node = <Button disabled count={1} label="Ok" />;
```

### string expression attribute collapses

String literal expression containers collapse to plain string attributes.

```tsx:main.tsx
const node = <div title={"Hello"} className={'card'} />
```

```tsx expected
const node = <div title="Hello" className="card" />;
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
const node = <div className={cx('a', { b: cond })} />;
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

## Line Breaking

### attributes break when too long

Long attribute lists break across lines.

```tsx:main.tsx line-width=40
const node = <Panel title="Settings" description="Long description" icon={settingsIcon} />
```

```tsx expected
const node = <Panel
    title="Settings"
    description="Long description"
    icon={settingsIcon}
/>;
```

### bracket same line option

When `bracket_same_line` is true, the closing bracket stays on the last line.

```tsx:main.tsx line-width=40 bracket-same-line=true
const node = <Panel title="Settings" description="Long description" icon={settingsIcon} />
```

```tsx expected
const node = <Panel
    title="Settings"
    description="Long description"
    icon={settingsIcon} />;
```

### single attribute per line option

When `single_attribute_per_line` is true, each attribute is on its own line.

```tsx:main.tsx single-attribute-per-line=true
const node = <Button variant="primary" size="large" disabled />
```

```tsx expected
const node = <Button
    variant="primary"
    size="large"
    disabled
/>;
```
