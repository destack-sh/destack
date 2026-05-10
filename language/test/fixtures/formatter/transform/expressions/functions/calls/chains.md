# Method Chains

## Method Chains

### short chains stay on one line

Short method chains remain on a single line.

```ds
foo().bar().baz()
```

```ds expected
foo().bar().baz();
```

### long chain breaks at each method

When chains exceed line width, each method gets its own line with semicolon on last line.

```ds line-width=30
data.filter(x => x.active).map(x => x.name).join(", ")
```

```ds expected
data.filter((x) => x.active)
    .map((x) => x.name)
    .join(", ");
```

### promise chain

Promise chains break nicely across lines with semicolon on last line.

```ds line-width=40
fetch(url).then(r => r.json()).then(data => process(data)).catch(handleError)
```

```ds expected
fetch(url)
    .then((r) => r.json())
    .then((data) => process(data))
    .catch(handleError);
```

### chain with mixed access

Method and property access can mix in chains.

```ds
obj.items.filter(x => x.valid).length
```

```ds expected
obj.items.filter((x) => x.valid).length;
```

### nested chains

Nested method chains format correctly.

```ds
outer.map(x => x.inner.filter(y => y.ok))
```

```ds expected
outer.map((x) => x.inner.filter((y) => y.ok));
```
