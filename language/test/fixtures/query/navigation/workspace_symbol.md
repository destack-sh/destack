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

### Cross-module ranking with duplicates

Workspace symbol search should rank identical names deterministically across files.

```ds:dup_a.ds
export class Widget {}
export function WidgetFactory(): void {}
```

```ds:dup_b.ds
export struct Widget {}
export struct WidgetBox {}
```

```query workspace_symbols Widget
Widget(class) file=dup_a.ds range=1:1-1:23
Widget(struct) file=dup_b.ds range=1:1-1:24
WidgetBox(struct) file=dup_b.ds range=2:1-2:27
WidgetFactory(function) file=dup_a.ds range=2:1-2:41
```

### Stable ordering for equal names and kinds

Workspace symbol search should keep deterministic ordering when score, name, and kind are equal.

```ds:tie_a.ds
export function render(): void {}
```

```ds:tie_b.ds
export function render(): void {}
```

```query workspace_symbols render
render(function) file=tie_a.ds range=1:1-1:34
render(function) file=tie_b.ds range=1:1-1:34
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
UserId(type_parameter) file=kinds.ds range=1:1-1:28
```

```query workspace_symbols Api
Api(namespace) file=kinds.ds range=2:1-4:2
```

## Empty Search

### No results for unmatched query

Workspace symbol search should return no results when nothing matches.

```ds:empty_match.ds
export function alphaOnly(): void {}
```

```query workspace_symbols does_not_exist
<none>
```

## Damaged Source

### Return no symbols for heavily malformed files

Workspace symbol search should fail gracefully when the file cannot be indexed.

```ds:damaged.ds
export function stable(): void {}
export function broken( {}
```

```query workspace_symbols stable
<none>
```
