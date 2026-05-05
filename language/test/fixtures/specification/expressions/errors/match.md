# Catch Match

Catch match branches on propagated failures.

## results

### catch match binds errors

A catch match arm can bind the propagated error.

```ds
declare function read(): Result<int, string>;

const value = try {
    read()?;
    1
} catch match (failure) {
    error => {
        error satisfies string;
        0
    }
};
value satisfies int;
```

### catch match narrows error unions

Catch match arms narrow the propagated error union.

```ds
struct MissingError {
    path: string;
}

struct FormatError {
    line: int32;
}

declare function read(): Result<int, MissingError>;
declare function parse(): Result<int, FormatError>;

const value = try {
    read()?;
    parse()?;
    1
} catch match (failure) {
    MissingError { path } => {
        path satisfies string;
        0
    }
    FormatError { line } => {
        line satisfies int32;
        0
    }
};
value satisfies int;
```

### fallback arms cover remaining errors

A fallback arm covers the remaining failures.

```ds
struct MissingError {
    path: string;
}

struct FormatError {
    line: int32;
}

declare function read(): Result<int, MissingError | FormatError>;

const value = try {
    read()?;
    1
} catch match (failure) {
    MissingError { path } => path.length
    _ => 0
};
value satisfies int;
```

### catch match arms join with the try body

Catch match arm types join with the try body type.

```ds
declare function read(): Result<int, string>;

const value = try {
    read()?;
    1
} catch match (failure) {
    _ => "fallback"
};
value satisfies int | string;
```

## nullish

### nullish failures can be matched

Nullish failures can be matched directly.

```ds
declare function read(): Result<int, Error | null> | undefined;

const value = try {
    read()?;
    1
} catch match (failure) {
    null => 0
    undefined => 0
    _ => 0
};
value satisfies int;
```

## exhaustiveness

### catch match must be exhaustive

Catch match must cover every propagated failure.

```ds
struct MissingError {
    path: string;
}

struct FormatError {
    line: int32;
}

declare function read(): Result<int, MissingError | FormatError>;

const value = try {
    read()?;
    1
} catch match (failure) {
    MissingError { path } => 0
};
```

- contains: not exhaustive
