# @destack/audit

Declare audit actions, record every executed call once in a journal, and keep, query and follow a scope's history as call events.

## Declarations

`defineAuditAction` declares a `noun.verb` action with the one object it acts on and its recorded details.

```ts
import { defineAuditAction } from "@destack/audit";

export const noteRename = defineAuditAction({
    name: "note.rename",
    target: schema.object({ type: schema.literal("note"), id: schema.string() }),
    details: schema.object({ title: schema.string() }),
});
```

## Journal

`Journal` runs each request of one database once in one transaction and replays its outcome to retries, and `Replay` decides which outcomes a retry replays.

```ts
import { Replay } from "@destack/audit";
import { Journal } from "@destack/audit/server";

const calls = new Journal(database, callKey);
const results = await calls.execute(request, fingerprint, {
    authorize: (transaction) => authorization.within(transaction).require(permission, target),
    run: (transaction) => updateAccount(transaction, input),
});
await calls.replay(request, fingerprint); // the recorded results, or undefined before the request ran
Replay.failure(new ServiceError("CONFLICT", { message: "name is taken" })); // { kind: "failure", error: { code: "CONFLICT", status: 409, … } }
```

### Clock

`clock` sets the time a journal checks request identifiers' retry periods and call lifetimes on, the system clock by default.

```ts
const calls = new Journal(database, callKey, { clock: server.clock });
```

### Journal table

`journal` is the table a database with journaled calls includes.

```ts
import { journal } from "@destack/audit/stack";

export const main = defineDatabase({ name: "main", tables: [journal, ...note.tables] });
```

## Recorders

`AuditRecorder` attributes calls to the verified caller and writes them to a journal.

```ts
import { AuditRecorder } from "@destack/audit/server";

const audit = AuditRecorder.service(calls, { package: notes.package, service: "notes" });
const recorder = audit(spaceId, context);
```

## Writes

`record` writes a write's call in its transaction as one `activity` call.

```ts
await database.transaction(async (transaction) => {
    await transaction.update(note).set({ title }).where(eq(note.id, id));
    await recorder.record(transaction, noteRename, {
        target: { type: "note", id },
        details: { title },
        outcome: { kind: "success" },
    });
});
```

## Effects and reads

`attempt` records an external effect as a running `activity` call and its outcome, and `read` records a read as one `access` call.

```ts
await recorder.attempt(sendInvitation, { target, details: {} }, () => invitations.send(id));
const secret = await recorder.read(openSecret, { target, details: {} }, () => secrets.get(id));
```

## Procedures

`AuditRecorder.procedure` records each procedure call of a server as one call when it ends, by the category the procedure declares, and every refusal as a `denial`.

```ts
Server.start({
    ...options,
    audit: AuditRecorder.procedure(({ context }) => recorderOf(context), { isAccessAudited }),
});
```

## Delivery

`Journal.controller` delivers each ended audited call once to a history, without its input and result values.

```ts
import { ControlLoop } from "@destack/service/control";

await new ControlLoop(database, [calls.controller(history)], { report }).run(signal);
```

## History

`AuditHistory` keeps each ended call once as an event of the `call` kind in the host's `EventStore`, copied to every scope enclosing its scope, which the event service reads.

```ts
import { AuditActor } from "@destack/audit";
import { AuditHistory } from "@destack/audit/history";

const history = new AuditHistory(store); // an EventStore keeping the call kind
await history.ingest({ calls }); // ended calls, each once
await events.query({
    kind: "call",
    scope: organisationId,
    within: true,
    where: 'outcome = "denied"',
});
await events.query({
    kind: "call",
    scope,
    where: `actor = ${JSON.stringify(AuditActor.key(actor))}`,
});
```

### Intake

`implementAudit` takes the ended calls another host's journal delivers, once its `intake` admits the sender for their scopes.

```ts
implementAudit({ history, intake: (context, scopes) => requireServing(context, scopes) });
```

### Retention

`forget` erases a person's addresses and user agents from the calls, which stay locked for 400 days.

```ts
await events.forget(Subject.key(person)); // the calls stay, their addresses gone
```

## Relays

`AuditHistory.relay` stores a batch of an instance's journal that its host relays, with the installation and the instance the host verified as each call's provenance.

```ts
await history.relay(batch, { scope: spaceId, packageId, installationId, instanceId }); // 2
// FORBIDDEN: a call of another space or package, or one with an installation, instance or machine
```

## Errors

A refused or failed audit operation throws an `AuditError`, and `toServiceError` returns the service error its caller receives.

```ts
import { AuditError } from "@destack/audit/error";

new AuditError("NOT_FOUND", "audited call not found").toServiceError(); // { code: "NOT_FOUND", message: "audited call not found" }
```
