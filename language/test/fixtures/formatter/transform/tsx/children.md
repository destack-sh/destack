# TSX Children

TSX child fixtures cover text, expression children, fragments, and multiline child layout.

## Text and Expressions

### text with expression stays inline

Text and expression children stay inline when they fit.

```tsx:main.tsx
const node = <div>Hello {name}!</div>
```

```tsx expected
const node = <div>Hello {name}!</div>;
```

### expression child comments

Expression child comments stay in source order around the expression child.

```tsx:main.tsx
const node = <List>{items.map((item) => <Item key={item.id}>{/* before */}{item.label}{/* after */}</Item>)}</List>
```

```tsx expected
const node = (
    <List>
        {items.map((item) => (
            <Item key={item.id}>
                {/* before */}
                {item.label}
                {/* after */}
            </Item>
        ))}
    </List>
);
```

### adjacent expression children stay inline

Adjacent expression children stay inline when short.

```tsx:main.tsx
const node = <div>{first}{second}{third}</div>
```

```tsx expected
const node = (
    <div>
        {first}
        {second}
        {third}
    </div>
);
```

### whitespace normalizes in text

Whitespace inside text nodes collapses to single spaces.

```tsx:main.tsx
const node = <div>  Hello   World </div>
```

```tsx expected
const node = <div> Hello World </div>;
```

### text with embedded expression wraps

Text and expression boundaries break cleanly when they exceed line width.

```tsx:main.tsx line-width=40
const node = <p>Current usage for X is ${(() => {
  // comment
})()}.</p>
```

```tsx expected
const node = (
    <p>
        Current usage for X is $
        {(() => {
            // comment
        })()}
        .
    </p>
);
```

### template child interpolation comments

Template children keep multiline interpolation comments indented from the template segment.

```tsx:main.tsx
const node = <div>{`
  color: ${theme?.activeColor[
    // selected mode
    mode === "dark" ? "dark" : "light"
  ]};
`}</div>
```

```tsx expected
const node = (
    <div>{`
  color: ${
      theme?.activeColor[
          // selected mode
          mode === "dark" ? "dark" : "light"
      ]
  };
`}</div>
);
```

### multiline children break

Multiple element children break to one per line.

```tsx:main.tsx
const node = <section><Header /><Body /><Footer /></section>
```

```tsx expected
const node = (
    <section>
        <Header />
        <Body />
        <Footer />
    </section>
);
```

### mixed children break when tree literals appear

Tree literal children force multiline formatting.

```tsx:main.tsx
const node = <div>{label}<Icon />{suffix}</div>
```

```tsx expected
const node = (
    <div>
        {label}
        <Icon />
        {suffix}
    </div>
);
```

### mixed text with spaced expressions

Text nodes keep explicit space expression containers.

```tsx:main.tsx
const node = <T>
  Pro tip: See more{' '}
  <Link href="https://example.com">Docs</Link>{' '}
  for details.
</T>
```

```tsx expected
const node = (
    <T>
        Pro tip: See more <Link href="https://example.com">Docs</Link> for details.
    </T>
);
```

### text with inline elements breaks into lines

Inline elements inside text blocks break into separate lines.

```tsx:main.tsx
export default function ProTip() {
  return (
    <T>
      <X />
      Pro tip: See more <Link href="https://mui.com/getting-started/templates/">
        BREAK THIS
      </Link> on
      the MUI documentation.
    </T>
  );
}
```

```tsx expected
export default function ProTip() {
    return (
        <T>
            <X />
            Pro tip: See more{" "}
            <Link href="https://mui.com/getting-started/templates/">BREAK THIS</Link> on the MUI
            documentation.
        </T>
    );
}
```

## Expression Children

### map expression with element return

Inline map expressions break when they exceed line width.

```tsx:main.tsx line-width=50
const node = <ul>{items.map((item) => <li key={item.id}>{item.name}</li>)}</ul>
```

```tsx expected
const node = (
    <ul>
        {items.map((item) => (
            <li key={item.id}>{item.name}</li>
        ))}
    </ul>
);
```

### conditional expression child

Conditional expression children break with aligned operators.

```tsx:main.tsx line-width=20
const node = <div>{ready ? <Ready /> : <Pending />}</div>
```

```tsx expected
const node = (
    <div>
        {ready ? (
            <Ready />
        ) : (
            <Pending />
        )}
    </div>
);
```

### if value with element children

If values used as children expand element branches.

```ds
const node = <Panel>{if (ready) { <Ready label={`state: ${readyLabel}`} /> } else {
  // pending branch
  <Pending label={`state: ${pendingLabel}`} />
}}</Panel>
```

```ds expected
const node = (
    <Panel>
        {if (ready) {
            <Ready label={`state: ${readyLabel}`} />
        } else {
            // pending branch
            <Pending label={`state: ${pendingLabel}`} />
        }}
    </Panel>
);
```

### if-let value with element children

If-let values used as children expand element branches and pattern heads.

```ds line-width=80
const node = <Panel>{if (let Some(item) = selected) { <Ready item={item} /> } else { <Pending /> }}</Panel>
```

```ds expected
const node = (
    <Panel>
        {if (let Some(item) = selected) {
            <Ready item={item} />
        } else {
            <Pending />
        }}
    </Panel>
);
```

### match value with element children

Match values used as children keep each element arm attached to its pattern.

```ds
const node = <Panel>{match (state) { Ready(item) => <Ready item={item} />; Pending => <Pending />; Failed(error) => <Failed error={error} /> }}</Panel>
```

```ds expected
const node = (
    <Panel>
        {match (state) {
            Ready(item) => <Ready item={item} />
            Pending => <Pending />
            Failed(error) => <Failed error={error} />
        }}
    </Panel>
);
```

### try value with element children

Try values used as children expand result and recovery element branches.

```ds
const node = <Panel>{try { <Ready data={load()} /> } catch (error) { <Failed error={error} /> }}</Panel>
```

```ds expected
const node = (
    <Panel>
        {try {
            <Ready data={load()} />
        } catch (error) {
            <Failed error={error} />
        }}
    </Panel>
);
```

### logical expression with jsx child

Logical expressions keep JSX children grouped with comments.

```tsx:main.tsx line-width=80
xxxxxxxxxxxx === "xxxxxxxxxxxxxxxxx" && (
  // test
  <div></div>
)
```

```tsx expected
xxxxxxxxxxxx === "xxxxxxxxxxxxxxxxx" && (
    // test
    <div></div>
);
```

## Fragments

### fragment with multiple children

Fragments with multiple children break across lines.

```tsx:main.tsx
const node = <><A /><B /><C /></>
```

```tsx expected
const node = (
    <>
        <A />
        <B />
        <C />
    </>
);
```
