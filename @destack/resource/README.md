# @destack/resource

Declare the resources a package needs, bind their clients, plan their changes and open their content.

## Declarations

`defineResourceKind` defines a resource kind by the schema of its spec and, for kinds whose providers reconcile it, the schema of its desired state.

```ts
import { defineResourceKind } from "@destack/resource";

const DatabaseKind = defineResourceKind("database", { spec: DatabaseSpec, state: DatabaseState });
const BucketKind = defineResourceKind("bucket", { spec: BucketSpec });
const files = BucketKind.description.parse({ name: "files", kind: "bucket", spec: {} });
```

## Kind descriptions

`describeResourceKind` returns a kind's name and the JSON Schemas of its spec and state, which builds record as a `resource-kind` declaration.

```ts
import { describeResourceKind } from "@destack/resource/inspect";

const { name, spec, state } = describeResourceKind(DatabaseKind);
```

## Clients

`ResourceContext.bind` sets the client of a declaration for one invocation, and the declaration's `get` reads it back.

```ts
import { ResourceContext } from "@destack/resource/context";

const context = new ResourceContext().bind(database, connection);
await database.get(context).select().from(note);
```

## Providers

A `Provider` manages one kind's resources on a host, and the host checks each optional `reconcile`, `provision`, `open` or `rewrap` member before it calls it.

```ts
const record = provider.kind.record(row);
if (provider.reconcile !== undefined) {
    const desired = provider.kind.states([notes.state(), tasks.state()]);
    const plan = await provider.reconcile.plan(record, desired);
    Plan.classify(plan); // "safe", "fallible", "backward-incompatible" or "destructive"
    await provider.reconcile.apply(record, desired, await Plan.digest(plan));
}
```

## Connectors

`connectors` maps a provider code to the `Connector` that opens a client inside a workload for the `ResourceBinding` the host sends.

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

`open.open` opens a resource's content as a database handle, and `rewrap.wrap` and `rewrap.unwrap` move its host-bound keys to another host's `Recipient`.

```ts
const handle = await provider.open.open(record, desired); // { database, blobs?, migrate, close }
const recipient = await Recipient.generate();
const wrapped = await source.rewrap.wrap(row, Recipient.of(recipient.key));
const unwrapped = await target.rewrap.unwrap(wrapped, recipient);
```

## Plans

A `Plan` lists `Step`s that each apply an action such as `create`, `replace` or `convert` to an address, and `Plan.classify` returns the highest risk.

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

`Upgrade.plan` lists the steps from the package's latest release to the declarations of a build.

```ts
const history = await History.read(latest.reader, vocabulary);
const upgrade = Upgrade.plan(history, declarations, (declaration) =>
    compares.get(declaration.kind),
);
// { from: "2026.9.0", steps: [{ action: "delete", target: "object/note/relation/editor", risk: "backward-incompatible", ... }] }
```

## Vocabulary

A `Vocabulary` records the release that added and removed each term, and `Vocabulary.plan` refuses a removed term declared again with another definition.

```ts
Vocabulary.plan(vocabulary, declarations); // PlanError: object/note/relation/editor: removed in 2026.10.0 with another definition; choose a new name
const advanced = Vocabulary.advance(vocabulary, declarations, "2026.11.0");
```
