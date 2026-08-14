# Tree Children

Tree child fixtures cover text, expressions, fragments, and multiline children.

## Text and Expressions

### text with expression stays inline

Text and expression children stay inline when they fit.

```ds:main.ds
const node = <div>Hello {name}!</div>
```

```ds expected
const node = <div>Hello {name}!</div>;
```

### expression child comments

Expression child comments stay in source order around the expression child.

```ds:main.ds
const node = <List>{items.map((item) => <Item key={item.id}>{/* before */}{item.label}{/* after */}</Item>)}</List>
```

```ds expected
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

### adjacent expression children break

Adjacent expression children break to one expression container per line.

```ds:main.ds
const node = <div>{first}{second}{third}</div>
```

```ds expected
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

```ds:main.ds
const node = <div>  Hello   World </div>
```

```ds expected
const node = <div> Hello World </div>;
```

### text with embedded expression wraps

Text and expression boundaries break cleanly when they exceed line width.

```ds:main.ds line-width=40
const node = <p>Current usage for X is ${(() => {
  // comment
})()}.</p>
```

```ds expected
const node = (
    <p>
        Current usage for X is $
        {(() => {
            // comment
        })()}.
    </p>
);
```

### text punctuation attaches to element children

Punctuation-only text after a wrapped child stays attached to the child.

```ds:main.ds line-width=30
const node = <p>Start <b>bold</b>, then stop.</p>
```

```ds expected
const node = (
    <p>
        Start <b>bold</b>,
        then stop.
    </p>
);
```

### inline prose fills around embedded children

Mixed text, expressions, and tree children wrap as inline prose.

```ds:main.ds line-width=45
const node = <p>Hello {name}, see <Link>docs</Link> for details.</p>
```

```ds expected
const node = (
    <p>
        Hello {name}, see <Link>docs</Link> for
        details.
    </p>
);
```

### inline prose keeps punctuation with embedded expression

Punctuation between expression and following text stays attached to the expression.

```ds:main.ds line-width=35
const node = <p>Hello {name}, welcome back.</p>
```

```ds expected
const node = (
    <p>
        Hello {name}, welcome back.
    </p>
);
```

### inline prose keeps source newline before punctuation

Punctuation with leading source newline remains its own text line.

```ds:main.ds
const node = <p>{value}
.</p>
```

```ds expected
const node = (
    <p>
        {value}
        .
    </p>
);
```

### inline prose keeps source space before punctuation

Punctuation with leading source space remains separated from the previous expression.

```ds:main.ds
const node = <p>Hello {name} .</p>
```

```ds expected
const node = <p>Hello {name} .</p>;
```

### inline prose keeps whitespace expression before punctuation

Punctuation after an explicit tree whitespace container remains separated from the previous expression.

```ds:main.ds
const node = <p>Hello {name}{" "}.</p>
```

```ds expected
const node = <p>Hello {name} .</p>;
```

### inline prose keeps punctuation runs attached

Punctuation runs stay attached to the previous inline expression before wrapping prose.

```ds:main.ds line-width=30
const node = <p>Hello {name}?! Really...</p>
```

```ds expected
const node = (
    <p>
        Hello {name}?!
        Really...
    </p>
);
```

### inline prose keeps punctuation with fragment child

Punctuation after a fragment child stays attached to the fragment.

```ds:main.ds line-width=35
const node = <p>Start <>{value}</>, done.</p>
```

```ds expected
const node = (
    <p>
        Start <>{value}</>, done.
    </p>
);
```

### inline prose keeps punctuation after forced child break

Punctuation after a multiline expression child stays attached when attributes force multiline layout.

```ds:main.ds line-width=40
const node = <p title="Long title value" description="Long description value">{value}.</p>
```

```ds expected
const node = (
    <p
        title="Long title value"
        description="Long description value"
    >
        {value}.
    </p>
);
```

### inline prose keeps comment boundary before punctuation

Punctuation after a tree comment remains its own text child.

```ds:main.ds
const node = <p>{/* keep */}.</p>
```

```ds expected
const node = (
    <p>
        {/* keep */}
        .
    </p>
);
```

### template child interpolation comments

Template child interpolations hug their braces while inner comments stay indented from the template segment.

```ds:main.ds
const node = <div>{`
  color: ${theme?.activeColor[
    // selected mode
    mode === "dark" ? "dark" : "light"
  ]};
`}</div>
```

```ds expected
const node = (
    <div>{`
  color: ${theme?.activeColor[
      // selected mode
      mode === "dark" ? "dark" : "light"
  ]};
`}</div>
);
```

### multiline children break

Multiple element children break to one per line.

```ds:main.ds
const node = <section><Header /><Body /><Footer /></section>
```

```ds expected
const node = (
    <section>
        <Header />
        <Body />
        <Footer />
    </section>
);
```

### mixed children break when tree literals appear

A tree literal between expression children forces multiline formatting.

```ds:main.ds
const node = <div>{label}<Icon />{suffix}</div>
```

```ds expected
const node = (
    <div>
        {label}
        <Icon />
        {suffix}
    </div>
);
```

### mixed text with spaced expressions

Whitespace expression containers become tree text spacing in inline prose.

```ds:main.ds
const node = <T>
  Pro tip: See more{" "}
  <Link href="https://example.com">Docs</Link>{" "}
  for details.
</T>
```

```ds expected
const node = (
    <T>
        Pro tip: See more <Link href="https://example.com">Docs</Link>
        for details.
    </T>
);
```

### text with inline elements breaks into lines

Inline elements inside text blocks use fill layout across lines.

```ds:main.ds
export default function ProTip() {
  return (
    <T>
      <X />
      Pro tip: See more <Link href="https://mui.com/getting-started/templates/">
        BREAK THIS
      </Link>{" "}
      on
      the MUI documentation.
    </T>
  );
}
```

```ds expected
export default function ProTip() {
    return (
        <T>
            <X />
            Pro tip: See more{" "}
            <Link href="https://mui.com/getting-started/templates/">
                BREAK THIS
            </Link>{" "}
            on the MUI documentation.
        </T>
    );
}
```

## Expression Children

### map expression with element return

Inline map expressions break when they exceed line width.

```ds:main.ds line-width=50
const node = <ul>{items.map((item) => <li key={item.id}>{item.name}</li>)}</ul>
```

```ds expected
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

```ds:main.ds line-width=20
const node = <div>{ready ? <Ready /> : <Pending />}</div>
```

```ds expected
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

### if value with scalar children

If values without tree branches stay inline when they fit.

```ds line-width=80
const node = <Panel>{if (ready) { label } else { fallback }}</Panel>
```

```ds expected
const node = <Panel>{if (ready) { label } else { fallback }}</Panel>;
```

### if value with condition comment

Line comments in control heads expand the expression child.

```ds line-width=80
const node = <Panel>{if (
  // ready state
  ready
) { <Ready /> } else { <Pending /> }}</Panel>
```

```ds expected
const node = (
    <Panel>
        {if (
            // ready state
            ready
        ) {
            <Ready />
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

### match value with mixed branches

Match values expand when any branch returns a tree.

```ds
const node = <Panel>{match (state) { Ready(item) => <Ready item={item} />; Empty => "empty"; Failed(error) => <Failed error={error} /> }}</Panel>
```

```ds expected
const node = (
    <Panel>
        {match (state) {
            Ready(item) => <Ready item={item} />
            Empty => "empty"
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

### try value with finally tree branch

Try values expand when the finally branch returns a tree.

```ds
const node = <Panel>{try { value } catch (error) { fallback } finally { <Cleanup /> }}</Panel>
```

```ds expected
const node = (
    <Panel>
        {try {
            value
        } catch (error) {
            fallback
        } finally {
            <Cleanup />
        }}
    </Panel>
);
```

### logical expression with tree child

Logical expressions keep tree children grouped with comments.

```ds:main.ds line-width=80
xxxxxxxxxxxx === "xxxxxxxxxxxxxxxxx" && (
  // test
  <div></div>
)
```

```ds expected
xxxxxxxxxxxx === "xxxxxxxxxxxxxxxxx" && (
    // test
    <div></div>
);
```

## Fragments

### fragment with multiple children

Fragments with multiple children break across lines.

```ds:main.ds
const node = <><A /><B /><C /></>
```

```ds expected
const node = (
    <>
        <A />
        <B />
        <C />
    </>
);
```

## Elements with Children

### element with text child

Elements can contain text content.

```ds
<Text>Hello World</Text>
```

```ds expected
<Text>Hello World</Text>;
```

### text whitespace normalizes

Text nodes collapse whitespace to single spaces.

```ds
<Text>  Hello   World </Text>
```

```ds expected
<Text> Hello World </Text>;
```

### whitespace-only text is preserved as a single space

Whitespace-only text normalizes to a single preserved space.

```ds
<Text> </Text>
```

```ds expected
<Text> </Text>;
```

### whitespace expression container normalizes

Whitespace expression containers normalize to inline text spacing.

```ds
<Text>{" "}Hello{" "}World{" "}</Text>
```

```ds expected
<Text> Hello World </Text>;
```

### element with element children

Elements can contain other elements.

```ds
<Container><Header /><Content /></Container>
```

```ds expected
<Container>
    <Header />
    <Content />
</Container>;
```

### nested elements

Deeply nested elements expand with stable indentation.

```ds line-width=40
<Outer><Middle><Inner>content</Inner></Middle></Outer>
```

```ds expected
<Outer>
    <Middle>
        <Inner>content</Inner>
    </Middle>
</Outer>;
```

### mixed children

Elements with mixed text and element children expand around fill-layout children.

```ds
<Paragraph>Hello <Strong>World</Strong>!</Paragraph>
```

```ds expected
<Paragraph>Hello <Strong>World</Strong>!</Paragraph>;
```

## Child Patterns

### conditional rendering

Ternary expressions work inside tree elements.

```ds
<Container>{isOpen ? <Panel /> : <Placeholder />}</Container>
```

```ds expected
<Container>{isOpen ? <Panel /> : <Placeholder />}</Container>;
```

### spread attributes

Spread operator passes all properties from an object.

```ds
<Button {...props} extra="value" />
```

```ds expected
<Button {...props} extra="value" />;
```

### component with callback

Callbacks can be passed as attributes.

```ds
<Button onClick={(e) => handleClick(e)} />
```

```ds expected
<Button onClick={(e) => handleClick(e)} />;
```

### fragments

Fragment syntax groups elements without a wrapper.

```ds
<><Header /><Content /><Footer /></>
```

```ds expected
<>
    <Header />
    <Content />
    <Footer />
</>;
```

## Callback Children

### map expression in children

Tree-returning callbacks in tree children break vertically.

```ds line-width=50
<List>{items.map((item) => <Item key={item.id} />)}</List>
```

```ds expected
<List>
    {items.map((item) => (
        <Item key={item.id} />
    ))}
</List>;
```

### long map with block body

Map with block body breaks.
Returned tree literals use parentheses when multiline.

```ds line-width=40
<List>{items.map((item) => { return <Item key={item.id} name={item.name} /> })}</List>
```

```ds expected
<List>
    {items.map((item) => {
        return (
            <Item
                key={item.id}
                name={item.name}
            />
        );
    })}
</List>;
```

### conditional with tree

Conditional expressions with tree children.

```ds
<div>{loading && <Spinner />}</div>
```

```ds expected
<div>{loading && <Spinner />}</div>;
```

### ternary with complex tree branches

Ternary with multi-attribute tree in branches.

```ds line-width=50
<div>{loading ? <Spinner size="large" /> : <Content data={data} />}</div>
```

```ds expected
<div>
    {loading ? (
        <Spinner size="large" />
    ) : (
        <Content data={data} />
    )}
</div>;
```

### ternary branch comments

Trailing tree branch comments format at conditional branch boundaries.

```ds line-width=40
<div>{isVideo ? <Video /> /* keep-video */ : <Image /> /* keep-image */}</div>
```

```ds expected
<div>
    {isVideo ? (
        <Video /> /* keep-video */
    ) : (
        <Image /> /* keep-image */
    )}
</div>;
```
