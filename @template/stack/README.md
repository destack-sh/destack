# @template/stack

Declare the base configuration of a space.

## Space

`personal` declares a space holding the `main` database, the `files` bucket and the `credentials` vault under the package and network policies.

```ts
import { defineSpace } from "@destack/space";
import { credentials, database, files, network, packages } from "@template/stack";

export const personal = defineSpace({
    policies: { packages, network },
    resources: {
        main: { declaration: database, retention: { within: { days: 30 } }, tags: {} },
        files: { declaration: files, retention: { within: { days: 30 } }, tags: {} },
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
});
```

## Alerts

`rollback` declares an alert rule each installation of the stack keeps in its space, rolling an installation back to its previous revision when a new fatal issue opens.

```ts
export const rollback = defineAlertRule({
    name: "Roll back new fatal issues",
    condition: { kind: "issue", on: "open", filter: "level = fatal" },
    actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
});
```
