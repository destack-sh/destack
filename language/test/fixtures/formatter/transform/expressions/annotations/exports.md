# Export Annotations

## Export and Interface Annotation Boundaries

### export declaration with boundary comments

Boundary comments around export declaration heads stay attached to the exported node.

```ts:main.ts
export // export-head
interface Shape {
  value: string // value-tail
}
```

```ts expected
export // export-head
interface Shape {
    value: string; // value-tail
}
```

### interface location comment boundaries

Comments around interface property and method type boundaries stay attached.

```ts:main.ts
interface Api {
  url: string // url-tail
  run(): // run-ret
  Promise<void>
}
```

```ts expected
interface Api {
    url: string; // url-tail
    run(): // run-ret
    Promise<void>;
}
```
