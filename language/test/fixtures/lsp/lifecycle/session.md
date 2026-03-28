# Session Lifecycle

Save, close, reopen, and open-text flows should preserve the exact editor-visible state across transitions.

## Save And Close Sequences

### Save then close a fix and keep both diagnostic surfaces clean
Saving a fixed overlay should leave both document and workspace diagnostics empty after close.

```ds:main.ds
const value = ;
```

```ds:main.ds[1]
const value = 1;
```

```lsp document_diagnostic main.ds [0]
file=main.ds
range=0:14-0:15
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Declarator
```

```lsp workspace_diagnostic [0]
file=main.ds
range=0:14-0:15
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Declarator
```

```lsp save main.ds [1]
```

```lsp close main.ds [1]
```

```lsp document_diagnostic main.ds [1]
```

```lsp workspace_diagnostic [1]
```

### Save then close a broken overlay and keep both diagnostic surfaces exact
Saving a broken overlay should preserve the same error after the file closes.

```ds:main.ds
const value = 1;
```

```ds:main.ds[1]
const value = ;
```

```lsp save main.ds [1]
```

```lsp close main.ds [1]
```

```lsp document_diagnostic main.ds [1]
file=main.ds
range=0:14-0:15
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Declarator
```

```lsp workspace_diagnostic [1]
file=main.ds
range=0:14-0:15
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Declarator
```

### Close an unsaved broken overlay and return both diagnostic surfaces to clean
Closing an unsaved broken overlay should drop the transient document and workspace errors.

```ds:main.ds
const value = 1;
```

```ds:main.ds[1]
const value = ;
```

```lsp document_diagnostic main.ds [1]
file=main.ds
range=0:14-0:15
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Declarator
```

```lsp workspace_diagnostic [1]
file=main.ds
range=0:14-0:15
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Declarator
```

```lsp close main.ds [1]
```

```lsp document_diagnostic main.ds [1]
```

```lsp workspace_diagnostic [1]
```

### Save and reopen a symbol rename through open text
A saved document should keep its exact symbol answers after a close and open-text reopen.

```ds:main.ds
[|function /*def*/ping(): void {}|]

/*use*/ping();
```

```ds:main.ds[1]
[|function /*def*/pong(): void {}|]

/*use*/pong();
```

```lsp save main.ds [1]
```

```lsp close main.ds [1]
```

```lsp open_text main.ds [1]
[|function /*def*/pong(): void {}|]

/*use*/pong();
```

```lsp definition use [1]
def
```

```lsp document_symbols main.ds [1]
def|function
```

## Save And Reopen

### Save a library edit, close the consumer, and reopen into the new definition
Save and reopen should preserve the new definition target across files.

```ds:lib.ds
export [|function /*old_def*/ping(): void {}|]
```

```ds:main.ds
import { ping } from "./lib.ds";

/*use*/ping();
```

```ds:lib.ds[1]
export [|function /*new_def*/pong(): void {}|]
```

```ds:main.ds[1]
import { pong } from "./lib.ds";

/*use*/pong();
```

```lsp save lib.ds [1]
```

```lsp close main.ds [1]
```

```lsp open_text main.ds [1]
import { pong } from "./lib.ds";

/*use*/pong();
```

```lsp definition use [1]
new_def
```

### Repeat save, close, and reopen across multiple renames
Repeated reopen sequences should keep navigation aligned with each saved on-disk revision.

```ds:main.ds
[|function /*first_def*/ping(): void {}|]

/*use*/ping();
```

```ds:main.ds[1]
[|function /*second_def*/pong(): void {}|]

/*use*/pong();
```

```ds:main.ds[2]
[|function /*third_def*/call(): void {}|]

/*use*/call();
```

```ds:main.ds[3]
[|function /*fourth_def*/render(): void {}|]

/*use*/render();
```

```lsp save main.ds [1]
```

```lsp close main.ds [1]
```

```lsp open_text main.ds [1]
[|function /*second_def*/pong(): void {}|]

/*use*/pong();
```

```lsp definition use [1]
second_def
```

```lsp save main.ds [2]
```

```lsp close main.ds [2]
```

```lsp open_text main.ds [2]
[|function /*third_def*/call(): void {}|]

/*use*/call();
```

```lsp definition use [2]
third_def
```

```lsp save main.ds [3]
```

```lsp close main.ds [3]
```

```lsp open_text main.ds [3]
[|function /*fourth_def*/render(): void {}|]

/*use*/render();
```

```lsp definition use [3]
fourth_def
```

## File Creation

### Create a new consumer file and open it into the current definition
Opening a file that first appears in a later state should use the created on-disk text and resolve against the current workspace.

```ds:lib.ds
export [|function /*def*/ping(): void {}|]
```

```ds:new_consumer.ds[1]
import { ping } from "./lib.ds";

/*use*/ping();
```

```lsp open new_consumer.ds [1]
```

```lsp definition use [1]
def
```

### Save one broken file and close another fixed overlay
Saving one broken file and closing another fixed overlay should leave both on-disk broken files in workspace diagnostics.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = ;
```

```ds:a.ds[1]
const a = ;
```

```ds:b.ds[1]
const b = 2;
```

```lsp save a.ds [1]
```

```lsp close b.ds [1]
```

```lsp workspace_diagnostic [1]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Declarator

file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Declarator
```

### Reopen one changed consumer after a saved library retarget
Open text should pick up the saved library retarget and restore exact navigation.

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

```lsp save main.ds [1]
```

```lsp close main.ds [1]
```

```lsp open_text main.ds [1]
import { value } from "./beta.ds";
const current = /*use*/value;
```

```lsp definition use [1]
beta_def
```

## Save And Reopen

### Save one library rewrite and reopen two consumers into the new target
Saving the library rewrite should let two reopened consumers resolve to the new definition exactly.

```ds:lib.ds
export [|function /*old_def*/ping(): void {}|]
```

```ds:a.ds
import { ping } from "./lib.ds";

/*use_a*/ping();
```

```ds:b.ds
import { ping } from "./lib.ds";

/*use_b*/ping();
```

```ds:lib.ds[1]
export [|function /*new_def*/pong(): void {}|]
```

```ds:a.ds[1]
import { pong } from "./lib.ds";

/*use_a*/pong();
```

```ds:b.ds[1]
import { pong } from "./lib.ds";

/*use_b*/pong();
```

```lsp save lib.ds [1]
```

```lsp close a.ds [1]
```

```lsp close b.ds [1]
```

```lsp open_text a.ds [1]
import { pong } from "./lib.ds";

/*use_a*/pong();
```

```lsp open_text b.ds [1]
import { pong } from "./lib.ds";

/*use_b*/pong();
```

```lsp definition use_a [1]
new_def
```

```lsp definition use_b [1]
new_def
```
