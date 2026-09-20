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
- `defineSchema()` checks supported declarations for JSON-compatible values.
- The declared validator determines accepted values.
- `toJsonSchema()` exports JSON Schema Draft 2020-12 for inspection and tooling.
- Exported descriptions can omit format checks, normalization behavior, and regex flags.
- Independent validators require a separate compatibility check before use.
- All exports support browser and server targets.

## Schemas

Use the exported constructors with Zod's standard composition methods.

```ts
schema.string(); schema.boolean(); schema.number(); schema.int();
schema.int32(); schema.uint32(); schema.float32(); schema.float64();
schema.null(); schema.never(); schema.json();

schema.array(schema.string());
schema.object({ name: schema.string() });
schema.tuple([schema.string(), schema.int()]);
schema.record(schema.string(), schema.json());
schema.partialRecord(schema.enum(["read", "write"]), schema.boolean());
schema.keyof(schema.object({ name: schema.string() }));
schema.literal("active"); schema.enum(["active", "paused"]);
schema.union([schema.string(), schema.number()]);
schema.xor([schema.string(), schema.number()]);
schema.intersection(schema.string().min(1), schema.string().max(100));
schema.discriminatedUnion("kind", [
    schema.object({ kind: schema.literal("file"), path: schema.string() }),
    schema.object({ kind: schema.literal("directory"), path: schema.string() }),
]);
schema.lazy(() => schema.string());
schema.templateLiteral(["item-", schema.uuidv7()]);
schema.object({
    optional: schema.optional(schema.string()),
    nullable: schema.nullable(schema.string()),
    nullish: schema.nullish(schema.string()),
});

schema.email(); schema.hostname(); schema.e164();
schema.url(); schema.httpUrl(); schema.emoji();
schema.ipv4(); schema.ipv6(); schema.cidrv4(); schema.cidrv6(); schema.mac();
schema.base64(); schema.base64url(); schema.hex(); schema.hash("sha256");
schema.currencyCode();
schema.jwt(); schema.creditCard(); schema.iban();
schema.guid(); schema.uuid(); schema.uuidv4(); schema.uuidv6(); schema.uuidv7();
schema.nanoid(); schema.cuid2(); schema.ulid(); schema.xid(); schema.ksuid();
schema.iso.date(); schema.iso.time(); schema.iso.datetime(); schema.iso.duration();
```

Formats follow Zod's validation behavior.
Template literals follow Zod's compiled-pattern behavior.

## Exclusions

Declared schemas use JSON values and built-in checks.

- `cuid`, `nativeEnum`: deprecated APIs.
- `any`, `unknown`, loose objects and records: use explicit JSON schemas.
- `bigint`, `Date`, `Map`, `Set`, symbols, functions, promises: use JSON representations.
- Coercion, transforms, codecs, defaults, fallbacks, freezing, custom callbacks: executable behavior.

Global and sticky regular expressions and explicit URL normalization are rejected.
Optional values are supported in object properties.

## Identifiers

Define prefixed UUIDv7 identifiers alongside their models.

```ts
import { identifier, schema, toJsonSchema } from "@destack/schema";

const SpaceId = identifier("space");
type SpaceId = schema.Infer<typeof SpaceId>;

const id = SpaceId.parse("space-01995f12-3456-7890-8abc-123456789abc");
const description = toJsonSchema(SpaceId);
```
