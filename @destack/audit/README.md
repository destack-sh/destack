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

## Calls

Every executed call is one `Call` from `@destack/sync`: the method, its input and release, and its `Execution`.

| Field | Holds |
|---|---|
| `execution.context` | The actor, subject, delegation, scope, package, service, session or token, and trace. |
| `execution.category` | `activity` for a write or an external effect, `access` for an audited read, `denial` for a refusal. |
| `execution.outcome` | `{ kind: "success", value? }`, or a failure, denial or cancellation with its error. |

## Journal

A `Journal` keeps the calls of one database, runs each request once in one transaction, and replays its outcome to retries.

```ts
import { Journal, journal } from "@destack/audit";

const calls = new Journal(database, callKey);
const results = await calls.execute(request, fingerprint, {
    authorize: (transaction) => authorization.within(transaction).require(permission, target),
    run: (transaction) => updateAccount(transaction, input),
});

export const main = defineDatabase({ name: "main", tables: [journal, ...note.tables] });
```

An `ObjectServer` builds its own journal from its database and call key, so most services never construct one.

## Recorders

An `AuditRecorder` attributes calls to the verified caller and writes them to a journal.

```ts
import { AuditRecorder } from "@destack/audit";

const audit = AuditRecorder.service(calls, { package: notes.package, service: "notes" });
const recorder = audit(spaceId, context);
```

A recorder records a write in its transaction, an external effect as a running call and its outcome, and a read as one access.

```ts
await database.transaction(async (transaction) => {
    await transaction.update(note).set({ title }).where(eq(note.id, id));
    await recorder.record(transaction, renameNote, {
        targets: { note: { type: "note", id } },
        details: { title },
        outcome: { kind: "success" },
    });
});

await recorder.attempt(sendInvitation, { targets, details: {} }, () => invitations.send(id));
const secret = await recorder.read(openSecret, { targets, details: {} }, () => secrets.get(id));
```

## Procedures

`AuditRecorder.procedure` records each procedure call of a server as one call when it ends, by the category the procedure declares, and every denial.

```ts
Server.start({
    ...options,
    audit: AuditRecorder.procedure(({ context }) => recorderOf(context), { isAccessAudited }),
});
```

## Delivery

A journal delivers its audited calls to a history in batches, without their input and result values.

```ts
import { createAuditClient } from "@destack/audit/client";
import { ControlLoop } from "@destack/service/control";

await new ControlLoop(database, [calls.controller(createAuditClient({ url, headers }))], {
    report,
}).run(signal);
```

## History

An `AuditHistory` stores each call once and lists, exports and prunes a scope's calls.

```ts
import { AuditHistory, auditTables } from "@destack/audit/history";
import { implementService } from "@destack/audit/server";

const history = new AuditHistory(historyDatabase);
Server.start({ ...implementService(history, { access, record }), ...hosting });

const page = await history.list({ scope: spaceId, limit: 100 });
await history.prune({ scope: spaceId, before: cutoff, limit: 100 });

export const histories = defineDatabase({ name: "history", tables: auditTables });
```
