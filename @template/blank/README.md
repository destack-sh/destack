# @template/blank

An empty Destack package that new packages start from.

## Setting

`language` declares the content language a user picks, and `readLanguage` resolves it from setting rows along a scope chain.

```ts
import { readLanguage } from "@template/blank";

const language = readLanguage(selection, rows, chain);
```
