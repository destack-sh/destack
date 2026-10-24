# @template/stack

Declare the base configuration of a space.

## Space

`personal` declares a space holding the `files` bucket and the `credentials` vault, and an alert rule rolling an installation back to its previous revision when a new fatal issue opens.

```ts
import observabilityPackage from "@destack/observability/package";
import { defineStack } from "@destack/stack";
import vaultPackage from "@destack/vault/package";
import { credentials, files } from "@template/stack";

export const personal = defineStack([vaultPackage, observabilityPackage], {
    buckets: { files: files.declare({ retention: { within: { days: 30 } } }) },
    vaults: { credentials: credentials.declare({ retention: { within: { days: 30 } } }) },
    alertRules: {
        rollback: {
            title: "Roll back installation on new fatal issues",
            condition: { kind: "issue", on: "open", filter: "level = fatal" },
            actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
        },
    },
});
```
