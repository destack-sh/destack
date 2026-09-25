# Type Template Literals

## Type Template Literals

### template literal type breaks after assignment

Template literal types stay inline with `=` at wider line widths.

```tspp:main.tspp
type templateLiteralType = `${
  TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? "_"
    : ""
}`;
```

```tspp expected
type templateLiteralType = `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? "_"
    : ""}`;
```

### template literal type breaks after assignment at narrower widths

Template literal types break after `=` when the configured line width is narrower.

```tspp:main.tspp line-width=80
type templateLiteralType = `${
  TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? "_"
    : ""
}`;
```

```tspp expected
type templateLiteralType =
    `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
        ? "_"
        : ""}`;
```

### template literal type with nested conditionals

Nested template literal types break with stable indentation.

```tspp:main.tspp line-width=80
type CamelToSnakeCase<TCamelCaseString: string> =
  TCamelCaseString extends `${infer TStringConvertedSoFar}${infer TStringYetToConvert}`
    ? `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
        ? "_"
        : ""}${Lowercase<TStringConvertedSoFar>}${CamelToSnakeCase<TStringYetToConvert>}`
    : TCamelCaseString
```

```tspp expected
type CamelToSnakeCase<TCamelCaseString: string> =
    TCamelCaseString extends `${infer TStringConvertedSoFar}${infer TStringYetToConvert}`
        ? `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
              ? "_"
              : ""}${Lowercase<TStringConvertedSoFar>}${CamelToSnakeCase<TStringYetToConvert>}`
        : TCamelCaseString;
```
