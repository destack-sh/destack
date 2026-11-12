# @destack/schema

Define Destack's portable values: validation schemas, identifiers, versions, time values, digests and canonical JSON.

## Overview

Every value Destack stores or sends has a schema that converts to JSON Schema and back, so packages, stacks and clients agree on it.

- **schema**: Zod 4 limited to what JSON Schema expresses; `toJsonSchema` and `fromJsonSchema` convert both ways, and `compareJsonSchemas` tells a wider schema from a narrower one.
- **Identifier**: a prefixed UUIDv7 such as `space-01995f12-…`, branded by its prefix; `create` from an environment's clock and randomness, `hash` from a namespace and a name.
- **Version**: a calendar version such as `2026.11.0`, dev builds ordered before their release.
- **Digest**: a lowercase hexadecimal SHA-256, of bytes or of RFC 8785 canonical JSON.
- **Time values**: Temporal's `Instant`, `PlainDate`, `PlainTime` and `Duration` as JSON.

```ts
import { fromJsonSchema, Identifier, schema, toJsonSchema, Version } from "@destack/schema";
import { BunEnvironment } from "@destack/environment/bun";

const Note = schema.object({
    id: schema.identifier("note"),
    title: schema.string().min(1).max(200),
    isArchived: schema.boolean(),
});
type Note = schema.Infer<typeof Note>;

const id = Identifier.create("note", BunEnvironment.system);
const note: Note = Note.parse({ id, title: "Plans", isArchived: false });
const same = fromJsonSchema(toJsonSchema(Note));
Version.compare("2026.11.0-dev.3", "2026.11.0"); // -1
```

## Examples

### Mark sensitive fields

`schema.sensitive` marks a secret that logs and events leave out, or personal data that events store sealed under its person's key.

```ts
import { schema } from "@destack/schema";

const Call = schema.object({
    address: schema.sensitive(schema.string(), "personal"),
    token: schema.sensitive(schema.string()),
});
schema.redact(Call, { address: "ada@example.com", token: "secret" }); // {}
```

### Compare schemas

`compareJsonSchemas` answers `"same"`, `"wider"`, `"narrower"` or `"incompatible"` by the values each schema accepts.

```ts
import { compareJsonSchemas, schema, toJsonSchema } from "@destack/schema";

const before = toJsonSchema(schema.enum(["a"]));
const after = toJsonSchema(schema.enum(["a", "b"]));
compareJsonSchemas(before, after); // "wider"
```

### Hash canonical JSON

`Digest.json` hashes a value's RFC 8785 canonical JSON, so equal values hash the same everywhere.

```ts
import { canonicalize, Digest } from "@destack/schema";
import { BunEnvironment } from "@destack/environment/bun";

canonicalize({ b: 1, a: [true] }); // '{"a":[true],"b":1}'
const digest = await Digest.json({ b: 1, a: [true] }, BunEnvironment.system.crypto);
```

### Read a value that must be there

`present`, `found` and `nth` throw on a missing value instead of returning `undefined`.

```ts
import { found, nth, present } from "@destack/schema";

const email = present("ada@example.com" as string | undefined, "email");
const title = found(new Map([["note-1", "Plans"]]), "note-1");
const last = nth(["a", "b"], -1); // "b"
```

## Comparison

### Zod

Schemas follow Zod 4, which they wrap; three things differ.

- A schema expresses only what JSON Schema does, so it converts to JSON Schema and back without loss.
- Patterns are matched by RE2, so a schema from another package cannot backtrack for long.
- Identifiers, versions, time values and digests are schemas too, shared by every package.
