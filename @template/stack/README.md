# @template/stack

Declare the base configuration of a space.

## Space

`personal` declares a space holding the `files` bucket and the `credentials` vault, and an alert rule rolling an installation back to its previous revision when a new fatal issue opens.

```ts
import type {} from "@destack/observability/stack";
import { defineStack } from "@destack/stack";
import { credentials, files } from "@template/stack";

export const personal = defineStack({
    resources: {
        files: { declaration: files, retention: { within: { days: 30 } }, tags: {} },
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
    alertRules: {
        rollback: {
            title: "Roll back installation on new fatal issues",
            condition: { kind: "issue", on: "open", filter: "level = fatal" },
            actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
        },
    },
});
```
