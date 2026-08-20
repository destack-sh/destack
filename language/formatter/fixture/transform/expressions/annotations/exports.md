# Export Annotations

## Export and Interface Annotation Boundaries

### export declaration with boundary comments

Boundary comments around export declaration heads stay attached to the exported node.

```ds:main.ds
export // export-head
interface Shape {
  value: string // value-tail
}
```

```ds expected
export // export-head
interface Shape {
    value: string; // value-tail
}
```

### interface location comment boundaries

Comments around interface property and method type boundaries stay attached.

```ds:main.ds
interface Api {
  url: string // url-tail
  run(): // run-ret
  Promise<void>
}
```

```ds expected
interface Api {
    url: string; // url-tail
    run(): // run-ret
    Promise<void>;
}
```
