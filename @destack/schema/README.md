# @destack/schema

Define, validate and describe the values of Destack packages, based on [Zod](https://zod.dev/).

## Schemas

`schema` is the Zod API, and `defineSchema` and `toJsonSchema` reject rules that JSON Schema cannot express, such as transforms, refinements, dates and loose objects.

```ts
import { defineSchema, schema, toJsonSchema } from "@destack/schema";

const Note = defineSchema(
    schema.object({ title: schema.string().min(1), archived: schema.boolean() }),
);
type Note = schema.Infer<typeof Note>;

Note.parse({ title: "Hello", archived: false });
toJsonSchema(Note);
```

## JSON Schema

`fromJsonSchema` turns a JSON Schema back into a schema that validates the same values, following local references at any pointer.

```ts
import { fromJsonSchema } from "@destack/schema";

fromJsonSchema(toJsonSchema(Note)).parse({ title: "Hello", archived: false });
fromJsonSchema({ $ref: "#/components/schemas/Pet", components: { schemas: { Pet } } }); // an API description's schema
```

## Comparisons

`compareJsonSchemas` compares two JSON Schemas by the values each accepts.

```ts
import { compareJsonSchemas } from "@destack/schema";

compareJsonSchemas(toJsonSchema(schema.enum(["a"])), toJsonSchema(schema.enum(["a", "b"]))); // "wider"
compareJsonSchemas(toJsonSchema(schema.string()), toJsonSchema(schema.string().max(5))); // "narrower"
compareJsonSchemas(toJsonSchema(schema.string()), toJsonSchema(schema.number())); // "incompatible"
```

## Comparison results

`SchemaComparison` states how the values of the new schema relate to the values of the old one.

```ts
type SchemaComparison =
    | "same" // each accepts exactly the other's values
    | "wider" // the new schema accepts every old value
    | "narrower" // the old schema accepts every new value
    | "incompatible"; // neither is true, or the comparison cannot prove it
```

## Text order

`Text.compare` orders strings by UTF-16 code units, the same on every host whatever its locale.

```ts
import { Text } from "@destack/schema";

names.toSorted((left, right) => Text.compare(left, right)); // "B" before "a" before "ä"
```

## Identifiers

`schema.identifier` validates a lowercase prefix and a UUIDv7 and brands the type by the prefix, `Identifier.uuid` returns the UUID, and `Identifier.derive` derives the identifier of a request's object.

```ts
import { Identifier, schema } from "@destack/schema";

const SpaceId = schema.identifier("space");
const id = SpaceId.parse("space-01995f12-3456-7890-8abc-123456789abc");
Identifier.uuid(id); // "01995f12-3456-7890-8abc-123456789abc"
Identifier.derive("message", requestId); // "message-01995f12-3456-7890-8abc-123456789abc" for request 01995f12-3456-8890-8abc-123456789abc
```

## Versions

`Version.compare` orders calendar versions with dev and nightly builds before their release, `Version.next` names the release after one, and `Version.between` lists the entries after one release and up to another.

```ts
import { Version } from "@destack/schema";

Version.compare("2026.10.0-nightly.3", "2026.10.0"); // -1
Version.compare("2026.10.0-dev.9", "2026.10.0-nightly.1"); // -1
Version.next("2026.10.0"); // "2026.10.1"
Version.between({ "2026.9.0": a, "2026.10.0": b }, "2026.8.0", "2026.9.0"); // [["2026.9.0", a]]
```

## Time

The time types take their names from Temporal and keep their JSON forms.

```ts
const instant: Instant = 1_760_000_000_000; // UTC epoch milliseconds
const duration: Duration = { hours: 1, minutes: 30 }; // days, hours, minutes, seconds and milliseconds, each optional
const date: PlainDate = { year: 2026, month: 10, day: 3 };
const time: PlainTime = "09:30"; // HH:MM on a 24-hour clock
const weekday: Weekday = 1; // 1 for Monday to 7 for Sunday
const zone: TimeZone = "Europe/Vienna"; // an IANA name
```

## Time arithmetic

`Duration.milliseconds` and `Duration.seconds` measure a duration, and `TimeZone.next` returns the next of some local times in a time zone.

```ts
import { Duration, TimeZone } from "@destack/schema";

Duration.milliseconds({ minutes: 1, seconds: 30 }); // 90000
Duration.seconds({ milliseconds: 1500 }); // 1.5
TimeZone.next("Europe/Vienna", ["09:00", "17:00"], Date.now()); // the next 09:00 or 17:00 in Vienna
```

## Sensitivity

`schema.sensitive` marks a secret that log entries, journal fingerprints, events and other derived records leave out, or with `"personal"` personal data that events keep sealed under its person's key, and `schema.redact` leaves out both.

```ts
const Call = schema.object({
    address: schema.sensitive(schema.string(), "personal"),
    token: schema.sensitive(schema.string()),
});
schema.sensitivityOf(Call.shape.address); // "personal"
schema.redact(Call, { address: "ada@example.com", token: "secret" }); // {}
schema.mapSensitive(Call, value, (held, sensitivity) =>
    sensitivity === "personal" ? seal(held) : undefined,
);
```

## JSON

`JsonValue.of` converts a value to its JSON form, and `schema.json()` validates a JSON value.

```ts
import { JsonValue, schema } from "@destack/schema";

JsonValue.of(new URL("https://destack.sh")); // "https://destack.sh/"
schema.json().parse({ tags: ["a"] }); // { tags: ["a"] }
```

## Checked reads

`present`, `found`, `aligned` and `zip` throw on a missing value instead of returning `undefined`.

```ts
import { aligned, found, present, zip } from "@destack/schema";

present(user.email, "email"); // the email, or TypeError "email is missing"
found(notes, id); // the map's value, or RangeError
aligned(rows, 2); // the third row, or RangeError
zip(ids, rows); // [[id, row], ...], or RangeError for lists of different lengths
```

## Digests

`canonicalize` writes JSON with sorted object keys, `Digest.json` hashes that form, and `Commit` validates a Git object identifier.

```ts
import { canonicalize, Commit, Digest } from "@destack/schema";

canonicalize({ b: 1, a: [true] }); // '{"a":[true],"b":1}'
await Digest.json({ b: 1, a: [true] }); // the SHA-256 of the canonical form, as hex
Commit.parse(sha); // a SHA-1 or SHA-256 commit in lowercase hex
```
