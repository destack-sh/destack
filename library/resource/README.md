Declare Destack resources and bind them to resources in a space.

## Usage

```ts
import { Binding, defineResourceSchema } from "@destack/resource";
import { schema } from "@destack/schema";

const Bucket = defineResourceSchema(
    "bucket",
    1,
    schema.object({
        versioning: schema.boolean(),
    }),
);

const files = Bucket.parse({
    name: "files",
    kind: "bucket",
    version: 1,
    spec: { versioning: true },
});

const binding = Binding.parse({
    installation: installation.id,
    package: stack.id,
    name: "files",
    target: { space: space.id, resource: documents.id },
});
```

## Context

The host supplies authorised clients for an invocation.

```ts
import { ResourceContext } from "@destack/resource/context";
import { database } from "@florian/stack";

const context = new ResourceContext().bind(database, connection);
await database.get(context).select().from(note);
```
