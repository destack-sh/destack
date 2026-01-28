# Workspace Symbols

## Basic Search

### Search by substring

Workspace symbol search should find matching declarations across files.

```ds:alpha.ds
export function alpha(): void {}
export class AlphaClass {}
```

```ds:beta.ds
export function beta(): void {}
export struct BetaStruct {}
```

```query workspace_symbols alpha
alpha(function) file=alpha.ds range=1:1-1:33
AlphaClass(class) file=alpha.ds range=2:1-2:27
```

## Case-Insensitive Search

### Search is case-insensitive

Workspace symbol search should match regardless of casing.

```ds:gamma.ds
export function CamelCaseFeature(): void {}
```

```query workspace_symbols camelcase
CamelCaseFeature(function) file=gamma.ds range=1:1-1:44
```

## Ranking and Ordering

### Exact, prefix, then substring ordering

Workspace symbol search should rank exact matches first, then prefix matches, then substring matches.

```ds:rank.ds
export function alpha(): void {}
export function alphabet(): void {}
export function megaAlpha(): void {}
```

```query workspace_symbols alpha
alpha(function) file=rank.ds range=1:1-1:33
alphabet(function) file=rank.ds range=2:1-2:36
megaAlpha(function) file=rank.ds range=3:1-3:37
```

## Members and Containers

### Include member symbols with containers

Workspace symbol search should include class members and enum fields with container names.

```ds:members.ds
class Logger {
    log(message: string): void {}
    level: int32
}

enum Color {
    Red,
}
```

```query workspace_symbols log
log(method) file=members.ds range=2:5-2:34 container=Logger
Logger(class) file=members.ds range=1:1-4:2
```

```query workspace_symbols Red
Red(enum_member) file=members.ds range=7:5-7:8 container=Color
```

## Additional Kinds

### Include type aliases and namespaces

Workspace symbol search should include type aliases and namespaces.

```ds:kinds.ds
export type UserId = string;
export namespace Api {
    export function fetch(): void {}
}
```

```query workspace_symbols UserId
UserId
```

```query workspace_symbols Api
Api
```
