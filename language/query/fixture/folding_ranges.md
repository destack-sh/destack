
## Declarations and Comments

### Fold structured declarations and comment blocks

Multiline declarations and their contiguous comments form folding ranges.

```tspp main.tspp
function add(left: int32, right: int32): int32 {
    return left + right;
}

// first line
// second line
const value = 1;
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..2
@folding_ranges.range lines=4..5 kind=comment
```

### Fold the current declaration body

A declaration becomes foldable when its body spans multiple lines.

```tspp main.tspp
function run(): void {}
```

```query folding_ranges main.tspp
@folding_ranges.none
```

```tspp main.tspp change
function run(): void {
    const value = 1;
}
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..2
```

```diff main.tspp
@@ -1,3 +1,3 @@
 function run(): void {
-    const value = 1;
+    const value = 2;
 }
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..2
```

## Single Lines

### Omit single line declarations

A one-line declaration has nothing to fold.

```tspp main.tspp
function ping(): void {}
```

```query folding_ranges main.tspp
@folding_ranges.none
```

## Classes

### Fold a class body

A multiline class body forms one folding range.

```tspp main.tspp
class Box {
    value: int32;
}
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..2
```

## Declaration Blocks

### Fold module and global declaration blocks

Module and global bodies use their declaration ranges.

```tspp main.tspp
module {
    const local = 1;
}

global {
    declare const ambient: int32;
}
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..2
@folding_ranges.range lines=4..6
```

## Type Declarations

### Fold type declaration bodies

Structs, interfaces, enums, and extensions expose their body ranges.

```tspp main.tspp
struct Point {
    x: int32;
}

interface Draw {
    draw(): void;
}

enum Color {
    Red,
}

extension of Point {
    draw(): void {}
}
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..2
@folding_ranges.range lines=4..6
@folding_ranges.range lines=8..10
@folding_ranges.range lines=12..14
```

## Imports

### Fold a multiline import

A multiline import exposes its source extent as an import fold.

```tspp main.tspp
import {
    alpha,
    beta,
} from "./library.tspp";
```

```tspp library.tspp
export const alpha = 1;
export const beta = 2;
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..3 kind=imports
```

### Fold a contiguous import section

Adjacent imports form one import fold without consuming the following declaration.

```tspp main.tspp
import { alpha } from "./alpha.tspp";
import { beta } from "./beta.tspp";
import { gamma } from "./gamma.tspp";

const value = alpha + beta + gamma;
```

```tspp alpha.tspp
export const alpha = 1;
```

```tspp beta.tspp
export const beta = 2;
```

```tspp gamma.tspp
export const gamma = 3;
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..2 kind=imports
```

## Nested Blocks

### Fold nested control-flow blocks

Nested statement blocks remain independently foldable.

```tspp main.tspp
function choose(value: boolean): int32 {
    if (value) {
        return 1;
    } else {
        return 2;
    }
}
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..6
@folding_ranges.range lines=1..3
@folding_ranges.range lines=3..5
```

## Lists and Literals

### Fold nested parameter and argument lists

Multiline lists remain independently foldable inside their declaration and call.

```tspp main.tspp
function add(
    left: int32,
    right: int32,
): int32 {
    return left + right;
}

const total = add(
    1,
    2,
);
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..5
@folding_ranges.range lines=0..3
@folding_ranges.range lines=7..10
```

### Fold collection and structural type bodies

Arrays, objects, and structural types expose their own source ranges.

```tspp main.tspp
const values = [
    1,
    2,
];

const options = {
    enabled: true,
    count: 2,
};

type Options = {
    enabled: boolean;
    count: int32;
};
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..3
@folding_ranges.range lines=5..8
@folding_ranges.range lines=10..13
```

## Block Comments

### Fold a multiline block comment

A multiline block comment forms one comment fold.

```tspp main.tspp
/*
 * first line
 * second line
 */
const value = 1;
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..3 kind=comment
```

## Regions

### Fold an explicit source region

Region markers form one named editor fold.

```tspp main.tspp
// #region setup
const first = 1;
const second = 2;
// #endregion

const result = first + second;
```

```query folding_ranges main.tspp
@folding_ranges.range lines=0..3 kind=region collapsed=setup
```
