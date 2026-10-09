# @destack/audit

`@destack/audit` is Google Cloud Audit Logs, with `activity`, `access` and `denial` calls as its Admin Activity, Data Access and Policy Denied logs kept 400 days, and `Journal` is Stripe's idempotency keys, running each request once and replaying its outcome to retries.

```ts
export const noteRename = defineAuditAction({ name: "note.rename", target, details }); // the log entry's methodName
await recorder.record(transaction, noteRename, { target, details, outcome: { kind: "success" } }); // Admin Activity
await recorder.read(openSecret, { target, details: {} }, () => secrets.get(id)); // Data Access
await calls.execute(request, fingerprint, { authorize, run }); // Stripe's idempotent request
await events.query({ kind: "call", scope: organisationId, within: true, where: 'outcome = "denied"' }); // Policy Denied
```

## Actions

`defineAuditAction` declares a `noun.verb` action with the one object it acts on and its recorded details.

```ts
export const noteRename = defineAuditAction({
    name: "note.rename",
    target: schema.object({ type: schema.literal("note"), id: schema.string() }),
    details: schema.object({ title: schema.string() }),
});
```

## Journal

`Journal` runs each request of one database once in one transaction, and `journal` is the table it keeps them in.

```ts
import { Journal } from "@destack/audit/server";
import { journal } from "@destack/audit/stack";

export const main = defineDatabase({ name: "main", tables: [journal, ...page.tables] });
const calls = new Journal(database, callKey);
await calls.replay(request, fingerprint); // the recorded results, or undefined before the request ran
```

## Recorders

`AuditRecorder` attributes calls to the verified caller, or to a principal running its own code, and `AuditRecorder.procedure` records each procedure call of a server.

```ts
const audit = AuditRecorder.service(calls, { package: pagesService.package, service: "pages" });
const recorder = audit(spaceId, context);
const own = audit(spaceId, { as: installation, component: "ReminderController" });
Server.start({ ...options, audit: AuditRecorder.procedure(({ context }) => recorderOf(context), { isAccessAudited }) });
```

## History

`AuditHistory` keeps each ended call once as a `call` event in an `@destack/event` store, which copies it to every enclosing scope.

```ts
import { AuditHistory } from "@destack/audit/history";

const history = new AuditHistory(store);
await new ControlLoop(database, [calls.controller(history)], { report }).run(signal);
await events.forget(Subject.key(person)); // the calls stay, their addresses gone
```
