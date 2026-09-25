# Directive Prologues

Directive fixtures cover directive prologues and comments around directives.

## Directive Prologues

### single directive

Directives keep quotes, end with semicolons, and separate the prologue from following code.

```tspp:main.tspp
"use strict"
doWork()
```

```tspp expected
"use strict";
doWork();
```

### multiple directives

Multiple directives stay grouped with one blank line before following code.

```tspp:main.tspp
"use client"
"use strict"
render()
```

```tspp expected
"use client";
"use strict";
render();
```

## Formatter Suppression

### suppression aliases preserve ignored statements

Recognized suppression aliases preserve ignored source and keep formatted statements stable.

```tspp:main.tspp
const keepFormatted = 1;

// fmt-ignore
const fmtIgnored   =  [  1,2,3 ]

// format-ignore
const formatIgnored   =  {  alpha:1,  beta:2 }

// prettier-ignore
const prettierIgnored   =  call(  alpha,  beta )

// oxfmt-ignore
const oxfmtIgnored   =  source /* hop */ ?. ( "value" )

// deno-fmt-ignore
const denoIgnored   =  foo?.( "value" )

// biome-ignore format: keep raw
const biomeIgnored   =  run(  first,  second )

// fmt-ignore-start
const rangeIgnoredA   =  [  4,5,6 ]
const rangeIgnoredB   =  {  gamma:3,  delta:4 }
// fmt-ignore-end

const keepFormattedToo = 2;
```

```tspp expected
const keepFormatted = 1;

// fmt-ignore
const fmtIgnored   =  [  1,2,3 ]

// format-ignore
const formatIgnored   =  {  alpha:1,  beta:2 }

// prettier-ignore
const prettierIgnored   =  call(  alpha,  beta )

// oxfmt-ignore
const oxfmtIgnored   =  source /* hop */ ?. ( "value" )

// deno-fmt-ignore
const denoIgnored   =  foo?.( "value" )

// biome-ignore format: keep raw
const biomeIgnored   =  run(  first,  second )

// fmt-ignore-start
const rangeIgnoredA   =  [  4,5,6 ]
const rangeIgnoredB   =  {  gamma:3,  delta:4 }
// fmt-ignore-end

const keepFormattedToo = 2;
```
