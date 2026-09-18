Define, validate, and inspect Destack schemas based on [Zod](https://zod.dev/).

## Usage

Validate a value and export its JSON Schema.

```ts
import { defineSchema, schema, toJsonSchema } from "@destack/schema";

const Note = defineSchema(schema.object({
    title: schema.string().min(1),
    archived: schema.boolean(),
}));

type Note = schema.Infer<typeof Note>;

const note: Note = Note.parse({ title: "Hello", archived: false });
const description = toJsonSchema(Note);
```

## Conventions

Destack follows Zod with these conventions:

- Import `schema` instead of `z`.
- `schema.object()` rejects unknown properties.
- `defineSchema()` requires JSON-compatible validation without transforms or custom callbacks.
- `toJsonSchema()` exports JSON Schema Draft 2020-12.
- All exports support `browser`, `worker`, and `host`.

## Identifiers

Define prefixed UUIDv7 identifiers alongside their models.

```ts
import { identifier, schema, toJsonSchema } from "@destack/schema";

const SpaceId = identifier("space");
type SpaceId = schema.Infer<typeof SpaceId>;

const id = SpaceId.parse("space-01995f12-3456-7890-8abc-123456789abc");
const description = toJsonSchema(SpaceId);
```
