# @destack/workflow

`@destack/workflow` runs durable functions step by step as objects of the space they run in: each advance runs the function again, and each step answers what it recorded, as DBOS, Inngest and Restate memoise theirs.

```ts
const summarize = defineWorkflow({
    name: "summarize",
    input: schema.object({ id: schema.string() }),
    output: NO_OUTPUT,
});
const served = summarize.implement(async (step, input) => {
    const note = await step.query("read", note.call("get", { spaceId, id: input.id }));
    await step.sleep("pause", { minutes: 5 });
    await step.call("write", summary.call("create", { spaceId, text: note.title }));
});
await store.mutate(caller, {
    id: RequestId.create(store.environment),
    calls: [summarize.start({ id }, { key: id, actor: caller.subject })],
});
```

## Workflows and controllers

A workflow runs a process that someone starts, and a controller keeps objects in the state their fields describe.

```ts
defineWorkflow({ name: "publish", input, output: NO_OUTPUT }); // ask for approval, wait a day, then publish
reminder.implement({ controller: { pending: { sentAt: { isNull: true } }, reconcile } }); // send when due
// both change state only through the methods of the type that keeps it
```

## Steps

A step answers its recorded output on replay, so a run makes each call once, however often it advances; anything nondeterministic between steps goes through `step.run`, as the lint and the replay refuse it elsewhere.

```ts
await step.call("charge", invoice.call("pay", target)); // an object method call as the run's actor
await step.call("publish", note.call("publish", target)); // waits on a change request when the grant asks approval
await step.query("read", note.call("get", target)); // a read, answered the same on replay
await step.run("count", schema.number(), async () => words(text)); // local code, recorded by its schema
await step.sleepUntil("morning", nine);
const approval = await step.waitFor("approval", note.call("list", { spaceId, where }), { timeout });
const total = await step.invoke("child", count, { ids }); // a child run, answering its output
await step.invoke("notify", notify, input, { onParentClose: "abandon" }); // runs on after its parent ends
```

## Runs

A run is due, waits, is paused or has ended; its actor pauses, resumes, cancels or resets it, and the space's controllers advance it.

```ts
summarize.start(input, { key: id, mode: "replace", at: tomorrow, actor });
summarize.run.call("reset", { ...target, step: "write" }); // run again from a step
defineWorkflow({
    name: "digest",
    input,
    output: schema.string(), // every workflow declares its output, NO_OUTPUT for none
    scope: "universe", // runs live in the universe, a space by default
    retries: { maximumAttempts: 3 }, // Temporal's RetryPolicy and its defaults
    timeout: { hours: 1 },
    expiring: { after: { days: 7 } },
    limits: {
        rate: { count: 10, every: { minutes: 1 } },
        concurrency: { slots: 1 },
        key: "caller",
    },
    singleton: true, // a run takes its slot from its start to its end
});
```

## Tasks

`Task` makes one call once, now or at a time, as the run's actor, as Cloud Tasks and Convex's `scheduler.runAt` do, through a task workflow of the package's own.

```ts
const task = defineWorkflow({ name: "task", input: TaskInput, output: TaskOutput });
const served = task.implement((step, input) => new Task(step, input).run());
await context.mutate(context.self, [
    task.start({ call: reminder.call("send", target) }, { at, actor }),
]);
```

## Triggers

A trigger starts runs on a schedule, an object's changes or signed webhook deliveries, one run per occurrence, change or delivery.

```ts
defineWorkflow({
    name: "remind",
    input,
    output: NO_OUTPUT,
    triggers: {
        daily: {
            kind: "schedule", // a Kubernetes CronJob: cron, interval or once
            schedule: {
                timing: { timing: "cron", cron: "0 9 * * *", timezone },
                concurrency: "forbid",
                deadline,
            },
            input: (occurrence) => ({ at: occurrence.at }),
        },
        published: {
            kind: "change",
            change: { object: note, where: { status: "published" }, operations: ["create"] },
            input: (change) => ({ id: change.after.id }),
        },
        pushes: {
            kind: "webhook", // Standard Webhooks, or a scheme of the sender's own
            webhook: { signature: new StandardSignature(), path: "/{repository}", secret },
            input: (delivery) => ({ repository: delivery.parameters.repository }),
        },
    },
});
remind.trigger.call("pause", target); // a paused change trigger keeps its changes six days at least
remind.trigger.call("reset", target); // start a stopped trigger over as a new one begins
```

## Approvals

`step.approve` asks approvers to decide on an object, and a step's call that waits for approvals opens object's change request and waits until it applies.

```ts
const decision = await step.approve("review", subject, { approvers: [bob, alice], count: 2 }); // principals, never groups
await step.call("publish", publish); // refused for approvals: a change request asks the grant's delegator, then answers
```
