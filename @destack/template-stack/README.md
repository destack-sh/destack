# @destack/template-stack

Start a space's stack from a template: a files bucket, a credentials vault, the baseline installations and an automatic rollback.

## Overview

The stack template declares what every new space starts with, and a new stack package copies and grows it.

- **personal**: the space's stack: the `files` bucket, the `credentials` vault, the vault, observability, notification, social and message installations, and an alert rule rolling an installation back when a new fatal issue opens.
- **files**: the shared bucket of documents and media.
- **credentials**: the vault of the space's application secrets.
- **account**: the stages the account's spaces share, `dev` and `production`.

```ts
import { credentials, files, personal } from "@destack/template-stack";

const resources = [files, credentials];
personal.definition; // the stack as JSON
```

## Examples

### Apply the stack

`destack apply` applies the `personal` export of the stack package's directory to the current space.

```sh
destack apply ./stack --export personal
```

### Roll back on fatal issues

The alert rule calls `rollBack` on the installation whose new issue is fatal, after notifying the space's managers.

```ts
import observabilityPackage from "@destack/observability/package";
import { defineStack } from "@destack/stack";

export const guarded = defineStack({
    types: [observabilityPackage],
    alertRules: {
        rollback: {
            title: "Roll back installation on new fatal issues",
            condition: { kind: "issue", on: "open", filter: "level = fatal" },
            actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
        },
    },
});
```
