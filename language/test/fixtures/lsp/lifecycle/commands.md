# Commands

## Workspace Commands

### Reindex keeps navigation responsive

Executing the reindex command should not break later definition requests.

```ds:lib.ds
export function /*def*/ping(): void {}
```

```ds:main.ds
import { ping } from "./lib.ds";

/*use*/ping();
```

```lsp execute_command destack.reindex
```

### Rescan keeps navigation responsive

Executing the rescan command should not break later definition requests.

```ds:lib.ds
export function /*def*/pong(): void {}
```

```ds:main.ds
import { pong } from "./lib.ds";

/*use*/pong();
```

```lsp execute_command destack.rescan
```

### Clear cache keeps navigation responsive

Executing the clear-cache command should not break later definition requests.

```ds:lib.ds
export function /*def*/buzz(): void {}
```

```ds:main.ds
import { buzz } from "./lib.ds";

/*use*/buzz();
```

```lsp execute_command destack.clearCache
```

## Command Sequences

### Reindex after an overlay edit keeps navigation and references exact
A reindex should preserve navigation and reference answers after the overlay adds a new call site.

```ds:lib.ds
export [|function /*def*/ping(): void {}|]
```

```ds:main.ds
import { /*ping_import*/ping } from "./lib.ds";

/*use*/ping();
```

```ds:lib.ds[1]
export [|function /*def*/ping(): void {}|]
```

```ds:main.ds[1]
import { /*ping_import*/ping } from "./lib.ds";

/*use*/ping();
/*use_second*/ping();
```

```lsp definition use [0]
def
```

```lsp references def [0]
def
ping_import
use
```

```lsp execute_command destack.reindex [1]
```

```lsp definition use [1]
def
```

```lsp references def [1]
def
ping_import
use
use_second
```

### Rescan after an overlay edit keeps symbols and links exact
A rescan should preserve symbol and document-link answers after an import rewrite.

```ds:alpha.ds
export const alpha = 1;
```

```ds:beta.ds
export const beta = 2;
```

```ds:main.ds
import { alpha } from "./alpha.ds";

[|function /*wrapper*/main(): void {
   const value = alpha;
}|]
```

```ds:alpha.ds[1]
export const alpha = 1;
```

```ds:beta.ds[1]
export const beta = 2;
```

```ds:main.ds[1]
import { beta } from "./beta.ds";

[|function /*wrapper*/main(): void {
   const value = beta;
}|]
```

```lsp document_symbols main.ds [0]
wrapper|function
```

```lsp execute_command destack.rescan [1]
```

```lsp document_link main.ds [1]
range=0:21-0:32
target=beta.ds
tooltip=Go to ./beta.ds
```

```lsp document_symbols main.ds [1]
wrapper|function
```

### Clear cache after overlay churn keeps symbols and completion exact
Clearing cache should preserve multiple query surfaces after the overlay changes a member set.

```ds:main.ds
[|class /*Point*/Point {
   [|/*first_name*/first_name: string|]
}|]

[|const /*point*/point = new Point();|] point./*completion*/fir
```

```ds:main.ds[1]
[|class /*Point*/Point {
   [|/*first_name*/first_name: string|]
   [|/*first_value*/first_value: int|]
}|]

[|const /*point*/point = new Point();|] point./*completion*/fir
```

```lsp completion completion [0]
first_name|field
```

```lsp execute_command destack.clearCache [1]
```

```lsp completion completion [1]
first_name|field
first_value|field
```

```lsp document_symbols main.ds [1]
Point|class
first_name|field
first_value|field
```

## Focused Requeries

### Reindex after two module rewrites and re-query definition and links
Reindex should preserve both definition and import-link answers after a two-file rewrite.

```ds:alpha.ds
export [|const /*alpha_def*/value = 1;|]
```

```ds:beta.ds
export [|const /*beta_def*/value = 2;|]
```

```ds:main.ds
import { value } from "./alpha.ds";
const current = /*use*/value;
```

```ds:main.ds[1]
import { value } from "./beta.ds";
const current = /*use*/value;
```

```lsp execute_command destack.reindex [1]
```

```lsp definition use [1]
beta_def
```

```lsp document_link main.ds [1]
range=0:22-0:33
target=beta.ds
tooltip=Go to ./beta.ds
```

### Rescan after multi-file growth and re-query references and workspace diagnostics
Rescan should preserve both references and workspace diagnostics after two files change together.

```ds:lib.ds
export [|function /*def*/ping(): void {}|]
```

```ds:a.ds
import { /*a_symbol*/ping } from "./lib.ds";

/*a_use*/ping();
```

```ds:b.ds
const broken = ;
```

```ds:a.ds[1]
import { /*a_symbol*/ping } from "./lib.ds";

/*a_use*/ping();
/*b_use*/ping();
```

```ds:b.ds[1]
const broken = ;
```

```lsp execute_command destack.rescan [1]
```

```lsp references def [1]
def
a_symbol
a_use
b_use
```

```lsp workspace_diagnostic [1]
file=b.ds
range=0:15-0:16
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Clear cache after class growth and re-query symbols
Clear cache should preserve document symbols after a type file grows.

```ds:model.ds
[|export class /*Point*/Point {
   [|/*first_name*/first_name: string = ""|]
}|]
```

```ds:main.ds
import { Point } from "./model.ds";

const point = new Point();
point.first_name;
```

```ds:model.ds[1]
[|export class /*Point*/Point {
   [|/*first_name*/first_name: string = ""|]
   [|/*first_value*/first_value: int = 0|]
}|]
```

```lsp execute_command destack.clearCache [1]
```

```lsp document_symbols model.ds [1]
Point|class
first_name|field
first_value|field
```

## Multi-Surface Stability

### Reindex after three-file import churn and re-query three surfaces
Reindex should keep definition, references, and links coherent after three files churn together.

```ds:lib.ds
export [|function /*def*/ping(): void {}|]
```

```ds:helper.ds
export const helper = 1;
```

```ds:main.ds
import { /*ping_import_symbol*/ping } from "./lib.ds";
import { helper } from "./helper.ds";

/*ping_import*/ping();

const current = /*use*/ping;
```

```ds:helper.ds[1]
export const helper = 2;

export const second_helper = 3;
```

```ds:main.ds[1]
import { /*ping_import_symbol*/ping } from "./lib.ds";
import { helper } from "./helper.ds";

/*ping_import*/ping();

const current = /*use*/ping;
const total = helper + second_helper;
```

```lsp execute_command destack.reindex [1]
```

```lsp definition use [1]
def
```

```lsp references def [1]
def
ping_import_symbol
ping_import
use
```

```lsp document_link main.ds [1]
range=0:21-0:31
target=lib.ds
tooltip=Go to ./lib.ds

range=1:23-1:36
target=helper.ds
tooltip=Go to ./helper.ds
```

### Clear cache after mixed symbol churn and re-query search and diagnostics
Clear cache should keep workspace search and diagnostics coherent after unrelated files change together.

```ds:logger.ds
[|class /*logger*/Logger {
   [|/*log*/log(message: string): void {}|]
}|]
```

```ds:broken.ds
const broken = ;
```

```ds:helper.ds
[|class Helper {
   [|value: int = 0|]
}|]
```

```ds:helper.ds[1]
[|class Helper {
   [|value: int = 0|]
   [|extra: int = 0|]
}|]
```

```lsp execute_command destack.clearCache [1]
```

```lsp workspace_symbols log [1]
log|method|Logger
logger|class
```

```lsp workspace_diagnostic [1]
file=broken.ds
range=0:15-0:16
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Rescan after a mixed fix and growth and re-query symbols and diagnostics
Rescan should preserve document symbols while reflecting the current mixed diagnostic state.

```ds:model.ds
[|export class /*Point*/Point {
   [|/*first_name*/first_name: string = ""|]
}|]
```

```ds:broken.ds
const broken = ;
```

```ds:main.ds
import { Point } from "./model.ds";

const point = new Point();
point.first_name;
```

```ds:model.ds[1]
[|export class /*Point*/Point {
   [|/*first_name*/first_name: string = ""|]
   [|/*first_value*/first_value: int = 0|]
}|]
```

```ds:broken.ds[1]
const broken = 1;
```

```lsp execute_command destack.rescan [1]
```

```lsp document_symbols model.ds [1]
Point|class
first_name|field
first_value|field
```

```lsp workspace_diagnostic [1]
```

### Reindex after two consumers switch imports and re-query both definitions
Reindex should preserve exact definitions for both consumers after they switch targets together.

```ds:alpha.ds
export [|const /*alpha_def*/value = 1;|]
```

```ds:beta.ds
export [|const /*beta_def*/value = 2;|]
```

```ds:a.ds
import { value } from "./alpha.ds";
const current = /*use_a*/value;
```

```ds:b.ds
import { value } from "./alpha.ds";
const current = /*use_b*/value;
```

```ds:a.ds[1]
import { value } from "./beta.ds";
const current = /*use_a*/value;
```

```ds:b.ds[1]
import { value } from "./beta.ds";
const current = /*use_b*/value;
```

```lsp execute_command destack.reindex [1]
```

```lsp definition use_a [1]
beta_def
```

```lsp definition use_b [1]
beta_def
```

## Cross-Module Requeries

### Reindex after a split import rewrite and re-query three consumers
Reindex should preserve exact definitions after three consumers move from a shared file to split files.

```ds:shared.ds
export [|const /*shared_def*/value = 1;|]
```

```ds:left.ds
export [|const /*left_def*/value = 2;|]
```

```ds:a.ds
import { value } from "./shared.ds";
const current = /*use_a*/value;
```

```ds:b.ds
import { value } from "./shared.ds";
const current = /*use_b*/value;
```

```ds:c.ds
import { value } from "./shared.ds";
const current = /*use_c*/value;
```

```ds:a.ds[1]
import { value } from "./left.ds";
const current = /*use_a*/value;
```

```ds:b.ds[1]
import { value } from "./left.ds";
const current = /*use_b*/value;
```

```ds:c.ds[1]
import { value } from "./left.ds";
const current = /*use_c*/value;
```

```lsp execute_command destack.reindex [1]
```

```lsp definition use_a [1]
left_def
```

```lsp definition use_b [1]
left_def
```

```lsp definition use_c [1]
left_def
```

### Clear cache after symbol growth and workspace search churn
Clear cache should keep document symbols and workspace search exact after two files grow together.

```ds:alpha.ds
[|class /*alpha_class*/Alpha {
   [|/*alpha_log*/log(message: string): void {}|]
}|]
```

```ds:beta.ds
[|class Beta {
   [|value: int|]
}|]
```

```ds:beta.ds[1]
[|class /*beta_class*/BetaLogger {
   [|/*beta_log*/log(message: string): void {}|]
}|]
```

```lsp execute_command destack.clearCache [1]
```

```lsp document_symbols alpha.ds [1]
alpha_class|class
alpha_log|method
```

```lsp workspace_symbols log [1]
alpha_log|method|Alpha
beta_log|method|BetaLogger
beta_class|class
```

## Cross-Module Stability

### Rescan after a helper split and re-query links and diagnostics
Rescan should preserve document links while mixed diagnostics remain exact after a helper split.

```ds:helper.ds
export const helper = 1;
```

```ds:other.ds
export const other = 2;
```

```ds:main.ds
import { helper } from "./helper.ds";
import { other } from "./other.ds";

const total = helper + other;
```

```ds:broken.ds
const broken = ;
```

```ds:helper.ds[1]
export const helper = 1;

export const second = 3;
```

```ds:main.ds[1]
import { helper, second } from "./helper.ds";
import { other } from "./other.ds";

const total = helper + other + second;
```

```ds:broken.ds[1]
const broken = ;
```

```lsp execute_command destack.rescan [1]
```

```lsp document_link main.ds [1]
range=0:31-0:44
target=helper.ds
tooltip=Go to ./helper.ds

range=1:22-1:34
target=other.ds
tooltip=Go to ./other.ds
```

```lsp workspace_diagnostic [1]
file=broken.ds
range=0:15-0:16
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Reindex after one broken file fixes and another grows symbols
Reindex should keep diagnostics clean while symbol queries see the new growth.

```ds:model.ds
[|class /*Point*/Point {
   [|/*first_name*/first_name: string = ""|]
}|]
```

```ds:broken.ds
const broken = ;
```

```ds:model.ds[1]
[|class /*Point*/Point {
   [|/*first_name*/first_name: string = ""|]
   [|/*first_value*/first_value: int = 0|]
}|]
```

```ds:broken.ds[1]
const broken = 1;
```

```lsp execute_command destack.reindex [1]
```

```lsp document_symbols model.ds [1]
Point|class
first_name|field
first_value|field
```

```lsp workspace_diagnostic [1]
```
