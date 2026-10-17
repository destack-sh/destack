# @destack/workflow

`@destack/workflow` runs durable functions step by step as objects of the space they run in: each advance runs the function again, and each step answers what it recorded, as DBOS, Inngest and Restate memoise theirs.

```ts
const summarize = defineWorkflow({
    name: "summarize",
    input: schema.object({ id: schema.string() }),
});
const served = summarize.implement(async (step, input) => {
    const note = await step.query("read", note.call("get", { spaceId, id: input.id }));
    await step.sleep("pause", { minutes: 5 });

    return step.call("write", summary.call("create", { spaceId, text: note.title }));
});
await store.mutate(caller, {
    id: RequestId.create(),
    calls: [summarize.start({ id }, { key: id, actor: caller.subject })],
});
```

## Steps

A step answers its recorded output on replay, so a run makes each call once, however often it advances; anything nondeterministic between steps goes through `step.run`, as the lint and the replay refuse it elsewhere.

```ts
await step.call("charge", invoice.call("pay", target)); // an object method call as the run's actor
await step.query("read", note.call("get", target)); // a read, answered the same on replay
await step.run("count", schema.number(), async () => words(text)); // local code, recorded by its schema
await step.sleepUntil("morning", nine);
const approval = await step.waitFor("approval", note.call("list", { spaceId, where }), { timeout });
const total = await step.invoke("child", count, { ids }); // a child run, answering its output
```

## Runs

A run is due, waits, is paused or has ended; its actor pauses, resumes, cancels or resets it, and the space's controllers advance it.

```ts
summarize.start(input, { key: id, mode: "replace", at: tomorrow, actor });
summarize.start(input, { actor: space }); // acts for the space within its grant to the installation
summarize.run.call("reset", { ...target, step: "write" }); // run again from a step
defineWorkflow({
    name: "digest",
    input,
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

## Scheduled calls

`ScheduledCall` makes one call once, now or at a time, as the run's actor, as Convex's `scheduler.runAt` does, through a workflow of the package's own.

```ts
const later = defineWorkflow({
    name: "call",
    input: ScheduledCall.input,
    output: ScheduledCall.output,
});
const served = later.implement(ScheduledCall.run);
await context.mutate(context.self, [
    ScheduledCall.send(later, reminder.call("send", target), { at }),
]);
```

## Triggers

A trigger starts runs on a schedule, an object's changes or signed webhook deliveries, one run per occurrence, change or delivery.

```ts
defineWorkflow({
    name: "remind",
    input,
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
```

## Approvals

`step.approve` asks approvers to decide on an object, and a change request makes a call once they approve it, as a pull request merges once approved.

```ts
const decision = await step.approve("review", subject, { approvers: [bob, alice], count: 2 });
await client.changeRequest.create({ spaceId, parent, call: rename, approvers: [bob] });
await client.changeRequest.create({ spaceId, call: publish }); // after APPROVAL_REQUIRED: asks the grant's delegator
await client.approval.approve(approval.target(asked));
```
