# @destack/workflow

Run durable functions step by step as objects of the space they run in.

## Overview

`@destack/workflow` is durable execution after DBOS: workflows replayed from recorded steps, with runs as objects of their space.

Packages declare workflows:

- `Workflow`: a durable function, declared with `defineWorkflow`.
- `Step`: one recorded part of a run, made once however often the run replays.
- `Trigger`: starts runs on a schedule, an object's changes, or a webhook delivery.
- `defineWebhook`: a mount other services deliver to, verified with a secret.

Spaces keep the runs:

- `<workflow>-run`: one run of a workflow, pending, waiting, paused, completed, failed or cancelled.

Declare a workflow, implement it in steps, and start a run:

```ts
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import type { Subject } from "@destack/sync";
import { defineWorkflow, NO_OUTPUT } from "@destack/workflow";

// declare a workflow
export const summarize = defineWorkflow({
    name: "summarize",
    title: t`Summarize`,
    input: schema.object({ text: schema.string() }),
    output: NO_OUTPUT,
});

// implement it in recorded steps
export const served = summarize.implement({
    run: async (step, input) => {
        const words = await step.run("count", schema.number(), async () => input.text.length);
        await step.sleep("pause", { minutes: 5 });
    },
});

/** Start a run for a note, once per note. */
export function startSummary(actor: Subject) {
    return summarize.start({ text: "Hello" }, { key: "note-1", actor });
}
```

## Reference

### defineWorkflow

`defineWorkflow(definition)` declares a workflow, `implement({ run })` gives its function, and `start(input, { key, actor })` starts a run.

| Option            | What it is                                                      |
| ----------------- | --------------------------------------------------------------- |
| `input`, `output` | the schemas of a run's input and output, `NO_OUTPUT` for none   |
| `retries`         | the retry policy, such as `{ maximumAttempts }`, as in Temporal |
| `timeout`         | how long a run may take                                         |
| `expiring`        | how long an ended run is kept                                   |
| `limits`          | a `rate`, a `concurrency` of slots, and the `key` they count by |
| `triggers`        | the triggers starting runs                                      |

### Step

A run's function takes a step, and each verb records its result once.

| Verb                                            | What it does                                                        |
| ----------------------------------------------- | ------------------------------------------------------------------- |
| `call(name, call)`                              | calls an object method as the run's actor, waiting for any approval |
| `query(name, call)`                             | reads through an object query method                                |
| `run(name, schema, code)`                       | runs local code, recording its output                               |
| `sleep(name, duration)`, `sleepUntil(name, at)` | waits for a duration, or until a time                               |
| `waitFor(name, listing)`                        | waits for a listing's first object                                  |
| `approve(name, …)`                              | asks approvers to decide on an object                               |
| `invoke(name, workflow, input)`                 | starts another workflow's run, answering its output                 |

### Trigger

A trigger starts one run per occurrence.

| Kind         | Fires on                                                  |
| ------------ | --------------------------------------------------------- |
| `"schedule"` | a cron or interval, with a `concurrency` and a `deadline` |
| `"change"`   | an object type's changes, matching an optional `where`    |
| `"webhook"`  | a webhook's verified deliveries                           |

### defineWebhook

`defineWebhook({ name, template, signature, secret })` declares a mount verified with a vault secret's readable versions.
`StandardSignature` verifies Standard Webhooks signatures.

## Examples

### Call, wait and sleep in steps

A run calls methods as its actor, runs local code once, sleeps, and waits for an object.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { defineWorkflow } from "@destack/workflow";

// declare notes a draft waits on
const note = defineObject(
    {
        name: "note",
        plural: "notes",
        title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
        container: "scope",
        fields: { title: field.string() },
        access: { permissions: ["read", "write"] },
    },
    (method) => ({
        create: method.create("write"),
        get: method.get("read"),
        list: method.list("read"),
    }),
);

// declare a draft that waits for its approval note
const draft = defineWorkflow({
    name: "draft",
    title: t`Draft`,
    input: schema.object({ title: schema.string().min(1) }),
    output: schema.object({ approval: schema.string().nullable() }),
    waits: [note],
});

export const drafting = draft.implement({
    run: async (step, input, context) => {
        // write a note, and read it back
        const scope = context.scope;
        const written = await step.call(
            "write",
            note.call("create", { scope, title: input.title }),
        );
        const read = await step.query("read", note.call("get", { scope, id: written.id }));

        // pause five minutes, then wait an hour for an approval note
        await step.sleep("pause", { minutes: 5 });
        const where = { title: `Approved: ${read.title}` };
        const approval = await step.waitFor(
            "approval",
            note.call("list", { scope, where, limit: 1 }),
            {
                timeout: { hours: 1 },
            },
        );

        return { approval: approval?.title ?? null };
    },
});
```

### Fan out child runs

`invoke` starts another workflow's run once, and answers its output.

```ts
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { defineWorkflow } from "@destack/workflow";

// declare doubling, and a sum of two doubled numbers
const Value = schema.object({ value: schema.number().int() });
const double = defineWorkflow({ name: "double", title: t`Double`, input: Value, output: Value });
const sum = defineWorkflow({
    name: "sum",
    title: t`Sum`,
    input: schema.object({ left: schema.number().int(), right: schema.number().int() }),
    output: Value,
    waits: [double.run],
});

export const summing = sum.implement({
    run: async (step, input) => {
        // double both sides in child runs, then add them
        const left = await step.invoke("left", double, { value: input.left });
        const right = await step.invoke("right", double, { value: input.right });

        return { value: left.value + right.value };
    },
});
```

### Ask for approval

`approve` asks approvers to decide on an object, and answers once enough approve, one rejects, or time runs out.

```ts
import { t } from "@destack/locale";
import { approval } from "@destack/object";
import type { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import type { Subject } from "@destack/sync";
import { defineWorkflow } from "@destack/workflow";

/** Declare a review asking two approvers about one note. */
export function reviewWorkflow(notes: PackageId, approvers: readonly Subject[]) {
    const review = defineWorkflow({
        name: "review",
        title: t`Review`,
        input: schema.object({ id: schema.string() }),
        output: schema.enum(["requested", "approved", "rejected"]),
        waits: [approval],
    });

    return review.implement({
        run: (step, input, context) =>
            step.approve(
                "review",
                { packageId: notes, type: "note", scope: context.scope, id: input.id },
                { approvers, count: 2, timeout: { hours: 1 } },
            ),
    });
}
```

### Start a run per change

A change trigger starts one run per matching change of an object type.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { defineWorkflow, NO_OUTPUT } from "@destack/workflow";

// declare notes, and a triage run per new urgent note
const note = defineObject({
    name: "note",
    plural: "notes",
    title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
    container: "scope",
    fields: { title: field.string() },
});
export const triage = defineWorkflow({
    name: "triage",
    title: t`Triage`,
    input: schema.object({ at: schema.number() }),
    output: NO_OUTPUT,
    triggers: [
        {
            name: "urgent",
            kind: "change",
            change: { object: note, where: { title: "urgent" }, operations: ["create"] },
            input: (change) => ({ at: change.position.sequence }),
        },
    ],
});
```

### Limit and retry runs

Limits count runs per key, such as per person.

```ts
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { defineWorkflow } from "@destack/workflow";

// declare a digest with retries, a timeout, an expiry and limits
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

// declare a reminder started daily at nine
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

## Comparison

### DBOS

Workflows replayed from recorded steps basically follow DBOS.

| DBOS                 | `@destack/workflow` | Difference                                        |
| -------------------- | ------------------- | ------------------------------------------------- |
| `@DBOS.workflow()`   | `defineWorkflow`    | Declared by a package, implemented separately.    |
| `@DBOS.step()`       | `Step` verbs        | Steps call object methods as the run's actor.     |
| workflow status rows | `run` objects       | Runs have access, audit and sync like any object. |
| scheduled workflows  | triggers            | Also on object changes and webhook deliveries.    |
