# Tree Children

Tree child fixtures cover text, expressions, fragments, and multiline children.

## Text and Expressions

### text with expression stays inline

Text and expression children stay inline when they fit.

```tspp:main.tspp
const node = <div>Hello {name}!</div>
```

```tspp expected
const node = <div>Hello {name}!</div>;
```

### expression child comments

Expression child comments stay in source order around the expression child.

```tspp:main.tspp
const node = <List>{items.map((item) => <Item key={item.id}>{/* before */}{item.label}{/* after */}</Item>)}</List>
```

```tspp expected
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

```tspp:main.tspp
const node = <div>{first}{second}{third}</div>
```

```tspp expected
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

```tspp:main.tspp
const node = <div>  Hello   World </div>
```

```tspp expected
const node = <div> Hello World </div>;
```

### text with embedded expression wraps

Text and expression boundaries break cleanly when they exceed line width.

```tspp:main.tspp line-width=40
const node = <p>Current usage for X is ${(() => {
  // comment
})()}.</p>
```

```tspp expected
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

```tspp:main.tspp line-width=30
const node = <p>Start <b>bold</b>, then stop.</p>
```

```tspp expected
const node = (
    <p>
        Start <b>bold</b>,
        then stop.
    </p>
);
```

### inline prose fills around embedded children

Mixed text, expressions, and tree children wrap as inline prose.

```tspp:main.tspp line-width=45
const node = <p>Hello {name}, see <Link>docs</Link> for details.</p>
```

```tspp expected
const node = (
    <p>
        Hello {name}, see <Link>docs</Link> for
        details.
    </p>
);
```

### inline prose keeps punctuation with embedded expression

Punctuation between expression and following text stays attached to the expression.

```tspp:main.tspp line-width=35
const node = <p>Hello {name}, welcome back.</p>
```

```tspp expected
const node = (
    <p>
        Hello {name}, welcome back.
    </p>
);
```

### inline prose keeps source newline before punctuation

Punctuation with leading source newline remains its own text line.

```tspp:main.tspp
const node = <p>{value}
.</p>
```

```tspp expected
const node = (
    <p>
        {value}
        .
    </p>
);
```

### inline prose keeps source space before punctuation

Punctuation with leading source space remains separated from the previous expression.

```tspp:main.tspp
const node = <p>Hello {name} .</p>
```

```tspp expected
const node = <p>Hello {name} .</p>;
```

### inline prose keeps whitespace expression before punctuation

Punctuation after an explicit tree whitespace container remains separated from the previous expression.

```tspp:main.tspp
const node = <p>Hello {name}{" "}.</p>
```

```tspp expected
const node = <p>Hello {name} .</p>;
```

### inline prose keeps punctuation runs attached

Punctuation runs stay attached to the previous inline expression before wrapping prose.

```tspp:main.tspp line-width=30
const node = <p>Hello {name}?! Really...</p>
```

```tspp expected
const node = (
    <p>
        Hello {name}?!
        Really...
    </p>
);
```

### inline prose keeps punctuation with fragment child

Punctuation after a fragment child stays attached to the fragment.

```tspp:main.tspp line-width=35
const node = <p>Start <>{value}</>, done.</p>
```

```tspp expected
const node = (
    <p>
        Start <>{value}</>, done.
    </p>
);
```

### inline prose keeps punctuation after forced child break

Punctuation after a multiline expression child stays attached when attributes force multiline layout.

```tspp:main.tspp line-width=40
const node = <p title="Long title value" description="Long description value">{value}.</p>
```

```tspp expected
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

```tspp:main.tspp
const node = <p>{/* keep */}.</p>
```

```tspp expected
const node = (
    <p>
        {/* keep */}
        .
    </p>
);
```

### template child interpolation comments

Template child interpolations hug their braces while inner comments stay indented from the template segment.

```tspp:main.tspp
const node = <div>{`
  color: ${theme?.activeColor[
    // selected mode
    mode === "dark" ? "dark" : "light"
  ]};
`}</div>
```

```tspp expected
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

```tspp:main.tspp
const node = <section><Header /><Body /><Footer /></section>
```

```tspp expected
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

```tspp:main.tspp
const node = <div>{label}<Icon />{suffix}</div>
```

```tspp expected
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

```tspp:main.tspp
const node = <T>
  Pro tip: See more{" "}
  <Link href="https://example.com">Docs</Link>{" "}
  for details.
</T>
```

```tspp expected
const node = (
    <T>
        Pro tip: See more <Link href="https://example.com">Docs</Link>
        for details.
    </T>
);
```

### text with inline elements breaks into lines

Inline elements inside text blocks use fill layout across lines.

```tspp:main.tspp
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

```tspp expected
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

```tspp:main.tspp line-width=50
const node = <ul>{items.map((item) => <li key={item.id}>{item.name}</li>)}</ul>
```

```tspp expected
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

```tspp:main.tspp line-width=20
const node = <div>{ready ? <Ready /> : <Pending />}</div>
```

```tspp expected
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

```tspp
const node = <Panel>{if (ready) { <Ready label={`state: ${readyLabel}`} /> } else {
  // pending branch
  <Pending label={`state: ${pendingLabel}`} />
}}</Panel>
```

```tspp expected
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

```tspp line-width=80
const node = <Panel>{if (let Some(item) = selected) { <Ready item={item} /> } else { <Pending /> }}</Panel>
```

```tspp expected
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

```tspp line-width=80
const node = <Panel>{if (ready) { label } else { fallback }}</Panel>
```

```tspp expected
const node = <Panel>{if (ready) { label } else { fallback }}</Panel>;
```

### if value with condition comment

Line comments in control heads expand the expression child.

```tspp line-width=80
const node = <Panel>{if (
  // ready state
  ready
) { <Ready /> } else { <Pending /> }}</Panel>
```

```tspp expected
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

```tspp
const node = <Panel>{match (state) { Ready(item) => <Ready item={item} />; Pending => <Pending />; Failed(error) => <Failed error={error} /> }}</Panel>
```

```tspp expected
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

```tspp
const node = <Panel>{match (state) { Ready(item) => <Ready item={item} />; Empty => "empty"; Failed(error) => <Failed error={error} /> }}</Panel>
```

```tspp expected
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

```tspp
const node = <Panel>{try { <Ready data={load()} /> } catch (error) { <Failed error={error} /> }}</Panel>
```

```tspp expected
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

```tspp
const node = <Panel>{try { value } catch (error) { fallback } finally { <Cleanup /> }}</Panel>
```

```tspp expected
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

```tspp:main.tspp line-width=80
xxxxxxxxxxxx === "xxxxxxxxxxxxxxxxx" && (
  // test
  <div></div>
)
```

```tspp expected
xxxxxxxxxxxx === "xxxxxxxxxxxxxxxxx"
    && (
        // test
        <div></div>
    );
```

## Fragments

### fragment with multiple children

Fragments with multiple children break across lines.

```tspp:main.tspp
const node = <><A /><B /><C /></>
```

```tspp expected
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

```tspp
<Text>Hello World</Text>
```

```tspp expected
<Text>Hello World</Text>;
```

### text whitespace normalizes

Text nodes collapse whitespace to single spaces.

```tspp
<Text>  Hello   World </Text>
```

```tspp expected
<Text> Hello World </Text>;
```

### whitespace-only text is preserved as a single space

Whitespace-only text normalizes to a single preserved space.

```tspp
<Text> </Text>
```

```tspp expected
<Text> </Text>;
```

### whitespace expression container normalizes

Whitespace expression containers normalize to inline text spacing.

```tspp
<Text>{" "}Hello{" "}World{" "}</Text>
```

```tspp expected
<Text> Hello World </Text>;
```

### element with element children

Elements can contain other elements.

```tspp
<Container><Header /><Content /></Container>
```

```tspp expected
<Container>
    <Header />
    <Content />
</Container>;
```

### nested elements

Deeply nested elements expand with stable indentation.

```tspp line-width=40
<Outer><Middle><Inner>content</Inner></Middle></Outer>
```

```tspp expected
<Outer>
    <Middle>
        <Inner>content</Inner>
    </Middle>
</Outer>;
```

### mixed children

Elements with mixed text and element children expand around fill-layout children.

```tspp
<Paragraph>Hello <Strong>World</Strong>!</Paragraph>
```

```tspp expected
<Paragraph>Hello <Strong>World</Strong>!</Paragraph>;
```

## Child Patterns

### conditional rendering

Ternary expressions work inside tree elements.

```tspp
<Container>{isOpen ? <Panel /> : <Placeholder />}</Container>
```

```tspp expected
<Container>{isOpen ? <Panel /> : <Placeholder />}</Container>;
```

### spread attributes

Spread operator passes all properties from an object.

```tspp
<Button {...props} extra="value" />
```

```tspp expected
<Button {...props} extra="value" />;
```

### component with callback

Callbacks can be passed as attributes.

```tspp
<Button onClick={(e) => handleClick(e)} />
```

```tspp expected
<Button onClick={(e) => handleClick(e)} />;
```

### fragments

Fragment syntax groups elements without a wrapper.

```tspp
<><Header /><Content /><Footer /></>
```

```tspp expected
<>
    <Header />
    <Content />
    <Footer />
</>;
```

## Callback Children

### map expression in children

Tree-returning callbacks in tree children break vertically.

```tspp line-width=50
<List>{items.map((item) => <Item key={item.id} />)}</List>
```

```tspp expected
<List>
    {items.map((item) => (
        <Item key={item.id} />
    ))}
</List>;
```

### long map with block body

Map with block body breaks.
Returned tree literals use parentheses when multiline.

```tspp line-width=40
<List>{items.map((item) => { return <Item key={item.id} name={item.name} /> })}</List>
```

```tspp expected
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

```tspp
<div>{loading && <Spinner />}</div>
```

```tspp expected
<div>{loading && <Spinner />}</div>;
```

### ternary with complex tree branches

Ternary with multi-attribute tree in branches.

```tspp line-width=50
<div>{loading ? <Spinner size="large" /> : <Content data={data} />}</div>
```

```tspp expected
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

```tspp line-width=40
<div>{isVideo ? <Video /> /* keep-video */ : <Image /> /* keep-image */}</div>
```

```tspp expected
<div>
    {isVideo ? (
        <Video /> /* keep-video */
    ) : (
        <Image /> /* keep-image */
    )}
</div>;
```
