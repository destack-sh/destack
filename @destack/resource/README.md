Declare the resources a package needs, bind their clients, plan their changes and open their content.

## Declarations

`defineResourceKind` defines a kind by its spec and, for kinds whose providers reconcile one, its desired state.

```ts
import { defineResourceKind } from "@destack/resource";

const DatabaseKind = defineResourceKind("database", { spec: DatabaseSpec, state: DatabaseState });
const BucketKind = defineResourceKind("bucket", { spec: BucketSpec });
const files = BucketKind.description.parse({ name: "files", kind: "bucket", spec: {} });
```

A kind is itself a declaration: builds describe it as a `resource-kind` with the JSON Schemas of its spec and state.

```ts
import { describeResourceKind } from "@destack/resource/inspect";

const { name, spec, state } = describeResourceKind(DatabaseKind);
```

## Clients

A `ResourceContext` holds the client the host binds for each declaration an invocation uses.

```ts
import { ResourceContext } from "@destack/resource/context";

const context = new ResourceContext().bind(database, connection);
await database.get(context).select().from(note);
```

## Providers

A `Provider` manages one kind's resources on a host, and has the capabilities its kind and technology allow, each a member holding the whole capability: `reconcile`, `provision`, `open` and `rewrap`.

A host parses stored rows through the provider's kind and checks a capability's presence before using it.

```ts
const record = provider.kind.record(row);
if (provider.reconcile !== undefined) {
    const desired = provider.kind.states([notes.state(), tasks.state()]);
    const plan = await provider.reconcile.plan(record, desired);
    Plan.classify(plan); // "safe", "data-dependent", "backward-incompatible" or "destructive"
    await provider.reconcile.apply(record, desired, await Plan.digest(plan));
}
```

## Connectors

A declaration's `Connector` for a provider opens a client inside a workload for the `ResourceBinding` its host sends.

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

A provider opens a resource's content as a database handle, and rewraps its host-bound rows for another host's `Recipient`.

```ts
const handle = await provider.open.open(record, desired); // { database, blobs?, migrate, close }
const recipient = await Recipient.generate();
const wrapped = await source.rewrap.wrap(row, Recipient.of(recipient.key));
const unwrapped = await target.rewrap.unwrap(wrapped, recipient);
```

## Plans

A `Plan` lists `Step`s, each an action on an address, such as `create`, `replace`, `rename`, `convert` or `restore`.

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
Plan.reaches(plan, "backward-incompatible"); // true: the plan needs approval
await Plan.digest(plan); // what an approval holds
```

## Upgrades

A build plans its `Upgrade` from the package's latest release.

```ts
const history = await History.read(latest.reader, vocabulary);
const upgrade = Upgrade.plan(history, declarations, (declaration) =>
    compares.get(declaration.kind),
);
// { from: "2026.9.0", steps: [{ action: "delete", target: "object/note/relation/editor", risk: "backward-incompatible", ... }] }
```

A `Vocabulary` records the release that added and removed each term.

```ts
Vocabulary.plan(vocabulary, declarations); // PlanError: object/note/relation/editor: removed in 2026.10.0 with another definition; choose a new name
const advanced = Vocabulary.advance(vocabulary, declarations, "2026.11.0");
```
