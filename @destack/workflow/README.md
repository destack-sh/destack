# @destack/workflow

`@destack/workflow` is DBOS: `defineWorkflow` is `@DBOS.workflow()`, each `Step` verb is a `@DBOS.step()`, and a run is a `<workflow>-run` object of its space.

```ts
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import type { Subject } from "@destack/sync";
import { defineWorkflow, NO_OUTPUT } from "@destack/workflow";
import { implementWorkflow } from "@destack/workflow/server";

// declared apart from its implementation, unlike @DBOS.workflow()
export const summarize = defineWorkflow({
    name: "summarize",
    title: t`Summarize`,
    input: schema.object({ text: schema.string() }),
    output: NO_OUTPUT,
});

export const served = implementWorkflow(summarize, {
    run: async (step, input) => {
        // recorded once across replays, as @DBOS.step()
        await step.run("count", schema.number(), async () => input.text.length);
        await step.sleep("pause", { minutes: 5 });
    },
});

// a key in place of DBOS's workflow ID: one live run per key
export function startSummary(actor: Subject) {
    return summarize.start({ text: "Hello" }, { key: "note-1", actor });
}
```

## Call and wait for objects

`step.call`, `step.query` and `step.waitFor` call object methods as the run's actor, recording each answer.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { defineWorkflow } from "@destack/workflow";
import { implementWorkflow } from "@destack/workflow/server";

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

const draft = defineWorkflow({
    name: "draft",
    title: t`Draft`,
    input: schema.object({ title: schema.string().min(1) }),
    output: schema.object({ approval: schema.string().nullable() }),
    waits: [note],
});

export const drafting = implementWorkflow(draft, {
    run: async (step, input, context) => {
        const { scope } = context;
        const written = await step.call("write", note.call("create", { scope, title: input.title }));
        const read = await step.query("read", note.call("get", { scope, id: written.id }));
        const where = { title: `Approved: ${read.title}` };
        const listing = note.call("list", { scope, where, limit: 1 });
        const approval = await step.waitFor("approval", listing, { timeout: { hours: 1 } });

        return { approval: approval?.title ?? null };
    },
});
```

## Place work on a runner

`step.place` runs a function a runner machine registers by name, once, and answers its output when the runner ends the step.

```ts
import { principal } from "@destack/access";
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { defineWorkflow, recorded } from "@destack/workflow";
import { implementWorkflow } from "@destack/workflow/server";

const render = defineWorkflow({
    name: "render",
    title: t`Render`,
    input: schema.object({ machine: schema.identifier("machine"), page: schema.string() }),
    output: schema.object({ bytes: schema.number() }),
});

export const rendering = implementWorkflow(render, {
    run: (step, input) => {
        const runner = principal.machine.reference(Scope.universe.id, input.machine);
        const work = { function: "render.page", input: recorded({ page: input.page }) };

        return step.place("render", schema.object({ bytes: schema.number() }), work, runner);
    },
});
```

## Start child runs

`step.invoke` starts another workflow's run once and answers its output.

```ts
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { defineWorkflow } from "@destack/workflow";
import { implementWorkflow } from "@destack/workflow/server";

const Value = schema.object({ value: schema.number().int() });
const double = defineWorkflow({ name: "double", title: t`Double`, input: Value, output: Value });
const sum = defineWorkflow({
    name: "sum",
    title: t`Sum`,
    input: schema.object({ left: schema.number().int(), right: schema.number().int() }),
    output: Value,
    waits: [double.run],
});

export const summing = implementWorkflow(sum, {
    run: async (step, input) => {
        const left = await step.invoke("left", double, { value: input.left });
        const right = await step.invoke("right", double, { value: input.right });

        return { value: left.value + right.value };
    },
});
```

## Ask for approval

`step.approve` asks approvers to decide on an object and answers once enough approve, one rejects, or time runs out.

```ts
import { t } from "@destack/locale";
import { approval } from "@destack/object";
import type { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import type { Subject } from "@destack/sync";
import { defineWorkflow } from "@destack/workflow";
import { implementWorkflow } from "@destack/workflow/server";

export function reviewWorkflow(notes: PackageId, approvers: readonly Subject[]) {
    const review = defineWorkflow({
        name: "review",
        title: t`Review`,
        input: schema.object({ id: schema.string() }),
        output: schema.enum(["requested", "approved", "rejected"]),
        waits: [approval],
    });

    return implementWorkflow(review, {
        run: (step, input, context) =>
            step.approve(
                "review",
                { packageId: notes, type: "note", scope: context.scope, id: input.id },
                { approvers, count: 2, timeout: { hours: 1 } },
            ),
    });
}
```

## Start a run per change

A `change` trigger starts one run per matching change of an object type.

```ts
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { defineWorkflow, NO_OUTPUT } from "@destack/workflow";

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

## Start runs on a schedule

A `schedule` trigger starts one run per occurrence, as a Kubernetes CronJob does.

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

## Start a run per webhook delivery

A `webhook` trigger starts one run per delivery a `defineWebhook` mount verifies, such as with `StandardSignature`.

```ts
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { defineWorkflow, NO_OUTPUT, type Webhook } from "@destack/workflow";

export function deliveries(webhook: Webhook) {
    return defineWorkflow({
        name: "deliver",
        title: t`Deliver`,
        input: schema.object({ event: schema.string() }),
        output: NO_OUTPUT,
        triggers: [
            { name: "pushed", kind: "webhook", webhook, input: (delivery) => ({ event: delivery.event }) },
        ],
    });
}
```

## Limit and retry runs

`limits` count runs per key, and `retries`, `timeout` and `expiring` bound each run.

```ts
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { defineWorkflow } from "@destack/workflow";

export const digest = defineWorkflow({
    name: "digest",
    title: t`Digest`,
    input: schema.object({ person: schema.string() }),
    output: schema.object({ sent: schema.number() }),
    retries: { maximumAttempts: 3 }, // Temporal's RetryPolicy
    timeout: { hours: 1 },
    expiring: { after: { days: 7 } },
    limits: {
        rate: { count: 10, every: { minutes: 1 } },
        concurrency: { slots: 1, until: "end" },
        key: { input: "person" },
    },
});
```
