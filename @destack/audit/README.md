# @destack/audit

Declare audit actions, record every executed call once in a journal, and query a scope's history.

## Declarations

`defineAuditAction` declares a `noun.verb` action with its affected objects and recorded details.

```ts
import { defineAuditAction } from "@destack/audit";

export const noteRename = defineAuditAction({
    name: "note.rename",
    targets: schema.object({
        note: schema.object({ type: schema.literal("note"), id: schema.string() }),
    }),
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

## Tables

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
        targets: { note: { type: "note", id } },
        details: { title },
        outcome: { kind: "success" },
    });
});
```

## Effects and reads

`attempt` records an external effect as a running `activity` call and its outcome, and `read` records a read as one `access` call.

```ts
await recorder.attempt(sendInvitation, { targets, details: {} }, () => invitations.send(id));
const secret = await recorder.read(openSecret, { targets, details: {} }, () => secrets.get(id));
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

`Journal.controller` delivers audited calls to a history in batches, without their input and result values: the history itself where its host runs the service, and the host relaying the journal where a workload runs it.

```ts
import { ControlLoop } from "@destack/service/control";

await new ControlLoop(database, [calls.controller(history)], { report }).run(signal);
```

### Chains

Each delivered call's context carries its `chain`, the scopes enclosing its scope outermost first, as the journal's scope copies keep them.

```ts
call.execution.context; // { scope: "space-…", chain: ["universe", "organisation-…", "account-…"], … }
```

## History

`AuditHistory` stores each call once and lists, exports and prunes a scope's calls.

```ts
import { AuditHistory } from "@destack/audit/history";
import { implementAudit } from "@destack/audit/server";

const history = new AuditHistory(historyDatabase);
Server.start({ ...implementAudit({ history, access, record }), ...hosting });

const page = await history.list({ scope: spaceId, limit: 100 });
await history.prune({ scope: spaceId, before: cutoff, limit: 100 });
```

### Scopes within

`within` reads the calls of a scope and of every scope inside it, by each call's chain.

```ts
const page = await client.list({
    scope: organisationId,
    within: true,
    outcome: "success",
    limit: 100,
});
page.cursor; // the position the next page continues after, null at the end
```

## Relays

`AuditHistory.relay` stores a batch of an instance's journal that its host relays, with the installation and the instance the host verified as each call's provenance.

```ts
await history.relay(batch, { scope: spaceId, packageId, installationId, instanceId }); // 2
// FORBIDDEN: a call of another space or package, or one naming an installation, instance or machine
```

## History tables

`auditTables` lists the tables of the history's database.

```ts
import { auditTables } from "@destack/audit/stack";

export const histories = defineDatabase({ name: "history", tables: auditTables });
```

## Errors

A refused or failed audit operation throws an `AuditError`, and `toServiceError` names the service error its caller receives.

```ts
import { AuditError } from "@destack/audit/error";

new AuditError("NOT_FOUND", "audited call not found").toServiceError(); // { code: "NOT_FOUND", message: "audited call not found" }
```
