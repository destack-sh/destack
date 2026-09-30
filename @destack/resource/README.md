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

A `Provider` manages one kind's resources on a host, and has the capabilities its kind and technology allow.

| Capability | Methods | Required |
|---|---|---|
| `Reconciling` | `plan`, `apply` | exactly when the kind declares a desired state |
| `Provisioning` | `provision`, `destroy` | when the provider hosts what it provides |
| `Copying` | `export`, `import` | when the provider moves content in and out |

A host parses stored rows through the provider's kind and checks a capability before using it.

```ts
const record = provider.kind.record(row);
if (Provider.reconciles(provider)) {
    const desired = provider.kind.states([notes.state(), tasks.state()]);
    const plan = await provider.plan(record, desired);
    Plan.classify(plan); // "safe", "data-dependent", "backward-incompatible" or "destructive"
    await provider.apply(record, desired, await Plan.digest(plan));
}
```

## Connectors

A declaration's `Connector` for a provider opens a client inside a workload for the `ResourceBinding` its host sends.

```ts
const binding = { resource, kind: "database", provider: "sqlite", reference: "file:///spaces/space-…/resource-….db" };
const connection = await notes.connectors.sqlite!.connect(binding, notes);
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

## Plans

A `Plan` lists `Step`s, each an action on an address.

```ts
const plan: Plan = {
    steps: [
        { action: "create", target: "resource/main/table/note/column/priority", risk: "safe", detail: "add column priority" },
        { action: "delete", target: "role/viewer", risk: "destructive", detail: "retire" },
    ],
};
Plan.classify(plan); // "destructive"
Plan.reaches(plan, "backward-incompatible"); // true: the plan needs approval
await Plan.digest(plan); // what an approval holds
```

| Action | Meaning |
|---|---|
| `create`, `update`, `delete` | add, change or remove the target |
| `replace` | recreate the target, such as rebuilding a table |
| `rename` | move the target to a new address |
| `convert` | rewrite stored values, such as rows of an earlier release |
| `restore` | reuse a removed term under its last definition |

## Upgrades

A build plans its `Upgrade` from the package's latest release.

```ts
const history = await History.read(latest.reader, vocabulary);
const build = await builder.build({ outputs, dependencies, history });
const upgrade = await build.reader.read(build.manifest.upgrade!.file, Upgrade);
// { from: "2026.9.0", steps: [{ action: "delete", target: "object/note/relation/editor", risk: "backward-incompatible", ... }] }
```

A `Vocabulary` records the release that added and removed each term.

```ts
Vocabulary.plan(vocabulary, declarations); // PlanError: object/note/relation/editor: removed in 2026.10.0 with another definition; choose a new name
const advanced = Vocabulary.advance(vocabulary, declarations, "2026.11.0");
```
