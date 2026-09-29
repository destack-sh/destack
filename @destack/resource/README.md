# @destack/resource

Declare the resources a package needs, bind their clients, plan their changes and copy their content.

## Declarations

`defineResourceSchema` validates the declarations of one resource kind by their kind, version and spec.

```ts
import { defineResourceSchema } from "@destack/resource";

const BucketDescription = defineResourceSchema("bucket", 1, schema.object({}));
const files = BucketDescription.parse({ name: "files", kind: "bucket", version: 1, spec: {} });
```

## Clients

A `ResourceContext` holds the client the host binds for each declaration an invocation uses.

```ts
import { ResourceContext } from "@destack/resource/context";

const context = new ResourceContext().bind(database, connection);
await database.get(context).select().from(note);
```

## Providers

A `Provider` provisions, plans, applies, connects and destroys the resources of one kind on a host.

```ts
const plan = await provider.plan(record, [notes.state(), tasks.state()]);
Plan.classify(plan); // "safe", "data-dependent", "backward-incompatible" or "destructive"
await provider.apply(record, desired, await Plan.digest(plan));
const client = await provider.connect(record, notes);
```

## Copies

A provider exports a resource's content as chunks and imports them into another host's resource, with secrets sealed to a fresh `Recipient`.

```ts
const recipient = await Recipient.generate();
const source = { record, desired, recipient: Recipient.of(recipient.key), stage: "live" } as const;
for await (const chunk of provider.export(source, after, signal)) {
    await target.import({ record: targetRecord, desired, recipient, stage: "live" }, chunk);
}
```
