# Tree Returns

## Return Statements with JSX

### return jsx inline

Short JSX in return statements stays inline.

```ds
return <App />
```

```ds expected
return <App />;
```

### return jsx multiline gets wrapped

When JSX in return breaks, it gets wrapped in parentheses.

```ds line-width=30
return <App prop="value" another="thing" />
```

```ds expected
return (
    <App
        prop="value"
        another="thing"
    />
);
```

### return nested jsx wrapped

Nested JSX in returns also gets wrapped.

```ds line-width=40
return <Container><Header /><Content /></Container>
```

```ds expected
return (
    <Container>
        <Header />
        <Content />
    </Container>
);
```
