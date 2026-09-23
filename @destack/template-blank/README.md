Start a TypeScript package with a shared stack dependency.

```ts
import { ClientContext } from "@destack/service/client";
import { settings } from "./connection/index.ts";
import { readLanguage } from "./index.ts";

const context = new ClientContext(configuration, transport);
context.bind(settings);
const language = await readLanguage(target, context.resources);
```
