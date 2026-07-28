# Rename Files

## Relative Import

### Rename one imported file

A relative import follows the renamed file and keeps its extension form.

```ds source/value.ds
export const value = 1;
```

```ds main.ds
import { value } from "./source/value";

const result = value;
```

```query rename_files
source/value.ds -> source/result.ds
```

```ds main.ds after
import { value } from "./source/result";

const result = value;
```

## Extensions

### Preserve an explicit extension

An explicit extension remains explicit.

```ds source/value.ds
export const value = 1;
```

```ds main.ds
import { value } from "./source/value.ds";

const result = value;
```

```query rename_files
source/value.ds -> source/result.ds
```

```ds main.ds after
import { value } from "./source/result.ds";

const result = value;
```

### [ignored] Preserve a dotted extensionless path

A dotted basename remains extensionless when the module extension was omitted.

```ds source/value.test.ds
export const value = 1;
```

```ds main.ds
import { value } from "./source/value.test";

const result = value;
```

```query rename_files
source/value.test.ds -> source/result.test.ds
```

```ds main.ds after
import { value } from "./source/result.test";

const result = value;
```

## Parent Relative Import

### Rename a file imported from a parent directory

A parent-relative import remains relative to the importing file.

```ds shared/value.ds
export const value = 1;
```

```ds application/main.ds
import { value } from "../shared/value";

const result = value;
```

```query rename_files
shared/value.ds -> shared/result.ds
```

```ds application/main.ds after
import { value } from "../shared/result";

const result = value;
```

## Multiple Files

### Rename multiple imported files

One edit updates every renamed file.

```ds source/left.ds
export const left = 1;
```

```ds source/right.ds
export const right = 2;
```

```ds main.ds
import { left } from "./source/left";
import { right } from "./source/right.ds";

const result = left + right;
```

```query rename_files
source/left.ds -> source/west.ds
source/right.ds -> source/east.ds
```

```ds main.ds after
import { left } from "./source/west";
import { right } from "./source/east.ds";

const result = left + right;
```

### Update multiple importing files

Every resolved reference to the renamed target receives its own relative rewrite.

```ds source/value.ds
export const value = 1;
```

```ds application/first.ds
import { value } from "../source/value";

export const first = value;
```

```ds application/nested/second.ds
import { value } from "../../source/value";

export const second = value;
```

```query rename_files
source/value.ds -> library/value.ds
```

```ds application/first.ds after
import { value } from "../library/value";

export const first = value;
```

```ds application/nested/second.ds after
import { value } from "../../library/value";

export const second = value;
```

## Directories

### Rename an imported directory

Every specifier below the renamed directory follows its new path.

```ds source/value.ds
export const value = 1;
```

```ds main.ds
import { value } from "./source/value";

const result = value;
```

```query rename_files
source -> library
```

```ds main.ds after
import { value } from "./library/value";

const result = value;
```

### Preserve paths inside a renamed directory

Moving an importer and its target together leaves their relative specifier unchanged.

```ds source/value.ds
export const value = 1;
```

```ds source/main.ds
import { value } from "./value";

const result = value;
```

```query rename_files
source -> library
@rename_files.none
```

### Prefer the most specific renamed path

An explicit file rename takes precedence over a renamed parent directory.

```ds source/value.ds
export const value = 1;
```

```ds main.ds
import { value } from "./source/value";
```

```query rename_files
source -> library
source/value.ds -> special/value.ds
```

```ds main.ds after
import { value } from "./special/value";
```

## Symbol Spaces

### Rename a module imported for a type

File renames preserve the plain import and its declaration symbol space.

```ds source/options.ds
export type Options = {
    enabled: boolean,
};
```

```ds main.ds
import { Options } from "./source/options";

declare const options: Options;
```

```query rename_files
source/options.ds -> source/configuration.ds
```

```ds main.ds after
import { Options } from "./source/configuration";

declare const options: Options;
```

## Re-Exports

### Rename a re-export target

File renames update re-export specifiers.

```ds source/value.ds
export const value = 1;
```

```ds barrel.ds
export { value } from "./source/value";
```

```query rename_files
source/value.ds -> source/result.ds
```

```ds barrel.ds after
export { value } from "./source/result";
```

### Rename a namespace re-export target

Namespace re-exports follow the renamed module.

```ds source/value.ds
export const value = 1;
```

```ds barrel.ds
export * as values from "./source/value";
```

```query rename_files
source/value.ds -> source/result.ds
```

```ds barrel.ds after
export * as values from "./source/result";
```

## Side Effects

### Rename a side-effect import target

Side-effect imports participate in module path rewrites.

```ds source/register.ds
export const registered = true;
```

```ds main.ds
import "./source/register";

const label = "./source/register";
```

```query rename_files
source/register.ds -> source/install.ds
```

```ds main.ds after
import "./source/install";

const label = "./source/register";
```

## Unresolved Specifiers

### Ignore an unresolved textual match

An unresolved specifier has no indexed target and cannot be rewritten from its text.

```ds main.ds
import { value } from "./source/missing";
```

```ds source/other.ds
export const other = 1;
```

```query rename_files
source -> library
@rename_files.none
```

## Importing Files

### Rename an importing file

Moving an importing file rewrites its relative specifiers from the new directory.

```ds source/value.ds
export const value = 1;
```

```ds main.ds
import { value } from "./source/value";

const result = value;
```

```query rename_files
main.ds -> application/main.ds
```

```ds main.ds after
import { value } from "../source/value";

const result = value;
```

### Rename both sides of an import

The rewritten specifier uses both new file locations.

```ds source/value.ds
export const value = 1;
```

```ds main.ds
import { value } from "./source/value";

const result = value;
```

```query rename_files
source/value.ds -> library/value.ds
main.ds -> application/main.ds
```

```ds main.ds after
import { value } from "../library/value";

const result = value;
```

## Quotes

### Preserve single quotes

Specifier rewrites do not change the surrounding quote style.

```ds source/value.ds
export const value = 1;
```

```ds main.ds
import { value } from './source/value';

const result = value;
```

```query rename_files
source/value.ds -> source/result.ds
```

```ds main.ds after
import { value } from './source/result';

const result = value;
```
