# Method Chains

## Method Chains

### short chains stay on one line

Short method chains remain on a single line.

```tspp
foo().bar().baz()
```

```tspp expected
foo().bar().baz();
```

### long chain breaks at each method

When chains exceed line width, each method gets its own line with semicolon on last line.

```tspp line-width=30
data.filter(x => x.active).map(x => x.name).join(", ")
```

```tspp expected
data.filter((x) => x.active)
    .map((x) => x.name)
    .join(", ");
```

### promise chain

Promise chains break nicely across lines with semicolon on last line.

```tspp line-width=40
fetch(url).then(r => r.json()).then(data => process(data)).catch(handleError)
```

```tspp expected
fetch(url)
    .then((r) => r.json())
    .then((data) => process(data))
    .catch(handleError);
```

### chain with mixed access

Method and property access can mix in chains.

```tspp
obj.items.filter(x => x.valid).length
```

```tspp expected
obj.items.filter((x) => x.valid).length;
```

### nested chains

Nested method chains format correctly.

```tspp
outer.map(x => x.inner.filter(y => y.ok))
```

```tspp expected
outer.map((x) => x.inner.filter((y) => y.ok));
```
