# @destack/audit

Declare audit actions, record every executed call once in a journal, and query a scope's history.

## Actions

`defineAuditAction` declares a `noun.verb` action with its affected objects and recorded details.

```ts
import { defineAuditAction } from "@destack/audit";

export const renameNote = defineAuditAction({
    name: "note.rename",
    targets: schema.object({
        note: schema.object({ type: schema.literal("note"), id: schema.string() }),
    }),
    details: schema.object({ title: schema.string() }),
});
```

## Journal

`Journal` runs each request of one database once in one transaction and replays its outcome to retries.

```ts
import { Journal, journal } from "@destack/audit";

const calls = new Journal(database, callKey);
const results = await calls.execute(request, fingerprint, {
    authorize: (transaction) => authorization.within(transaction).require(permission, target),
    run: (transaction) => updateAccount(transaction, input),
});
```

## Tables

`journal` is the table a database with journaled calls includes.

```ts
export const main = defineDatabase({ name: "main", tables: [journal, ...note.tables] });
```

## Recorders

`AuditRecorder` attributes calls to the verified caller and writes them to a journal.

```ts
import { AuditRecorder } from "@destack/audit";

const audit = AuditRecorder.service(calls, { package: notes.package, service: "notes" });
const recorder = audit(spaceId, context);
```

## Writes

`record` writes a write's call in its transaction as one `activity` call.

```ts
await database.transaction(async (transaction) => {
    await transaction.update(note).set({ title }).where(eq(note.id, id));
    await recorder.record(transaction, renameNote, {
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

`Journal.controller` delivers audited calls to a history in batches, without their input and result values.

```ts
import { createAuditClient } from "@destack/audit/client";
import { ControlLoop } from "@destack/service/control";

await new ControlLoop(database, [calls.controller(createAuditClient({ url, headers }))], {
    report,
}).run(signal);
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

## History tables

`auditTables` lists the tables of the history's database.

```ts
import { auditTables } from "@destack/audit/stack";

export const histories = defineDatabase({ name: "history", tables: auditTables });
```
