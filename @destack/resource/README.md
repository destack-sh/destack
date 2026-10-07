# @destack/resource

Declare, bind, plan and open the resources a package needs.

## Declarations

`defineResourceKind` defines a resource kind by the schemas of its spec and desired state.

```ts
import { defineResourceKind } from "@destack/resource";

const DatabaseKind = defineResourceKind("database", { spec: DatabaseSpec, state: DatabaseState });
const BucketKind = defineResourceKind("bucket", { spec: BucketSpec });
const files = BucketKind.description.parse({ name: "files", kind: "bucket", spec: {} });
```

## Definitions

`ResourceDefinition` is the declarable schema of a resource of any kind: its package's declaration, retention, placement and tags, an existing resource to adopt, a reference to connect, or a connection of its space lending its credential.

```ts
import { ResourceDefinition } from "@destack/resource";

const declared = ResourceDefinition.parse({
    declaration: { ...files, package: import.meta.destack.package },
    retention: { within: { days: 30 } },
    placement: { provider: "r2" },
    tags: {},
});
const lent = ResourceDefinition.parse({ ...declared, connection: connectionId }); // provider CONNECTION_PROVIDER
const named = ResourceDefinition.parse({
    ...declared,
    placement: { provider: CONNECTION_PROVIDER },
}); // each call names its connection
```

## Kind descriptions

`describeResourceKind` returns the JSON Schemas of a kind's spec and state.

```ts
import { describeResourceKind } from "@destack/resource/inspect";

const { name, spec, state } = describeResourceKind(DatabaseKind);
```

## Clients

`ResourceContext.bind` sets the client of a declaration for one invocation.

```ts
import { ResourceContext } from "@destack/resource/context";

const context = new ResourceContext().bind(database, connection);
await database.get(context).select().from(note);
```

## Providers

A `Provider` manages one kind's resources on a host.

```ts
const record = provider.kind.record(row);
if (provider.reconcile !== undefined) {
    const desired = provider.kind.states([notes.state(), tasks.state()]);
    const plan = await provider.reconcile.plan(record, desired);
    Plan.classify(plan); // "safe", "fallible", "backward-incompatible" or "destructive"
    await provider.reconcile.apply(record, desired, await Plan.digest(plan));
}
```

## Snapshots

`snapshot` copies a resource's content into a content-addressed store.

```ts
const recipient = await Recipient.generate(); // the restoring host's
const digest = await provider.snapshot.snapshot(record, desired, store, recipient);
await target.snapshot.restore(provisioned, desired, digest, store, recipient);
```

## Connectors

`connectors` maps a provider code to the `Connector` that opens its client in a workload.

```ts
const binding = {
    resource,
    kind: "database",
    provider: "sqlite",
    reference: "file:///spaces/space-…/database-….db",
};
const connector = notes.connectors[binding.provider];
if (connector === undefined) {
    throw new TypeError(`no connector for provider ${binding.provider}`);
}
const connection = await connector.connect(binding, notes);
```

## Moves

`fence` refuses writes to a resource while a move captures it.

```ts
await provider.fence?.fence(record);
const handle = await provider.open.open(record, desired); // { database, migrate, close }
const recipient = await Recipient.generate(); // from @destack/identity, on the target
const sealed = await source.rewrap.seal(row, Recipient.of(recipient.key));
const opened = await target.rewrap.open(sealed, recipient);
```

## Plans

A `Plan` lists the `Step`s that change resources.

```ts
const plan: Plan = {
    steps: [
        {
            action: "create",
            target: "resource/main/table/note/column/priority",
            risk: "safe",
            detail: "add column priority",
        },
        { action: "delete", target: "role/viewer", risk: "destructive", detail: "retire" },
    ],
};
Plan.classify(plan); // "destructive"
Plan.isAtLeast(plan, "backward-incompatible"); // true: the plan needs approval
await Plan.digest(plan); // what an approval holds
```

## Upgrades

`Upgrade.plan` lists the steps from the latest release to a build.

```ts
const history = await History.read(latest.reader, vocabulary);
const upgrade = Upgrade.plan(history, build.package, declarations, (declaration) =>
    compares.get(declaration.kind),
);
// { from: "2026.9.0", steps: [{ action: "delete", target: "object/note/relation/editor", risk: "backward-incompatible", ... }] }
```

## Vocabulary

A `Vocabulary` records the release that added and removed each term.

```ts
Vocabulary.plan(vocabulary, declarations); // PlanError: object/note/relation/editor: removed in 2026.10.0 with another definition; choose a new name
const advanced = Vocabulary.advance(vocabulary, declarations, "2026.11.0");
```

## Errors

`PlanError` reports every refusal of a plan at once, which callers receive as a conflict.

```ts
import { PlanError } from "@destack/resource/error";

new PlanError([{ target: "note", detail: "declare a conversion to version 2" }]).toServiceError(); // { code: "CONFLICT", … }
```
