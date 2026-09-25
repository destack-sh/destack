# Tree Returns

## Return Statements

### return tree inline

Short tree literals in return statements stay inline.

```tspp
return <App />
```

```tspp expected
return <App />;
```

### return tree multiline gets wrapped

Multiline returned tree literals use parentheses.

```tspp line-width=30
return <App prop="value" another="thing" />
```

```tspp expected
return (
    <App
        prop="value"
        another="thing"
    />
);
```

### return nested tree wrapped

Nested returned tree literals also use parentheses.

```tspp line-width=40
return <Container><Header /><Content /></Container>
```

```tspp expected
return (
    <Container>
        <Header />
        <Content />
    </Container>
);
```
