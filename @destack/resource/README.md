# @destack/resource

`defineResourceKind` is a Kubernetes CustomResourceDefinition with `spec` and desired `state` schemas (`management: "external"` for kinds whose resources live outside Destack, such as a third-party API), a `Provider` is a Terraform provider that plans, applies, snapshots and fences one kind's resources on a host, and `ResourceContext` holds a workload's clients as a Cloudflare Worker's `env` holds its bindings.

```ts
// a CRD's spec and status
const DatabaseKind = defineResourceKind("database", { spec: DatabaseSpec, state: DatabaseState });
const plan = await provider.reconcile.plan(record, desired); // terraform plan
// terraform apply of a saved plan
await provider.reconcile.apply(record, desired, await Plan.digest(plan, environment.crypto));
Plan.classify(plan); // "safe", "fallible", "backward-incompatible" or "destructive"
// env.DATABASE in a Worker
await database.get(new ResourceContext().bind(database, connection)).select().from(note);
```

## Declaring

`declare` describes the resource a stack creates from a declaration, and an external kind such as `https` takes the connection whose credential its calls carry.

```ts
export default defineStack([vaultPackage], {
    buckets: {
        files: files.declare({
            retention: { within: { days: 30 } },
            placement: { provider: "r2" },
        }),
    },
});
// an external API, called with the connection's credential
const stripe = stripeApi.declare({ connection: stripeConnection });
```

## Connectors

`connectors` maps a provider code to the `Connector` that opens a declaration's client in a workload.

```ts
const connection = await main.connectors["sqlite"]?.connect(binding, main, environment);
```

## Plans

A `Plan` lists the `Step`s that change resources, and a plan at or above `backward-incompatible` needs approval.

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
Plan.isAtLeast(plan, "backward-incompatible"); // true
```

## Upgrades

`Upgrade.plan` lists the steps from the latest release to a build, and a `Vocabulary` refuses a name reused with another definition.

```ts
const history = await History.read(latest.reader, vocabulary);
const upgrade = Upgrade.plan(history, build.package, declarations, (declaration) =>
    compares.get(declaration.kind),
);
const advanced = Vocabulary.advance(vocabulary, declarations, "2026.11.0");
```
