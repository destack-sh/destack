# @destack/template-blank

An empty Destack package that new packages start from.

## Setting

`language` declares the content language a user picks; resolve it along a scope chain, nearest scope first:

```ts
import { language } from "@destack/template-blank/setting";

const { value } = language.resolve(selection, rows, chain);
```
