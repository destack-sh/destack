# @destack/schema

`schema` is Zod 4 limited to what JSON Schema expresses, `toJsonSchema` is Zod's `z.toJSONSchema` and `fromJsonSchema` its exact inverse with patterns matched by RE2, the time types are Temporal's `Instant`, `PlainDate`, `PlainTime` and `Duration` in JSON, and `schema.identifier` brands a prefixed UUIDv7.

```ts
// z.object
const Note = schema.object({ title: schema.string().min(1), archived: schema.boolean() });
type Note = schema.Infer<typeof Note>; // z.infer
toJsonSchema(Note); // z.toJSONSchema, refusing transforms, refinements, dates and loose objects
// the same Note, refusing defaults, open objects and unknown keywords
fromJsonSchema(toJsonSchema(Note));
// Identifier<"space">
schema.identifier("space").parse("space-01995f12-3456-7890-8abc-123456789abc");
const date: PlainDate = { year: 2026, month: 10, day: 3 }; // Temporal.PlainDate in JSON
```

## Comparisons

`compareJsonSchemas` compares two JSON Schemas by the values each accepts: `"same"`, `"wider"`, `"narrower"` or `"incompatible"`.

```ts
// "wider"
compareJsonSchemas(toJsonSchema(schema.enum(["a"])), toJsonSchema(schema.enum(["a", "b"])));
// "narrower"
compareJsonSchemas(toJsonSchema(schema.string()), toJsonSchema(schema.string().max(5)));
```

## Sensitivity

`schema.sensitive` marks a secret that log entries, journals and events leave out, or with `"personal"` personal data that events store sealed under its person's key.

```ts
const Call = schema.object({
    address: schema.sensitive(schema.string(), "personal"),
    token: schema.sensitive(schema.string()),
});
schema.redact(Call, { address: "ada@example.com", token: "secret" }); // {}
```

## Versions

`Version` orders calendar versions with dev builds before their release.

```ts
Version.compare("2026.10.0-dev.3", "2026.10.0"); // -1
Version.next("2026.10.0"); // "2026.10.1"
```

## Checked reads

`present`, `found`, `nth` and `zip` throw on a missing value instead of returning `undefined`.

```ts
present(user.email, "email"); // the email, or TypeError "email is missing"
found(notes, id); // the map's value, or RangeError
nth(changes, -1); // the last change, as Array.prototype.at, or RangeError for an empty list
zip(ids, rows); // [[id, row], ...], or RangeError for lists of different lengths
```

## Mapped records

`mapValues` builds a record from an object's own keys, typed as the mapped type its caller states, and is the one place a record built from runtime keys gets its type.

```ts
const columns = mapValues<Builders, TableColumnMap<Builders, Name>>(
    builders,
    (builder) => new Column(sqlName, builder.definition),
);
```

## Digests

`canonicalize` writes RFC 8785's canonical JSON, keys sorted by UTF-16 code units, and `Digest.json` hashes that form with SHA-256.

```ts
canonicalize({ b: 1, a: [true] }); // '{"a":[true],"b":1}'
await Digest.json({ b: 1, a: [true] });
```
