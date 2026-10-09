# @destack/resource

`defineResourceKind` is a Kubernetes CustomResourceDefinition with `spec` and desired `state` schemas, a `Provider` is a Terraform provider that plans, applies, snapshots and fences one kind's resources on a host, and `ResourceContext` holds a workload's clients as a Cloudflare Worker's `env` holds its bindings.

```ts
const DatabaseKind = defineResourceKind("database", { spec: DatabaseSpec, state: DatabaseState }); // a CRD's spec and status
const plan = await provider.reconcile.plan(record, desired); // terraform plan
await provider.reconcile.apply(record, desired, await Plan.digest(plan)); // terraform apply of a saved plan
Plan.classify(plan); // "safe", "fallible", "backward-incompatible" or "destructive"
await database.get(new ResourceContext().bind(database, connection)).select().from(note); // env.DATABASE in a Worker
```

## Definitions

`ResourceDefinition` is the declarable schema of a resource of any kind: its package's declaration, retention, placement and tags.

```ts
const files = BucketKind.description.parse({ name: "files", kind: "bucket", spec: {} });
const declared = ResourceDefinition.parse({
    declaration: { ...files, package: import.meta.destack.package },
    retention: { within: { days: 30 } },
    placement: { provider: "r2" },
    tags: {},
});
```

## Connectors

`connectors` maps a provider code to the `Connector` that opens a declaration's client in a workload.

```ts
const connection = await main.connectors["sqlite"]?.connect(binding, main);
```

## Plans

A `Plan` lists the `Step`s that change resources, and a plan at or above `backward-incompatible` needs approval.

```ts
const plan: Plan = {
    steps: [
        { action: "create", target: "resource/main/table/note/column/priority", risk: "safe", detail: "add column priority" },
        { action: "delete", target: "role/viewer", risk: "destructive", detail: "retire" },
    ],
};
Plan.isAtLeast(plan, "backward-incompatible"); // true
```

## Upgrades

`Upgrade.plan` lists the steps from the latest release to a build, and a `Vocabulary` refuses a name reused with another definition.

```ts
const history = await History.read(latest.reader, vocabulary);
const upgrade = Upgrade.plan(history, build.package, declarations, (declaration) => compares.get(declaration.kind));
const advanced = Vocabulary.advance(vocabulary, declarations, "2026.11.0");
```
