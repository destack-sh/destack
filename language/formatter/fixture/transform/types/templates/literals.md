# Type Template Literals

## Type Template Literals

### template literal type breaks after assignment

Template literal types stay inline with `=` at wider line widths.

```ds:main.ds
type templateLiteralType = `${
  TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? "_"
    : ""
}`;
```

```ds expected
type templateLiteralType = `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? "_"
    : ""}`;
```

### template literal type breaks after assignment at narrower widths

Template literal types break after `=` when the configured line width is narrower.

```ds:main.ds line-width=80
type templateLiteralType = `${
  TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? "_"
    : ""
}`;
```

```ds expected
type templateLiteralType =
    `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
        ? "_"
        : ""}`;
```

### template literal type with nested conditionals

Nested template literal types break with stable indentation.

```ds:main.ds line-width=80
type CamelToSnakeCase<TCamelCaseString: string> =
  TCamelCaseString extends `${infer TStringConvertedSoFar}${infer TStringYetToConvert}`
    ? `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
        ? "_"
        : ""}${Lowercase<TStringConvertedSoFar>}${CamelToSnakeCase<TStringYetToConvert>}`
    : TCamelCaseString
```

```ds expected
type CamelToSnakeCase<TCamelCaseString: string> =
    TCamelCaseString extends `${infer TStringConvertedSoFar}${infer TStringYetToConvert}`
        ? `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
              ? "_"
              : ""}${Lowercase<TStringConvertedSoFar>}${CamelToSnakeCase<TStringYetToConvert>}`
        : TCamelCaseString;
```
