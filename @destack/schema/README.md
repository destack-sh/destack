Define, validate and describe the data of Destack packages, based on [Zod](https://zod.dev/).

## Schemas

```ts
import { defineSchema, schema, toJsonSchema } from "@destack/schema";

const Note = defineSchema(schema.object({ title: schema.string().min(1), archived: schema.boolean() }));
type Note = schema.Infer<typeof Note>;

Note.parse({ title: "Hello", archived: false });
toJsonSchema(Note);
```

`schema` is Zod's API.
A declared schema describes JSON values with rules JSON Schema can express, so transforms, refinements, dates and loose objects are rejected.

A described schema validates again after it travels as JSON Schema.

```ts
import { fromJsonSchema } from "@destack/schema";

fromJsonSchema(toJsonSchema(Note)).parse({ title: "Hello", archived: false });
```

## Identifiers

```ts
import { identifier, identifierTime } from "@destack/schema";

const SpaceId = identifier("space");
const id = SpaceId.parse("space-01995f12-3456-7890-8abc-123456789abc");
identifierTime(id); // the Unix milliseconds of the UUIDv7
```

## Sensitivity

A sensitive schema's values stay in their own column: nothing derived from a request or record, such as a log entry or a journal fingerprint, keeps them.

```ts
const Credential = schema.object({ name: schema.string(), token: schema.sensitive(schema.string()) });
schema.isSensitive(Credential.shape.token); // true
```

## Digests

```ts
import { canonicalize, digest } from "@destack/schema/json";

canonicalize({ b: 1, a: [true] }); // '{"a":[true],"b":1}'
await digest({ b: 1, a: [true] }); // the SHA-256 of the canonical form, as hex
```

Canonical JSON sorts object keys, omits undefined fields and rejects values JSON cannot hold.
