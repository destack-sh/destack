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
