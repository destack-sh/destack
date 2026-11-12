# @destack/workflow

Run durable functions step by step as objects of the space they run in.

## Overview

A package declares its workflows, and each run is an object of the space it runs in: every advance runs the function again, and each step answers what it recorded.

- **Workflow**: a durable function with its input, output, retries, timeout and limits; `defineWorkflow`, then `implement` and `start`.
- **Run**: one start of a workflow, due, waiting, paused or ended; its actor pauses, resumes, cancels or resets it.
- **Step**: one recorded part of a run: `call`, `query`, `run`, `sleep`, `waitFor`, `invoke` and `approve`, each made once however often the run advances.
- **Trigger**: what starts runs on a schedule, an object's changes or webhook deliveries, one run per occurrence.
- **Webhook**: a mount other services deliver to, each delivery verified with its bound secret before a trigger sees it; `defineWebhook`.

```ts
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import type { Subject } from "@destack/sync";
import { defineWorkflow, NO_OUTPUT } from "@destack/workflow";

export const summarize = defineWorkflow({
    name: "summarize",
    title: t`Summarize`,
    input: schema.object({ text: schema.string() }),
    output: NO_OUTPUT,
});
export const served = summarize.implement({
    run: async (step, input) => {
        const words = await step.run("count", schema.number(), async () => input.text.length);
        await step.sleep("pause", { minutes: 5 });
    },
});

(actor: Subject) => summarize.start({ text: "Hello" }, { key: "note-1", actor });
```

## Examples

### Limit and retry runs

A workflow declares its retries, timeout, expiry and limits, after Temporal's retry policy.

```ts
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { defineWorkflow } from "@destack/workflow";

export const digest = defineWorkflow({
    name: "digest",
    title: t`Digest`,
    input: schema.object({ person: schema.string() }),
    output: schema.object({ sent: schema.number() }),
    retries: { maximumAttempts: 3 },
    timeout: { hours: 1 },
    expiring: { after: { days: 7 } },
    limits: {
        rate: { count: 10, every: { minutes: 1 } },
        concurrency: { slots: 1, until: "end" },
        key: { input: "person" },
    },
});
```

### Start runs on a schedule

A schedule trigger starts one run per occurrence, as a Kubernetes CronJob does.

```ts
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { defineWorkflow, NO_OUTPUT } from "@destack/workflow";

export const remind = defineWorkflow({
    name: "remind",
    title: t`Remind`,
    input: schema.object({ at: schema.number() }),
    output: NO_OUTPUT,
    triggers: [
        {
            name: "daily",
            kind: "schedule",
            schedule: {
                timing: { timing: "cron", cron: "0 9 * * *", timezone: "Europe/Vienna" },
                concurrency: "forbid",
                deadline: 60_000,
            },
            input: (occurrence) => ({ at: occurrence.at }),
        },
    ],
});
```

### Receive webhooks

`defineWebhook` declares a mount that verifies each delivery with a vault secret's live versions.

```ts
import type { ResourceHandle, Secret } from "@destack/resource";
import { defineWebhook, StandardSignature } from "@destack/workflow";

(secret: ResourceHandle<Secret>) =>
    defineWebhook({
        name: "pushes",
        template: "/{repository}",
        signature: new StandardSignature(),
        secret,
    });
```

## Comparison

### DBOS

Workflows replayed from recorded steps follow DBOS; three things differ.

- A run is an object of its space, with its access, audit and sync, and it changes state only through object methods.
- A step's call waits on a change request when the actor's grant asks for approval.
- Triggers start runs on a schedule, an object's changes or a verified webhook delivery, all declared beside the workflow.
