# @destack/template-stack

Start a space's stack from a template: a files bucket, a credentials vault, the baseline installations and an alert on fatal issues.

## Overview

`@destack/template-stack` is the stack every new space starts from, to copy and grow.

It exports:

- `personal`: the space's stack: a bucket, a vault, the baseline installations, and an alert on fatal issues.
- `files`: the bucket of documents and media.
- `credentials`: the vault of application secrets.
- `account`: the stages the account's spaces share, `dev` and `production`.

Apply the `personal` stack to the current space:

```sh
destack apply ./stack --export personal
```

## Examples

### Alert on fatal issues

An alert rule notifies the space's managers of a new fatal issue.

```ts
import observabilityPackage from "@destack/observability/package";
import { defineStack } from "@destack/stack";

// notify the managers of each new fatal issue
export const guarded = defineStack({
    types: [observabilityPackage],
    alertRules: {
        fatal: {
            title: "Notify on new fatal issues",
            condition: { kind: "issue", on: "open", filter: "level = fatal" },
            actions: [{ kind: "notify" }],
        },
    },
});
```
