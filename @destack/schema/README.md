Define, validate and describe the data of Destack packages, based on [Zod](https://zod.dev/).

## Schemas

`schema` is Zod's API, limited to rules JSON Schema can express: transforms, refinements, dates and loose objects are rejected.

```ts
import { defineSchema, schema, toJsonSchema } from "@destack/schema";

const Note = defineSchema(
    schema.object({ title: schema.string().min(1), archived: schema.boolean() }),
);
type Note = schema.Infer<typeof Note>;

Note.parse({ title: "Hello", archived: false });
toJsonSchema(Note);
```

A described schema validates again after it travels as JSON Schema.

```ts
import { fromJsonSchema } from "@destack/schema";

fromJsonSchema(toJsonSchema(Note)).parse({ title: "Hello", archived: false });
```

Two described schemas compare by the values each accepts.

```ts
import { compareJsonSchemas } from "@destack/schema";

compareJsonSchemas(toJsonSchema(schema.enum(["a"])), toJsonSchema(schema.enum(["a", "b"]))); // "wider"
compareJsonSchemas(toJsonSchema(schema.string()), toJsonSchema(schema.string().max(5))); // "narrower"
compareJsonSchemas(toJsonSchema(schema.string()), toJsonSchema(schema.number())); // "incompatible"
```

| Change         | Meaning                                            |
| -------------- | -------------------------------------------------- |
| `same`         | each accepts exactly the other's values            |
| `wider`        | the new schema accepts every old value             |
| `narrower`     | the old schema accepts every new value             |
| `incompatible` | neither is true, or the comparison cannot prove it |

## Identifiers

An identifier is a lowercase prefix and a UUIDv7, branded by its prefix.

```ts
import { identifier, Identifier } from "@destack/schema";

const SpaceId = identifier("space");
const id = SpaceId.parse("space-01995f12-3456-7890-8abc-123456789abc");
Identifier.uuid(id); // "01995f12-3456-7890-8abc-123456789abc"
```

## Versions

A version is a calendar release, and a nightly build sorts before its release.

```ts
import { Version } from "@destack/schema";

Version.compare("2026.10.0-nightly.3", "2026.10.0"); // -1
Version.between({ "2026.9.0": a, "2026.10.0": b }, "2026.8.0", "2026.9.0"); // [["2026.9.0", a]]
```

## Time

Time values follow Temporal's names and keep their JSON forms.

| Value       | Form                                                   |
| ----------- | ------------------------------------------------------ |
| `Instant`   | UTC epoch milliseconds                                 |
| `Duration`  | `{ days?, hours?, minutes?, seconds?, milliseconds? }` |
| `PlainDate` | `{ year, month, day }`                                 |
| `PlainTime` | `"HH:MM"` on a 24-hour clock                           |
| `Weekday`   | 1 for Monday to 7 for Sunday                           |
| `TimeZone`  | an IANA name, such as `"Europe/Vienna"`                |

```ts
import { Duration, TimeZone } from "@destack/schema";

Duration.milliseconds({ minutes: 1, seconds: 30 }); // 90000
TimeZone.next("Europe/Vienna", ["09:00", "17:00"], Date.now()); // the next 09:00 or 17:00 in Vienna
```

## Sensitivity

A sensitive schema's values stay in their own column: nothing derived from a request or record, such as a log entry or a journal fingerprint, keeps them.

```ts
const Credential = schema.object({
    name: schema.string(),
    token: schema.sensitive(schema.string()),
});
schema.isSensitive(Credential.shape.token); // true
schema.redact(Credential, { name: "ci", token: "secret" }); // { name: "ci" }
```

## Digests

```ts
import { canonicalize, Digest } from "@destack/schema";

canonicalize({ b: 1, a: [true] }); // '{"a":[true],"b":1}'
await Digest.json({ b: 1, a: [true] }); // the SHA-256 of the canonical form, as hex
```
