Declare the resources a package needs, bind their clients, and plan their changes.

## Declarations

A resource kind validates its declarations with a schema of its kind, version and spec.

```ts
import { defineResourceSchema } from "@destack/resource";

const BucketDescription = defineResourceSchema("bucket", 1, schema.object({}));
const files = BucketDescription.parse({ name: "files", kind: "bucket", version: 1, spec: {} });
```

## Clients

The host binds one client for each declaration an invocation uses.

```ts
import { ResourceContext } from "@destack/resource/context";

const context = new ResourceContext().bind(database, connection);
await database.get(context).select().from(note);
```

## Plans

A provider plans the steps taking a resource to the desired states of its declarations, and applies the plan a review saw.

```ts
import { Plan } from "@destack/resource";

const plan = await provider.plan(record, [notes.state(), tasks.state()]);
Plan.classify(plan); // "safe", "data-dependent", "backward-incompatible" or "destructive"
await provider.apply(record, desired, await Plan.digest(plan));
```
