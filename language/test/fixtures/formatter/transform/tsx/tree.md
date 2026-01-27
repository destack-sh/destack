# TSX Tree Literals

Tests for TSX formatting with tree literal syntax.

## Basic Elements

### simple element with text

Text content stays inline when it fits.

```tsx:main.tsx
const node = <div>Hello</div>
```

```tsx expected
const node = <div>Hello</div>;
```

### text whitespace normalizes

Whitespace in JSX text collapses to single spaces.

```tsx:main.tsx
const node = <div>  Hello   World </div>
```

```tsx expected
const node = <div>Hello World</div>;
```

### whitespace expression containers are preserved

Whitespace expression containers stay as string literals.

```tsx:main.tsx
const node = <div>{" "}Hello{" "}World{" "}</div>
```

```tsx expected
const node = <div>{' '}Hello{' '}World{' '}</div>;
```

## Attributes

### boolean attribute shorthand

Boolean attributes omit `={true}`.

```tsx:main.tsx
const node = <Button disabled={true} primary={true} />
```

```tsx expected
const node = <Button disabled primary />;
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
const node = <>
    <Header />
    <Body />
    <Footer />
</>;
```

## Multiline Elements

### multiline element wraps in parentheses

Multiline JSX in assignments is wrapped in parentheses.

```tsx:main.tsx line-width=30
const node = <Panel title="Settings" description="Long description" />
```

```tsx expected
const node = <Panel
    title="Settings"
    description="Long description"
/>;
```
