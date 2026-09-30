# @destack/audit

Declare audit actions, record them with the changes they describe, and query a scope's history.

## Actions

`defineAuditAction` declares a `Noun.verb` action with its affected objects and recorded details.

```ts
import { defineAuditAction } from "@destack/audit";

export const renameNote = defineAuditAction({
    name: "Note.rename",
    targets: schema.object({ note: schema.object({ type: schema.literal("note"), id: schema.string() }) }),
    details: schema.object({ title: schema.string() }),
});
```

## Recorders

An `AuditRecorder` attributes events to the verified caller and writes them to an `AuditOutbox`.

```ts
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";

const audit = AuditRecorder.from(context.caller, new AuditOutbox(database), {
    package: import.meta.destack.package,
    service: "notes",
    scope: spaceId,
    requestId: context.requestId,
});
```

## Recording

A recorder writes a database change's event in its transaction, an external effect's event as an attempt and its result, and a read as one access event.

```ts
await database.transaction(async (transaction) => {
    await transaction.update(note).set({ title }).where(eq(note.id, id));
    await audit.record(transaction, renameNote, { targets: { note: { type: "note", id } }, details: { title }, outcome: "success" });
});

await audit.attempt(sendInvitation, { targets, details: {} }, () => invitations.send(id));
const secret = await audit.read(openSecret, { targets, details: {} }, () => secrets.get(id));
```

Each event carries its category.

| Category | Records |
|---|---|
| `activity` | A committed write or an external effect. |
| `access` | A read of data, when the scope audits reads or the object type always does. |
| `denial` | A refused call. |

## Procedures

`AuditRecorder.procedure` records each procedure call of a server as one event when it ends, by the category the procedure declares, and every denial.

```ts
Server.start({ ...options, audit: AuditRecorder.procedure(({ context }) => recorderOf(context), { isAccessAudited }) });
```

## Delivery

An `AuditOutbox` delivers committed events to a history in batches through a `ControlLoop`.

```ts
import { createAuditClient } from "@destack/audit/client";
import { ControlLoop } from "@destack/service/control";

const outbox = new AuditOutbox(database);
await new ControlLoop(database, [outbox.controller(createAuditClient({ url, headers }))], { report }).run(signal);
```

## History

An `AuditHistory` stores each event once and lists, exports and prunes a scope's events.

```ts
import { AuditHistory } from "@destack/audit/history";
import { implementService } from "@destack/audit/server";

const history = new AuditHistory(historyDatabase);
Server.start({ ...implementService(history, { access, record }), ...hosting });

const page = await history.list({ scope: spaceId, limit: 100 });
await history.prune({ scope: spaceId, before: cutoff, limit: 100 });
```

## Storage

A service database holds the `outbox` every object type's tables include, and a history database holds `auditTables`.

```ts
export const main = defineDatabase({ name: "main", tables: note.tables });
export const history = defineDatabase({ name: "history", tables: auditTables });
```
