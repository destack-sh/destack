
## Relative Import

### Rename one imported file

A relative import follows the renamed file across edits, links, and definition navigation.

```tspp source/value.tspp
export function greet(name: string): string {
^ declaration:start
                ^^^^^ definition
    return name;
}
^ declaration:end
```

```tspp main.tspp
import { greet } from "./source/value";
                      ^^^^^^^^^^^^^^^^ specifier

const message = greet("World");
                ^^^^^ reference
```

```query goto_definition main.tspp#reference
@goto_definition.target origin=main.tspp#reference location=source/value.tspp#declaration selection=source/value.tspp#definition symbol=source/value.tspp#greet@1
```

```query rename_files apply
source/value.tspp -> source/result.tspp
```

```tspp main.tspp after
import { greet } from "./source/result";
                      ^^^^^^^^^^^^^^^^^ specifier

const message = greet("World");
                ^^^^^ reference
```

```move source/value.tspp source/result.tspp
```

```query links main.tspp
@links.link range=main.tspp#specifier path=source/result.tspp
```

```query goto_definition main.tspp#reference
@goto_definition.target origin=main.tspp#reference location=source/result.tspp#declaration selection=source/result.tspp#definition symbol=source/result.tspp#greet@1
```

### Rename an already renamed import

Each rename uses the current file paths.

```tspp source/value.tspp
export const value = 1;
```

```tspp main.tspp
import { value } from "./source/value";

const result = value;
```

```query rename_files
source/value.tspp -> source/result.tspp
```

```tspp main.tspp after
import { value } from "./source/result";

const result = value;
```

```move source/value.tspp source/result.tspp
```

```tspp main.tspp change
import { value } from "./source/result";

const result = value;
```

```query rename_files
source/result.tspp -> source/final.tspp
```

```tspp main.tspp after
import { value } from "./source/final";

const result = value;
```

## Extensions

### Preserve an explicit extension

An explicit extension remains explicit.

```tspp source/value.tspp
export const value = 1;
```

```tspp main.tspp
import { value } from "./source/value.tspp";

const result = value;
```

```query rename_files
source/value.tspp -> source/result.tspp
```

```tspp main.tspp after
import { value } from "./source/result.tspp";

const result = value;
```

### Preserve a dotted extensionless path

A dotted basename remains extensionless when the module extension was omitted.

```tspp source/value.generated.tspp
export const value = 1;
```

```tspp main.tspp
import { value } from "./source/value.generated";

const result = value;
```

```query rename_files
source/value.generated.tspp -> source/result.generated.tspp
```

```tspp main.tspp after
import { value } from "./source/result.generated";

const result = value;
```

## Parent Relative Import

### Rename a file imported from a parent directory

A parent-relative import remains relative to the importing file.

```tspp shared/value.tspp
export const value = 1;
```

```tspp application/main.tspp
import { value } from "../shared/value";

const result = value;
```

```query rename_files
shared/value.tspp -> shared/result.tspp
```

```tspp application/main.tspp after
import { value } from "../shared/result";

const result = value;
```

## Multiple Files

### Rename multiple imported files

One edit updates every renamed file.

```tspp source/left.tspp
export const left = 1;
```

```tspp source/right.tspp
export const right = 2;
```

```tspp main.tspp
import { left } from "./source/left";
import { right } from "./source/right.tspp";

const result = left + right;
```

```query rename_files
source/left.tspp -> source/west.tspp
source/right.tspp -> source/east.tspp
```

```tspp main.tspp after
import { left } from "./source/west";
import { right } from "./source/east.tspp";

const result = left + right;
```

### Update multiple importing files

Every resolved reference to the renamed target receives its own relative rewrite.

```tspp source/value.tspp
export const value = 1;
```

```tspp application/first.tspp
import { value } from "../source/value";

export const first = value;
```

```tspp application/nested/second.tspp
import { value } from "../../source/value";

export const second = value;
```

```query rename_files
source/value.tspp -> library/value.tspp
```

```tspp application/first.tspp after
import { value } from "../library/value";

export const first = value;
```

```tspp application/nested/second.tspp after
import { value } from "../../library/value";

export const second = value;
```

## Directories

### Rename an imported directory

Every specifier below the renamed directory follows its new path.

```tspp source/value.tspp
export const value = 1;
```

```tspp main.tspp
import { value } from "./source/value";

const result = value;
```

```query rename_files
source -> library
```

```tspp main.tspp after
import { value } from "./library/value";

const result = value;
```

### Preserve paths inside a renamed directory

Moving an importer and its target together leaves their relative specifier unchanged.

```tspp source/value.tspp
export const value = 1;
```

```tspp source/main.tspp
import { value } from "./value";

const result = value;
```

```query rename_files
source -> library
@rename_files.none
```

### Prefer the most specific renamed path

An explicit file rename takes precedence over a renamed parent directory.

```tspp source/value.tspp
export const value = 1;
```

```tspp main.tspp
import { value } from "./source/value";
```

```query rename_files
source -> library
source/value.tspp -> special/value.tspp
```

```tspp main.tspp after
import { value } from "./special/value";
```

## Symbol Spaces

### Rename a module imported for a type

File renames preserve the plain import and its declaration symbol space.

```tspp source/options.tspp
export type Options = {
    enabled: boolean,
};
```

```tspp main.tspp
import { Options } from "./source/options";

declare const options: Options;
```

```query rename_files
source/options.tspp -> source/configuration.tspp
```

```tspp main.tspp after
import { Options } from "./source/configuration";

declare const options: Options;
```

## Re-Exports

### Rename a re-export target

File renames update re-export specifiers.

```tspp source/value.tspp
export const value = 1;
```

```tspp barrel.tspp
export { value } from "./source/value";
```

```query rename_files
source/value.tspp -> source/result.tspp
```

```tspp barrel.tspp after
export { value } from "./source/result";
```

### Rename a namespace re-export target

Namespace re-exports follow the renamed module.

```tspp source/value.tspp
export const value = 1;
```

```tspp barrel.tspp
export * as values from "./source/value";
```

```query rename_files
source/value.tspp -> source/result.tspp
```

```tspp barrel.tspp after
export * as values from "./source/result";
```

## Side Effects

### Rename a side-effect import target

Side-effect imports participate in module path rewrites.

```tspp source/register.tspp
export const registered = true;
```

```tspp main.tspp
import "./source/register";

const label = "./source/register";
```

```query rename_files
source/register.tspp -> source/install.tspp
```

```tspp main.tspp after
import "./source/install";

const label = "./source/register";
```

## Unresolved Specifiers

### Ignore an unresolved textual match

An unresolved specifier has no indexed target and cannot be rewritten from its text.

```tspp main.tspp
import { value } from "./source/missing";
```

```tspp source/other.tspp
export const other = 1;
```

```query rename_files
source -> library
@rename_files.none
```

## Importing Files

### Rename an importing file

Moving an importing file rewrites its relative specifiers from the new directory.

```tspp source/value.tspp
export const value = 1;
```

```tspp main.tspp
import { value } from "./source/value";

const result = value;
```

```query rename_files
main.tspp -> application/main.tspp
```

```tspp main.tspp after
import { value } from "../source/value";

const result = value;
```

### Rename both sides of an import

The rewritten specifier uses both new file locations.

```tspp source/value.tspp
export const value = 1;
```

```tspp main.tspp
import { value } from "./source/value";

const result = value;
```

```query rename_files
source/value.tspp -> library/value.tspp
main.tspp -> application/main.tspp
```

```tspp main.tspp after
import { value } from "../library/value";

const result = value;
```

## Quotes

### Preserve single quotes

Specifier rewrites do not change the surrounding quote style.

```tspp source/value.tspp
export const value = 1;
```

```tspp main.tspp
import { value } from './source/value';

const result = value;
```

```query rename_files
source/value.tspp -> source/result.tspp
```

```tspp main.tspp after
import { value } from './source/result';

const result = value;
```
