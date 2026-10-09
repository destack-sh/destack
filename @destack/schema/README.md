# @destack/schema

`schema` is Zod 4 limited to what JSON Schema expresses, `toJsonSchema` and `fromJsonSchema` are Zod's `z.toJSONSchema` and `z.fromJSONSchema`, the time types are Temporal's `Instant`, `PlainDate`, `PlainTime` and `Duration` in JSON, and `schema.identifier` brands a prefixed UUIDv7.

```ts
const Note = defineSchema(schema.object({ title: schema.string().min(1), archived: schema.boolean() })); // z.object
type Note = schema.Infer<typeof Note>; // z.infer
toJsonSchema(Note); // z.toJSONSchema, refusing transforms, refinements, dates and loose objects
fromJsonSchema({ $ref: "#/components/schemas/Pet", components: { schemas: { Pet } } }); // an OpenAPI schema
schema.identifier("space").parse("space-01995f12-3456-7890-8abc-123456789abc"); // Identifier<"space">
const date: PlainDate = { year: 2026, month: 10, day: 3 }; // Temporal.PlainDate in JSON
```

## Comparisons

`compareJsonSchemas` compares two JSON Schemas by the values each accepts: `"same"`, `"wider"`, `"narrower"` or `"incompatible"`.

```ts
compareJsonSchemas(toJsonSchema(schema.enum(["a"])), toJsonSchema(schema.enum(["a", "b"]))); // "wider"
compareJsonSchemas(toJsonSchema(schema.string()), toJsonSchema(schema.string().max(5))); // "narrower"
```

## Sensitivity

`schema.sensitive` marks a secret that log entries, journals and events leave out, or with `"personal"` personal data that events keep sealed under its person's key.

```ts
const Call = schema.object({
    address: schema.sensitive(schema.string(), "personal"),
    token: schema.sensitive(schema.string()),
});
schema.redact(Call, { address: "ada@example.com", token: "secret" }); // {}
```

## Versions

`Version` orders calendar versions with dev and nightly builds before their release.

```ts
Version.compare("2026.10.0-nightly.3", "2026.10.0"); // -1
Version.next("2026.10.0"); // "2026.10.1"
```

## Checked reads

`present`, `found`, `aligned` and `zip` throw on a missing value instead of returning `undefined`.

```ts
present(user.email, "email"); // the email, or TypeError "email is missing"
found(notes, id); // the map's value, or RangeError
zip(ids, rows); // [[id, row], ...], or RangeError for lists of different lengths
```

## Digests

`canonicalize` writes RFC 8785's canonical JSON, keys sorted by UTF-16 code units, and `Digest.json` hashes that form with SHA-256.

```ts
canonicalize({ b: 1, a: [true] }); // '{"a":[true],"b":1}'
await Digest.json({ b: 1, a: [true] });
```
