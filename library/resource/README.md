Declare Destack resources and bind them to resources in a space.

## Usage

```ts
import { Binding, defineResource } from "@destack/resource";
import { schema } from "@destack/schema";

const Bucket = defineResource(
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
    installation: "notes",
    name: "files",
    target: { space: "personal", resource: "documents" },
});
```
