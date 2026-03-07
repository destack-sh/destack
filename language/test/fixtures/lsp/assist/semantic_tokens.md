# Semantic Tokens

## Full Document

### Function and parameter tokens

Semantic tokens should classify function and parameter declarations across the full file.

```ds:main.ds
function greet(name: string): string {
    return name;
}
```

```lsp semantic_tokens
range=1:10-1:15
text=greet
kind=function
modifier=declaration

range=1:16-1:20
text=name
kind=parameter
modifier=declaration

range=1:22-1:28
text=string
kind=type

range=1:31-1:37
text=string
kind=type

range=2:12-2:16
text=name
kind=variable
```

## Range Requests

### Parameter and type tokens in range

Range requests should return only the tokens that fall inside the requested span.

```ds:main.ds
function greet([|name: string|]): string {
    return name;
}
```

```lsp semantic_tokens_range
range=1:16-1:20
text=name
kind=parameter
modifier=declaration

range=1:22-1:28
text=string
kind=type
```

## Delta Updates

### Comment shifts preserve token meaning

Semantic-token delta updates should rewrite the baseline token stream after a text shift.

```ds:main.ds
function greet(name: string): string {
   return name;
}
```

```lsp semantic_tokens_delta_source
// moved tokens
function greet(name: string): string {
    return name;
}
```

```lsp semantic_tokens_delta
range=2:10-2:15
text=greet
kind=function
modifier=declaration

range=2:16-2:20
text=name
kind=parameter
modifier=declaration

range=2:22-2:28
text=string
kind=type

range=2:31-2:37
text=string
kind=type

range=3:12-3:16
text=name
kind=variable
```

## Delta Churn

### Rebuild token deltas while a sibling file churns and the active file grows a helper

Semantic-token deltas should stay exact after the active file grows a helper and a sibling file changes in the same transition before follow-up navigation and symbol queries run.

```ds:sibling.ds
export const stable = 1;
```

```ds:main.ds
function /*convert_def*//*convert*/convert(/*value*/value: number): number {
    return value;
}

const /*total*/total = /*use*/convert(1);
```

```lsp semantic_tokens main.ds [0]
range=1:10-1:17
text=convert
kind=function
modifier=declaration

range=1:18-1:23
text=value
kind=parameter
modifier=declaration

range=1:25-1:31
text=number
kind=type

range=1:34-1:40
text=number
kind=type

range=2:12-2:17
text=value
kind=variable

range=5:7-5:12
text=total
kind=variable
modifier=declaration

range=5:15-5:22
text=convert
kind=function

range=5:23-5:24
text=1
kind=number
```

```ds:sibling.ds[1]
export const stable = 1;
export const extra = 2;
```

```ds:main.ds[1]
function /*helper_def*//*helper*/helper(/*amount*/amount: number): number {
    return amount;
}

function /*convert_def*//*convert*/convert(/*value*/value: number): number {
    return /*helper_use*/helper(value);
}

const /*total*/total = /*use*/convert(1);
```

```lsp semantic_tokens_delta main.ds [1]
range=1:10-1:16
text=helper
kind=function
modifier=declaration

range=1:17-1:23
text=amount
kind=parameter
modifier=declaration

range=1:25-1:31
text=number
kind=type

range=1:34-1:40
text=number
kind=type

range=2:12-2:18
text=amount
kind=variable

range=5:10-5:17
text=convert
kind=function
modifier=declaration

range=5:18-5:23
text=value
kind=parameter
modifier=declaration

range=5:25-5:31
text=number
kind=type

range=5:34-5:40
text=number
kind=type

range=6:12-6:18
text=helper
kind=function

range=6:19-6:24
text=value
kind=variable

range=9:7-9:12
text=total
kind=variable
modifier=declaration

range=9:15-9:22
text=convert
kind=function

range=9:23-9:24
text=1
kind=number
```

```lsp definition helper_use [1]
helper_def
```

```lsp definition helper_use [1]
helper_def
```

### Keep token deltas exact after a cross-file type rename and helper split

Semantic-token deltas should stay exact after one file renames a shared type and the active file grows a helper before follow-up navigation runs.

```ds:types.ds
export type /*amount_def*/Amount = number;
```

```ds:main.ds
import { Amount } from "./types.ds";

function /*convert_def*/convert(value: /*type_use*/Amount): Amount {
    return value;
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens main.ds [0]
range=1:10-1:16
text=Amount
kind=type
modifier=declaration

range=3:10-3:17
text=convert
kind=function
modifier=declaration

range=3:18-3:23
text=value
kind=parameter
modifier=declaration

range=3:25-3:31
text=Amount
kind=type

range=3:34-3:40
text=Amount
kind=type

range=4:12-4:17
text=value
kind=variable

range=7:7-7:12
text=total
kind=variable
modifier=declaration

range=7:15-7:22
text=convert
kind=function

range=7:23-7:24
text=1
kind=number
```

```ds:types.ds[1]
export type /*size_def*/Size = number;
```

```ds:main.ds[1]
import { Size } from "./types.ds";

[|function /*helper_def*/helper(amount: Size): Size {
    return amount;
}|]

function /*convert_def*/convert(value: /*type_use*/Size): Size {
    return /*helper_use*/helper(value);
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens_delta main.ds [1]
range=1:10-1:14
text=Size
kind=type
modifier=declaration

range=3:11-3:17
text=helper
kind=function
modifier=declaration

range=3:18-3:24
text=amount
kind=parameter
modifier=declaration

range=3:26-3:30
text=Size
kind=type

range=3:33-3:37
text=Size
kind=type

range=4:12-4:18
text=amount
kind=variable

range=7:10-7:17
text=convert
kind=function
modifier=declaration

range=7:18-7:23
text=value
kind=parameter
modifier=declaration

range=7:25-7:29
text=Size
kind=type

range=7:32-7:36
text=Size
kind=type

range=8:12-8:18
text=helper
kind=function

range=8:19-8:24
text=value
kind=variable

range=11:7-11:12
text=total
kind=variable
modifier=declaration

range=11:15-11:22
text=convert
kind=function

range=11:23-11:24
text=1
kind=number
```

```lsp definition type_use [1]
size_def
```

### Keep token deltas exact across two cross-file type renames

Semantic-token deltas should stay exact across two successive cross-file type renames while the active file keeps the helper split alive.

```ds:types.ds
export type /*amount_def*/Amount = number;
```

```ds:main.ds
import { Amount } from "./types.ds";

function /*convert_def*/convert(value: /*type_use*/Amount): Amount {
    return value;
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens main.ds [0]
range=1:10-1:16
text=Amount
kind=type
modifier=declaration

range=3:10-3:17
text=convert
kind=function
modifier=declaration

range=3:18-3:23
text=value
kind=parameter
modifier=declaration

range=3:25-3:31
text=Amount
kind=type

range=3:34-3:40
text=Amount
kind=type

range=4:12-4:17
text=value
kind=variable

range=7:7-7:12
text=total
kind=variable
modifier=declaration

range=7:15-7:22
text=convert
kind=function

range=7:23-7:24
text=1
kind=number
```

```ds:types.ds[1]
export type /*size_def*/Size = number;
```

```ds:main.ds[1]
import { Size } from "./types.ds";

function /*helper_def*/helper(amount: Size): Size {
    return amount;
}

function /*convert_def*/convert(value: /*type_use*/Size): Size {
    return /*helper_use*/helper(value);
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens_delta main.ds [1]
range=1:10-1:14
text=Size
kind=type
modifier=declaration

range=3:10-3:16
text=helper
kind=function
modifier=declaration

range=3:17-3:23
text=amount
kind=parameter
modifier=declaration

range=3:25-3:29
text=Size
kind=type

range=3:32-3:36
text=Size
kind=type

range=4:12-4:18
text=amount
kind=variable

range=7:10-7:17
text=convert
kind=function
modifier=declaration

range=7:18-7:23
text=value
kind=parameter
modifier=declaration

range=7:25-7:29
text=Size
kind=type

range=7:32-7:36
text=Size
kind=type

range=8:12-8:18
text=helper
kind=function

range=8:19-8:24
text=value
kind=variable

range=11:7-11:12
text=total
kind=variable
modifier=declaration

range=11:15-11:22
text=convert
kind=function

range=11:23-11:24
text=1
kind=number
```

```ds:types.ds[2]
export type /*count_def*/Count = number;
```

```ds:main.ds[2]
import { Count } from "./types.ds";

function /*helper_def*/helper(amount: Count): Count {
    return amount;
}

function /*convert_def*/convert(value: /*type_use*/Count): Count {
    return /*helper_use*/helper(value);
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens_delta main.ds [2]
range=1:10-1:15
text=Count
kind=type
modifier=declaration

range=3:10-3:16
text=helper
kind=function
modifier=declaration

range=3:17-3:23
text=amount
kind=parameter
modifier=declaration

range=3:25-3:30
text=Count
kind=type

range=3:33-3:38
text=Count
kind=type

range=4:12-4:18
text=amount
kind=variable

range=7:10-7:17
text=convert
kind=function
modifier=declaration

range=7:18-7:23
text=value
kind=parameter
modifier=declaration

range=7:25-7:30
text=Count
kind=type

range=7:33-7:38
text=Count
kind=type

range=8:12-8:18
text=helper
kind=function

range=8:19-8:24
text=value
kind=variable

range=11:7-11:12
text=total
kind=variable
modifier=declaration

range=11:15-11:22
text=convert
kind=function

range=11:23-11:24
text=1
kind=number
```

```lsp definition type_use [2]
count_def
```

### Keep token deltas exact after a barrel type retarget and helper split

Semantic-token deltas should stay exact when the active file keeps importing through a barrel while the barrel retargets to a renamed type and the active file grows a helper.

```ds:alpha.ds
export type Amount = number;
```

```ds:beta.ds
export type /*size_def*/Size = number;
```

```ds:types.ds
export { Amount } from "./alpha.ds";
```

```ds:main.ds
import { Amount } from "./types.ds";

function /*convert_def*/convert(value: /*type_use*/Amount): Amount {
    return value;
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens main.ds [0]
range=1:10-1:16
text=Amount
kind=type
modifier=declaration

range=3:10-3:17
text=convert
kind=function
modifier=declaration

range=3:18-3:23
text=value
kind=parameter
modifier=declaration

range=3:25-3:31
text=Amount
kind=type

range=3:34-3:40
text=Amount
kind=type

range=4:12-4:17
text=value
kind=variable

range=7:7-7:12
text=total
kind=variable
modifier=declaration

range=7:15-7:22
text=convert
kind=function

range=7:23-7:24
text=1
kind=number
```

```ds:types.ds[1]
export { Size } from "./beta.ds";
```

```ds:main.ds[1]
import { Size } from "./types.ds";

function /*helper_def*/helper(amount: Size): Size {
    return amount;
}

function /*convert_def*/convert(value: /*type_use*/Size): Size {
    return /*helper_use*/helper(value);
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens_delta main.ds [1]
range=1:10-1:14
text=Size
kind=type
modifier=declaration

range=3:10-3:16
text=helper
kind=function
modifier=declaration

range=3:17-3:23
text=amount
kind=parameter
modifier=declaration

range=3:25-3:29
text=Size
kind=type

range=3:32-3:36
text=Size
kind=type

range=4:12-4:18
text=amount
kind=variable

range=7:10-7:17
text=convert
kind=function
modifier=declaration

range=7:18-7:23
text=value
kind=parameter
modifier=declaration

range=7:25-7:29
text=Size
kind=type

range=7:32-7:36
text=Size
kind=type

range=8:12-8:18
text=helper
kind=function

range=8:19-8:24
text=value
kind=variable

range=11:7-11:12
text=total
kind=variable
modifier=declaration

range=11:15-11:22
text=convert
kind=function

range=11:23-11:24
text=1
kind=number
```

```lsp definition type_use [1]
size_def
```

### Keep token deltas exact after command churn on a cross-file type rename

Semantic-token deltas should stay exact after a cross-file type rename, helper split, and a reindex command before the follow-up delta request.

```ds:types.ds
export type /*amount_def*/Amount = number;
```

```ds:aux.ds
export const stable = 1;
```

```ds:main.ds
import { Amount } from "./types.ds";

function /*convert_def*/convert(value: /*type_use*/Amount): Amount {
    return value;
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens main.ds [0]
range=1:10-1:16
text=Amount
kind=type
modifier=declaration

range=3:10-3:17
text=convert
kind=function
modifier=declaration

range=3:18-3:23
text=value
kind=parameter
modifier=declaration

range=3:25-3:31
text=Amount
kind=type

range=3:34-3:40
text=Amount
kind=type

range=4:12-4:17
text=value
kind=variable

range=7:7-7:12
text=total
kind=variable
modifier=declaration

range=7:15-7:22
text=convert
kind=function

range=7:23-7:24
text=1
kind=number
```

```ds:types.ds[1]
export type /*size_def*/Size = number;
```

```ds:aux.ds[1]
export const stable = 1;
export const extra = 2;
```

```ds:main.ds[1]
import { Size } from "./types.ds";

function /*helper_def*/helper(amount: Size): Size {
    return amount;
}

function /*convert_def*/convert(value: /*type_use*/Size): Size {
    return /*helper_use*/helper(value);
}

const total = /*use*/convert(1);
```

```lsp execute_command destack.reindex [1]
```

```lsp semantic_tokens_delta main.ds [1]
range=1:10-1:14
text=Size
kind=type
modifier=declaration

range=3:10-3:16
text=helper
kind=function
modifier=declaration

range=3:17-3:23
text=amount
kind=parameter
modifier=declaration

range=3:25-3:29
text=Size
kind=type

range=3:32-3:36
text=Size
kind=type

range=4:12-4:18
text=amount
kind=variable

range=7:10-7:17
text=convert
kind=function
modifier=declaration

range=7:18-7:23
text=value
kind=parameter
modifier=declaration

range=7:25-7:29
text=Size
kind=type

range=7:32-7:36
text=Size
kind=type

range=8:12-8:18
text=helper
kind=function

range=8:19-8:24
text=value
kind=variable

range=11:7-11:12
text=total
kind=variable
modifier=declaration

range=11:15-11:22
text=convert
kind=function

range=11:23-11:24
text=1
kind=number
```

```lsp definition type_use [1]
size_def
```

### Keep token deltas exact after clear-cache churn on a cross-file type rename

Semantic-token deltas should stay exact after a cross-file type rename, helper split, and a clear-cache command before the follow-up delta request.

```ds:types.ds
export type /*amount_def*/Amount = number;
```

```ds:aux.ds
export const stable = 1;
```

```ds:main.ds
import { Amount } from "./types.ds";

function /*convert_def*/convert(value: /*type_use*/Amount): Amount {
    return value;
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens main.ds [0]
range=1:10-1:16
text=Amount
kind=type
modifier=declaration

range=3:10-3:17
text=convert
kind=function
modifier=declaration

range=3:18-3:23
text=value
kind=parameter
modifier=declaration

range=3:25-3:31
text=Amount
kind=type

range=3:34-3:40
text=Amount
kind=type

range=4:12-4:17
text=value
kind=variable

range=7:7-7:12
text=total
kind=variable
modifier=declaration

range=7:15-7:22
text=convert
kind=function

range=7:23-7:24
text=1
kind=number
```

```ds:types.ds[1]
export type /*size_def*/Size = number;
```

```ds:aux.ds[1]
export const stable = 1;
export const extra = 2;
```

```ds:main.ds[1]
import { Size } from "./types.ds";

function /*helper_def*/helper(amount: Size): Size {
    return amount;
}

function /*convert_def*/convert(value: /*type_use*/Size): Size {
    return /*helper_use*/helper(value);
}

const total = /*use*/convert(1);
```

```lsp execute_command destack.clearCache [1]
```

```lsp semantic_tokens_delta main.ds [1]
range=1:10-1:14
text=Size
kind=type
modifier=declaration

range=3:10-3:16
text=helper
kind=function
modifier=declaration

range=3:17-3:23
text=amount
kind=parameter
modifier=declaration

range=3:25-3:29
text=Size
kind=type

range=3:32-3:36
text=Size
kind=type

range=4:12-4:18
text=amount
kind=variable

range=7:10-7:17
text=convert
kind=function
modifier=declaration

range=7:18-7:23
text=value
kind=parameter
modifier=declaration

range=7:25-7:29
text=Size
kind=type

range=7:32-7:36
text=Size
kind=type

range=8:12-8:18
text=helper
kind=function

range=8:19-8:24
text=value
kind=variable

range=11:7-11:12
text=total
kind=variable
modifier=declaration

range=11:15-11:22
text=convert
kind=function

range=11:23-11:24
text=1
kind=number
```

```lsp definition type_use [1]
size_def
```

### Keep token deltas exact through save, close, and reopen on a cross-file type rename

Semantic-token deltas should stay exact after a cross-file type rename and helper split when the edited file is saved, closed, and reopened before the follow-up delta request.

```ds:types.ds
export type /*amount_def*/Amount = number;
```

```ds:main.ds
import { Amount } from "./types.ds";

function /*convert_def*/convert(value: /*type_use*/Amount): Amount {
    return value;
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens main.ds [0]
range=1:10-1:16
text=Amount
kind=type
modifier=declaration

range=3:10-3:17
text=convert
kind=function
modifier=declaration

range=3:18-3:23
text=value
kind=parameter
modifier=declaration

range=3:25-3:31
text=Amount
kind=type

range=3:34-3:40
text=Amount
kind=type

range=4:12-4:17
text=value
kind=variable

range=7:7-7:12
text=total
kind=variable
modifier=declaration

range=7:15-7:22
text=convert
kind=function

range=7:23-7:24
text=1
kind=number
```

```ds:types.ds[1]
export type /*size_def*/Size = number;
```

```ds:main.ds[1]
import { Size } from "./types.ds";

function /*helper_def*/helper(amount: Size): Size {
    return amount;
}

function /*convert_def*/convert(value: /*type_use*/Size): Size {
    return /*helper_use*/helper(value);
}

const total = /*use*/convert(1);
```

```lsp save types.ds [1]
```

```lsp save main.ds [1]
```

```lsp close main.ds [1]
```

```lsp open_text main.ds [1]
import { Size } from "./types.ds";

function /*helper_def*/helper(amount: Size): Size {
    return amount;
}

function /*convert_def*/convert(value: /*type_use*/Size): Size {
    return /*helper_use*/helper(value);
}

const total = /*use*/convert(1);
```

```lsp semantic_tokens_delta main.ds [1]
range=1:10-1:14
text=Size
kind=type
modifier=declaration

range=3:10-3:16
text=helper
kind=function
modifier=declaration

range=3:17-3:23
text=amount
kind=parameter
modifier=declaration

range=3:25-3:29
text=Size
kind=type

range=3:32-3:36
text=Size
kind=type

range=4:12-4:18
text=amount
kind=variable

range=7:10-7:17
text=convert
kind=function
modifier=declaration

range=7:18-7:23
text=value
kind=parameter
modifier=declaration

range=7:25-7:29
text=Size
kind=type

range=7:32-7:36
text=Size
kind=type

range=8:12-8:18
text=helper
kind=function

range=8:19-8:24
text=value
kind=variable

range=11:7-11:12
text=total
kind=variable
modifier=declaration

range=11:15-11:22
text=convert
kind=function

range=11:23-11:24
text=1
kind=number
```

```lsp definition type_use [1]
size_def
```

### Keep code-lens counts and token deltas exact through one shared churn sequence

Code-lens counts and semantic-token deltas should both stay exact when a shared type renames, the active file grows a helper, and another consumer appears in the same transition.

```ds:types.ds
export type /*amount_def*/Amount = number;
```

```ds:lib.ds
export function /*lens*/ping(): void {}
```

```ds:main.ds
import { Amount } from "./types.ds";
import { ping } from "./lib.ds";

function /*convert_def*/convert(value: /*type_use*/Amount): Amount {
    ping();
    return value;
}

const total = /*use*/convert(1);
```

```ds:other.ds
export const idle = 1;
```

```lsp semantic_tokens main.ds [0]
range=1:10-1:16
text=Amount
kind=type
modifier=declaration

range=2:10-2:14
text=ping
kind=function
modifier=declaration

range=4:10-4:17
text=convert
kind=function
modifier=declaration

range=4:18-4:23
text=value
kind=parameter
modifier=declaration

range=4:25-4:31
text=Amount
kind=type

range=4:34-4:40
text=Amount
kind=type

range=5:5-5:9
text=ping
kind=function

range=6:12-6:17
text=value
kind=variable

range=9:7-9:12
text=total
kind=variable
modifier=declaration

range=9:15-9:22
text=convert
kind=function

range=9:23-9:24
text=1
kind=number
```

```lsp code_lens lib.ds [0]
range=0:16-0:20
title=1 reference
command=destack.showReferences
```

```ds:types.ds[1]
export type /*size_def*/Size = number;
```

```ds:main.ds[1]
import { Size } from "./types.ds";
import { ping } from "./lib.ds";

function /*helper_def*/helper(amount: Size): Size {
    ping();
    return amount;
}

function /*convert_def*/convert(value: /*type_use*/Size): Size {
    return /*helper_use*/helper(value);
}

const total = /*use*/convert(1);
```

```ds:other.ds[1]
import { ping } from "./lib.ds";

ping();
```

```lsp semantic_tokens_delta main.ds [1]
range=1:10-1:14
text=Size
kind=type
modifier=declaration

range=2:10-2:14
text=ping
kind=function
modifier=declaration

range=4:10-4:16
text=helper
kind=function
modifier=declaration

range=4:17-4:23
text=amount
kind=parameter
modifier=declaration

range=4:25-4:29
text=Size
kind=type

range=4:32-4:36
text=Size
kind=type

range=5:5-5:9
text=ping
kind=function

range=6:12-6:18
text=amount
kind=variable

range=9:10-9:17
text=convert
kind=function
modifier=declaration

range=9:18-9:23
text=value
kind=parameter
modifier=declaration

range=9:25-9:29
text=Size
kind=type

range=9:32-9:36
text=Size
kind=type

range=10:12-10:18
text=helper
kind=function

range=10:19-10:24
text=value
kind=variable

range=13:7-13:12
text=total
kind=variable
modifier=declaration

range=13:15-13:22
text=convert
kind=function

range=13:23-13:24
text=1
kind=number
```

```lsp code_lens lib.ds [1]
range=0:16-0:20
title=2 references
command=destack.showReferences
```

```lsp definition type_use [1]
size_def
```
