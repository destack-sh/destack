# Newtype Tagged Unions

`@tagged` adds constructor helpers for tagged newtype unions.

## constructors

### tagged constructors insert discriminants

```ds
@tagged
newtype Shape =
    | { kind: "rectangle"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };

const rectangle = Shape.rectangle({ width: 10, height: 20 });
const circle = Shape.circle({ radius: 5 });

rectangle satisfies Shape;
circle satisfies Shape;
```

### tagged constructors preserve payload types

```ds
@tagged
newtype AppError =
    | { kind: "missing"; path: string }
    | { kind: "denied"; code: int32 };

const error = AppError.missing({ path: "config.json" });

match (error) {
    AppError { kind: "missing", path } => path satisfies string
    AppError { kind: "denied", code } => code satisfies int32
}
```

## naming

### preserve keeps discriminant names

```ds
@tagged("preserve")
newtype Shape =
    | { kind: "rectangle"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };

const rectangle = Shape.rectangle({ width: 10, height: 20 });
const circle = Shape.circle({ radius: 5 });

rectangle satisfies Shape;
circle satisfies Shape;
```

### camelCase converts discriminant names

```ds
@tagged("camelCase")
newtype Event =
    | { kind: "parse-error"; line: int32 }
    | { kind: "file-missing"; path: string };

const parse = Event.parseError({ line: 10 });
const missing = Event.fileMissing({ path: "config.json" });

parse satisfies Event;
missing satisfies Event;
```

### UpperCamelCase converts discriminant names

```ds
@tagged("UpperCamelCase")
newtype Event =
    | { kind: "parse-error"; line: int32 }
    | { kind: "file-missing"; path: string };

const parse = Event.ParseError({ line: 10 });
const missing = Event.FileMissing({ path: "config.json" });

parse satisfies Event;
missing satisfies Event;
```

### snake_case converts discriminant names

```ds
@tagged("snake_case")
newtype Event =
    | { kind: "parseError"; line: int32 }
    | { kind: "fileMissing"; path: string };

const parse = Event.parse_error({ line: 10 });
const missing = Event.file_missing({ path: "config.json" });

parse satisfies Event;
missing satisfies Event;
```

### SCREAMING_SNAKE_CASE converts discriminant names

```ds
@tagged("SCREAMING_SNAKE_CASE")
newtype Event =
    | { kind: "parseError"; line: int32 }
    | { kind: "fileMissing"; path: string };

const parse = Event.PARSE_ERROR({ line: 10 });
const missing = Event.FILE_MISSING({ path: "config.json" });

parse satisfies Event;
missing satisfies Event;
```

### explicit names cover numeric discriminants

```ds
@tagged({ names: { "1": "Ready", "2": "Done" } })
newtype State =
    | { kind: 1; path: string }
    | { kind: 2; code: int32 };

const ready = State.Ready({ path: "config.json" });
const done = State.Done({ code: 0 });

ready satisfies State;
done satisfies State;
```

### object directive selects naming policy

```ds
@tagged({ case: "UpperCamelCase" })
newtype Event =
    | { kind: "parse-error"; line: int32 }
    | { kind: "file-missing"; path: string };

const parse = Event.ParseError({ line: 10 });
const missing = Event.FileMissing({ path: "config.json" });

parse satisfies Event;
missing satisfies Event;
```

### object directive selects discriminant field

```ds
@tagged({ field: "type", case: "UpperCamelCase" })
newtype Shape =
    | { type: "rectangle"; width: int32; height: int32 }
    | { type: "circle"; radius: int32 };

const rectangle = Shape.Rectangle({ width: 10, height: 20 });
const circle = Shape.Circle({ radius: 5 });

rectangle satisfies Shape;
circle satisfies Shape;
```

### compiler taggedCase selects default naming

```ds:main.ds
@tagged
newtype Event =
    | { kind: "parse-error"; line: int32 }
    | { kind: "file-missing"; path: string };

const parse = Event.ParseError({ line: 10 });
const missing = Event.FileMissing({ path: "config.json" });

parse satisfies Event;
missing satisfies Event;
```

```json:destack.json
{
    "compiler": {
        "taggedCase": "UpperCamelCase"
    }
}
```

## rejections

### tagged constructors require object variants

```ds
@tagged
newtype Value = string | int32;
```

- contains: tagged

### tagged constructors require string discriminants by default

```ds
@tagged
newtype Event =
    | { kind: 1; path: string }
    | { kind: 2; code: int32 };
```

- contains: discriminant

### explicit names are required for every non-string discriminant

```ds
@tagged({ names: { "1": "Ready" } })
newtype Event =
    | { kind: 1; path: string }
    | { kind: 2; code: int32 };
```

- contains: discriminant

### tagged constructors require unique discriminants

```ds
@tagged
newtype Event =
    | { kind: "message"; text: string }
    | { kind: "message"; code: int32 };
```

- contains: duplicate

### preserve requires valid property names

```ds
@tagged
newtype Event =
    | { kind: "parse-error"; line: int32 }
    | { kind: "file-missing"; path: string };
```

- contains: property

### tagged constructors require unique generated names

```ds
@tagged("camelCase")
newtype Event =
    | { kind: "parse-error"; line: int32 }
    | { kind: "parse_error"; path: string };
```

- contains: duplicate
