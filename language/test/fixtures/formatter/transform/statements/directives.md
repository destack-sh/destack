# Directive Prologues

Tests for directive prologue formatting.

## Basic Directives

### single directive

Directives keep quotes and end with semicolons.

```ts:main.ts
"use strict"
doWork()
```

```ts expected
"use strict";

doWork();
```

### multiple directives

Multiple directives stay grouped without blank lines.

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
