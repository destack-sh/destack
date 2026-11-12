# @destack/template-blank

Start a new Destack package from an empty package with one setting.

## Overview

The blank template is the smallest Destack package: a package handle and one setting, which a new package copies and grows.

- **Package**: the handle stacks install the package by, with no objects yet; `definePackage`.
- **language**: a setting for the language of generated content, set per user and overridden per space or installation.

```ts
import { language } from "@destack/template-blank/setting";
import blank from "@destack/template-blank/package";

blank.name; // "@destack/template-blank"
language.definition.default; // "en"
```

## Examples

### Install the package

A space installs the package like any other.

```sh
destack install @destack/template-blank
```

### Resolve the content language

`resolve` picks the language from the values placed along a user's scope chain.

```ts
import type { SettingValue } from "@destack/setting/object";
import { language } from "@destack/template-blank/setting";

(values: readonly SettingValue[], chain: readonly string[]) => {
    const { value } = language.resolve({ scope: "user-1" }, values, chain);
};
```
