Declare the resources a package needs and (subsequently) bind them to resources in a space.

## Declarations

```ts
import { defineResourceSchema } from "@destack/resource";

const Bucket = defineResourceSchema("bucket", 1, schema.object({ versioning: schema.boolean() }));
const files = Bucket.parse({ name: "files", kind: "bucket", version: 1, spec: { versioning: true } });
```

## Clients

The host binds a client for each declaration an invocation uses.

```ts
import { ResourceContext } from "@destack/resource/context";

const context = new ResourceContext().bind(database, connection);
await database.get(context).select().from(note);
```

## Plans

A provider creates the resources of one kind and moves each one to the desired states of the declarations bound to it.

```ts
import { classify, digestPlan } from "@destack/resource";

const plan = await provider.plan(record, [notes.state(), tasks.state()]);
classify(plan); // "safe", "data-dependent", "backward-incompatible" or "destructive"
await provider.apply(record, desired, await digestPlan(plan));
```

A desired state no plan can reach throws a `PlanError` naming what the declarations must add, such as a conversion.
