# Newtype Tagged Unions

`Tagged` derives constructor helpers for nominal discriminated unions.

## constructors

### tagged constructors insert discriminants

Constructors fill the discriminant in, so payloads omit it.

```ds
@derive(Tagged)
newtype Shape =
    | { kind: "rectangle"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };

const rectangle = Shape.Rectangle({ width: 10, height: 20 });
const circle = Shape.Circle({ radius: 5 });

rectangle satisfies Shape;
circle satisfies Shape;
```

### tagged constructors preserve payload types

Payload fields keep their declared types through construction and matching.

```ds
@derive(Tagged)
newtype AppError = { kind: "missing"; path: string } | { kind: "denied"; code: int32 };

const error = AppError.Missing({ path: "config.json" });

match (error) {
    AppError.Missing({ path }) => path satisfies string
    AppError.Denied({ code }) => code satisfies int32
}
```

## naming

### preserve keeps discriminant names

`case: "preserve"` uses the discriminant strings verbatim.

```ds
@derive(Tagged({ case: "preserve" }))
newtype Shape =
    | { kind: "rectangle"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };

const rectangle = Shape.rectangle({ width: 10, height: 20 });
const circle = Shape.circle({ radius: 5 });

rectangle satisfies Shape;
circle satisfies Shape;
```

### camelCase converts discriminant names

Discriminant strings convert to the configured case.

```ds
@derive(Tagged({ case: "camelCase" }))
newtype Event = { kind: "parse-error"; line: int32 } | { kind: "file-missing"; path: string };

const parse = Event.parseError({ line: 10 });
const missing = Event.fileMissing({ path: "config.json" });

parse satisfies Event;
missing satisfies Event;
```

### UpperCamelCase is the default

Without configuration, constructors are UpperCamelCase.

```ds
@derive(Tagged)
newtype Event = { kind: "parse-error"; line: int32 } | { kind: "file-missing"; path: string };

const parse = Event.ParseError({ line: 10 });
const missing = Event.FileMissing({ path: "config.json" });

parse satisfies Event;
missing satisfies Event;
```

### snake_case converts discriminant names

Conversion works toward any configured case.

```ds
@derive(Tagged({ case: "snake_case" }))
newtype Event = { kind: "parseError"; line: int32 } | { kind: "fileMissing"; path: string };

const parse = Event.parse_error({ line: 10 });
const missing = Event.file_missing({ path: "config.json" });

parse satisfies Event;
missing satisfies Event;
```

### SCREAMING_SNAKE_CASE converts discriminant names

Conversion handles every supported case.

```ds
@derive(Tagged({ case: "SCREAMING_SNAKE_CASE" }))
newtype Event = { kind: "parseError"; line: int32 } | { kind: "fileMissing"; path: string };

const parse = Event.PARSE_ERROR({ line: 10 });
const missing = Event.FILE_MISSING({ path: "config.json" });

parse satisfies Event;
missing satisfies Event;
```

### explicit names cover numeric discriminants

Numeric discriminants get their constructor names from `names`.

```ds
@derive(Tagged({ names: { "1": "Ready", "2": "Done" } }))
newtype State = { kind: 1; path: string } | { kind: 2; code: int32 };

const ready = State.Ready({ path: "config.json" });
const done = State.Done({ code: 0 });

ready satisfies State;
done satisfies State;
```

### tagged constructors infer the discriminant field

Any shared literal field works as the discriminant, not just `kind`.

```ds
@derive(Tagged)
newtype Shape =
    | { type: "rectangle"; width: int32; height: int32 }
    | { type: "circle"; radius: int32 };

const rectangle = Shape.Rectangle({ width: 10, height: 20 });
const circle = Shape.Circle({ radius: 5 });

rectangle satisfies Shape;
circle satisfies Shape;
```

## rejections

### tagged unions require object variants

Bare scalars have no field to discriminate on.

```ds
@derive(Tagged)
newtype Value = string | int32;
```

- contains: Tagged

### numeric discriminants require explicit names

Numbers cannot become identifiers on their own.

```ds
@derive(Tagged)
newtype Event = { kind: 1; path: string } | { kind: 2; code: int32 };
```

- contains: discriminant

### explicit names must cover every numeric discriminant

Partial name maps leave variants unconstructable.

```ds
@derive(Tagged({ names: { "1": "Ready" } }))
newtype Event = { kind: 1; path: string } | { kind: 2; code: int32 };
```

- contains: discriminant

### tagged discriminants must be unique

Two variants cannot share a discriminant value.

```ds
@derive(Tagged)
newtype Event = { kind: "message"; text: string } | { kind: "message"; code: int32 };
```

- contains: duplicate

### tagged constructor names must be unique

Case conversion must not collide constructor names.

```ds
@derive(Tagged({ case: "camelCase" }))
newtype Event = { kind: "parse-error"; line: int32 } | { kind: "parse_error"; path: string };
```

- contains: duplicate

### tagged unions require one discriminant field

Multiple shared literal fields are ambiguous.

```ds
@derive(Tagged)
newtype Event =
    | { kind: "parse"; type: "error"; line: int32 }
    | { kind: "file"; type: "missing"; path: string };
```

- contains: discriminant
