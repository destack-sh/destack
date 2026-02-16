# Directive Prologues

Tests for directive prologue formatting.

## Basic Directives

### single directive

Directives keep quotes, end with semicolons, and separate the prologue from following code.

```ts:main.ts
"use strict"
doWork()
```

```ts expected
"use strict";
doWork();
```

### multiple directives

Multiple directives stay grouped with one blank line before following code.

```ts:main.ts
"use client"
"use strict"
render()
```

```ts expected
"use client";
"use strict";
render();
```
