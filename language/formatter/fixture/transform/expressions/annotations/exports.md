# Export Annotations

## Export and Interface Annotation Boundaries

### export declaration with boundary comments

Boundary comments around export declaration heads stay attached to the exported node.

```tspp:main.tspp
export // export-head
interface Shape {
  value: string // value-tail
}
```

```tspp expected
export // export-head
interface Shape {
    value: string; // value-tail
}
```

### interface location comment boundaries

Comments around interface property and method type boundaries stay attached.

```tspp:main.tspp
interface Api {
  url: string // url-tail
  run(): // run-ret
  Promise<void>
}
```

```tspp expected
interface Api {
    url: string; // url-tail
    run(): // run-ret
    Promise<void>;
}
```
