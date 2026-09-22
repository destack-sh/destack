# @destack/audit

Declare audit actions, record them durably, and query authorized history.

## Usage

```ts
import { defineAuditAction } from "@destack/audit";
import { schema } from "@destack/schema";

export const renameNote = defineAuditAction({
    package: import.meta.destack.package,
    name: "note.rename",
    version: 1,
    targets: schema.object({
        note: schema.object({ type: schema.literal("note"), id: schema.string() }),
    }),
    details: schema.object({ title: schema.string() }),
});
```

```ts
await database.transaction(async (transaction) => {
    await transaction.update(note).set({ title }).where(eq(note.id, id));
    await context.audit.record(transaction, renameNote, {
        targets: { note: { type: "note", id } },
        details: { title },
        outcome: "success",
    });
});
```

```ts
const attempt = context.audit.begin(sendInvitation, {
    targets: { invitation: { type: "invitation", id } },
    details: {},
});
await context.audit.append(attempt);

try {
    await invitations.send(id);
} catch (error) {
    const result = context.audit.complete(attempt, { outcome: "failure", errorCode: "SEND_FAILED" });
    await context.audit.append(result);
    throw error;
}

const result = context.audit.complete(attempt, { outcome: "success" });
await context.audit.append(result);
```

## Host

```ts
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox, auditOutboxSchema } from "@destack/audit/outbox";
import { AuditHistory, auditSchema } from "@destack/audit/history";
import { implementService, createProcedureAudit } from "@destack/audit/server";
import { Server } from "@destack/service/server";
import { createAuditClient } from "@destack/audit/client";

// migrate auditOutboxSchema alongside each application's schema
const outbox = new AuditOutbox(database);
const audit = new AuditRecorder(verifiedContext, outbox);

// migrate auditSchema in the local or regional history database
const history = new AuditHistory(historyDatabase);
const server = await Server.start({
    ...implementService(history, { authorize: authorizeAudit, record: createAuditRecorder }),
    audience: receivingPackageId,
    spaceId,
    resources,
    health,
    authenticate,
    authorizeHost: authorizeInstallation,
    drainTimeout: 10000,
});
const response = await server.fetch(request);

const client = createAuditClient({ url, headers: authenticatedHeaders });
await outbox.run(client, { signal, report: reportDeliveryFailure });

const procedureAudit = createProcedureAudit((call) => call.context.audit);
```

## History

```ts
const query = { scope: { type: "space", accountId, spaceId }, limit: 100 };
const page = await client.list(query);
const next = page.cursor && (await client.list({ ...query, cursor: page.cursor }));

for await (const record of await client.export(query)) {
    await archive.write(record);
}

await client.prune({ scope: query.scope, before: retentionCutoff, limit: 100 });
```
