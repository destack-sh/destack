# @destack/resource

Declare the resources a package needs, bind their clients, plan their changes and copy their content.

## Declarations

`defineResourceKind` defines a kind by its spec and, for kinds whose providers reconcile one, its desired state.

```ts
import { defineResourceKind } from "@destack/resource";

const DatabaseKind = defineResourceKind("database", { spec: DatabaseSpec, state: DatabaseState });
const BucketKind = defineResourceKind("bucket", { spec: BucketSpec });
const files = BucketKind.description.parse({ name: "files", kind: "bucket", spec: {} });
```

## Clients

A `ResourceContext` holds the client the host binds for each declaration an invocation uses.

```ts
import { ResourceContext } from "@destack/resource/context";

const context = new ResourceContext().bind(database, connection);
await database.get(context).select().from(note);
```

## Providers

A `Provider` connects the resources of one kind on a host, and has the capabilities its kind and technology allow.

| Capability | Methods | Required |
|---|---|---|
| connecting | `connect` | always |
| `Reconciling` | `plan`, `apply` | exactly when the kind declares a desired state |
| `Provisioning` | `provision`, `destroy` | when the provider hosts what it provides |
| `Copying` | `export`, `import` | when the provider moves content in and out |

A host checks a capability before using it.

```ts
const client = await provider.connect(record, notes);
if (Provider.reconciles(provider)) {
    const plan = await provider.plan(record, [notes.state(), tasks.state()]);
    Plan.classify(plan); // "safe", "data-dependent", "backward-incompatible" or "destructive"
    await provider.apply(record, desired, await Plan.digest(plan));
}
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
