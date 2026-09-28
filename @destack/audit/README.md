# @destack/audit

Declare audit actions, record them durably, and query authorized history.

## Actions

An action names a Noun.verb, the objects it affects and the details it records, leaving out values its schema marks sensitive.

```ts
import { defineAuditAction } from "@destack/audit";

export const renameNote = defineAuditAction({
    name: "Note.rename",
    version: 1,
    targets: schema.object({ note: schema.object({ type: schema.literal("note"), id: schema.string() }) }),
    details: schema.object({ title: schema.string() }),
});
```

## Recording

A recorder attributes events to the verified caller and writes them to the outbox, in the application's transaction where there is one.

```ts
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";

const audit = AuditRecorder.from(context.caller, new AuditOutbox(database), {
    package: import.meta.destack.package,
    service: "notes",
    scope: spaceId,
    requestId: context.requestId,
});

// record a database change in its transaction
await database.transaction(async (transaction) => {
    await transaction.update(note).set({ title }).where(eq(note.id, id));
    await audit.record(transaction, renameNote, {
        targets: { note: { type: "note", id } },
        details: { title },
        outcome: "success",
    });
});

// record an external effect as an attempt and its result
await audit.attempt(sendInvitation, { targets, details: {} }, () => invitations.send(id));

// name in the result what only the result knows, such as the version a read disclosed
await audit.attempt(readSecret, { targets, details: {} }, () => secrets.read(id), (read) => ({
    version: read.version,
}));

// record every procedure call of a server
Server.start({ ...options, audit: AuditRecorder.procedure(({ context }) => recorder(context)) });
```

## Delivery

An outbox delivers events to the history as they commit, in order, and backs off after failed deliveries.

```ts
import { createAuditClient } from "@destack/audit/client";

await outbox.run(createAuditClient({ url, headers }), { signal, report });
```

## History

A history accepts events once per producer position and serves them to callers with the scope's `read` permission.

```ts
import { AuditHistory } from "@destack/audit/history";
import { implementService } from "@destack/audit/server";

const history = new AuditHistory(historyDatabase);
Server.start({ ...implementService(history, { access, record }), ...hosting });

const page = await history.list({ scope: spaceId, limit: 100 });
for await (const record of await client.export({ scope: spaceId, limit: 100 })) {
    await archive.write(record);
}
await client.prune({ scope: spaceId, before: retentionCutoff, limit: 100 });
```
