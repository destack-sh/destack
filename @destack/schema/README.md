# @destack/schema

Define Destack's portable values: validation schemas, identifiers, versions, time values, digests and canonical JSON.

## Overview

`@destack/schema` is Zod 4 with the limits a value needs to travel: a schema that is sent or stored converts to JSON Schema and back without loss.

You describe values with schemas:

- `schema`: Zod 4's builders.
- `toJsonSchema` and `fromJsonSchema`: convert a schema to JSON Schema and back.

Packages share these values:

- `Identifier`: a prefixed UUIDv7 such as `note-0199…`, typed by its prefix.
- `Version`: a calendar version such as `2026.11.0`, with dev builds before their release.
- `Instant`, `Duration`, `TimeZone`, `PlainDate` and `PlainTime`: time values after Temporal, as JSON.
- `Digest`: a SHA-256 of bytes, or of canonical JSON.

Declare a schema, create an identifier, and send the schema as JSON Schema:

```ts
import { fromJsonSchema, Identifier, schema, toJsonSchema } from "@destack/schema";
import { BunEnvironment } from "@destack/environment/bun";

// declare a note
const Note = schema.object({
    id: schema.identifier("note"),
    title: schema.string().min(1).max(200),
    isArchived: schema.boolean(),
});
type Note = schema.Infer<typeof Note>;

// create a note with a new identifier
const id = Identifier.create("note", BunEnvironment.system);
const note: Note = Note.parse({ id, title: "Plans", isArchived: false });

// send the schema as JSON Schema, and read it back
const same = fromJsonSchema(toJsonSchema(Note));
```

## Reference

### schema

`schema` has Zod 4's builders, such as `object`, `string`, `enum` and `union`, and the extras below.
A schema that is sent or stored allows only rules JSON Schema has, with patterns on RE2.

| Extra                                   | What it does                                                |
| --------------------------------------- | ----------------------------------------------------------- |
| `identifier(prefix)`, `anyIdentifier()` | accepts an identifier of a prefix, or of any prefix         |
| `json()`, `jsonObject()`                | accepts any JSON value, or any JSON object                  |
| `sensitive(schema, sensitivity)`        | marks a `"secret"`, the default, or `"personal"` data       |
| `redact(schema, value)`                 | drops a value's sensitive fields, for logs                  |
| `mask(schema, value)`                   | shows a value's personal fields as `****`                   |
| `standard()`                            | accepts a Standard Schema, such as one from another library |

### JSON Schema

`toJsonSchema` turns a schema into JSON Schema, and `fromJsonSchema` turns JSON Schema back into a schema that validates the same values.
`compareJsonSchemas(before, after)` compares the values two schemas accept.

| Answer           | Meaning                                             |
| ---------------- | --------------------------------------------------- |
| `"same"`         | both accept the same values                         |
| `"wider"`        | `after` accepts every value `before` does, and more |
| `"narrower"`     | `before` accepts every value `after` does, and more |
| `"incompatible"` | each accepts values the other refuses               |

### Identifier

An identifier is `<prefix>-<uuid>`, typed by its prefix.

| Verb                                               | What it does                                                |
| -------------------------------------------------- | ----------------------------------------------------------- |
| `Identifier.create(prefix, environment)`           | creates a UUIDv7 identifier at the environment's time       |
| `Identifier.hash(prefix, namespace, name, crypto)` | hashes a name into a UUIDv8 identifier, the same everywhere |
| `Identifier.derive(prefix, requestId)`             | derives the identifier of an object a request creates       |
| `Identifier.filter(prefix, values)`                | keeps the values that are identifiers of a prefix           |
| `Identifier.prefix(id)`, `Identifier.uuid(id)`     | reads its prefix or its UUID                                |

### Version

A version is `<year>.<month>.<release>`, such as `2026.11.0`.
A dev build is `<release>-dev.<UTC time>.<12 hex of its build key>`, as in Go's pseudo-versions, and comes before its release.

| Verb                                  | What it does                                             |
| ------------------------------------- | -------------------------------------------------------- |
| `Version.compare(left, right)`        | orders versions, dev builds by time and then by key      |
| `Version.dev(release, at, key)`       | writes a dev build's version                             |
| `Version.release(version)`            | reads the release a version leads to                     |
| `Version.next(version)`               | reads the next release in the same month                 |
| `Version.isPrerelease(version)`       | reports whether a version is a dev build                 |
| `Version.between(keyed, after, upto)` | lists entries keyed by the releases between two versions |

### Time values

Time values follow Temporal and are plain JSON.

| Value       | Shape                                                            | Verbs                                                   |
| ----------- | ---------------------------------------------------------------- | ------------------------------------------------------- |
| `Instant`   | UTC epoch milliseconds                                           |                                                         |
| `Duration`  | `{ days, hours, minutes, seconds, milliseconds }`, each optional | `milliseconds`, `seconds`, `require`                    |
| `TimeZone`  | an IANA name, such as `Europe/Vienna`                            | `date`, `time`, `instant`, `next`, `isKnown`, `require` |
| `PlainDate` | `{ year, month, day }`                                           | `add`, `compare`, `equals`, `weekday`                   |
| `PlainTime` | `HH:MM` on a 24-hour clock                                       |                                                         |
| `Weekday`   | Monday `1` through Sunday `7`, as in ISO 8601                    |                                                         |

### Digest

A digest is a SHA-256 as lowercase hexadecimal, the address of a build, a file or a lock.

| Verb                         | What it does                              |
| ---------------------------- | ----------------------------------------- |
| `Digest.of(bytes, crypto)`   | hashes bytes, a stream of chunks, or text |
| `Digest.json(value, crypto)` | hashes a value's canonical JSON           |

### JSON

`canonicalize` writes RFC 8785 canonical JSON, so equal values have equal text and equal digests.
`JsonValue` reads and checks JSON values.

| Verb                                             | What it does                                |
| ------------------------------------------------ | ------------------------------------------- |
| `canonicalize(value)`                            | writes a value's canonical JSON             |
| `JsonValue.parse(text)`                          | parses JSON text                            |
| `JsonValue.of(value)`, `JsonValue.object(value)` | checks a value is JSON, or a JSON object    |
| `JsonValue.isObject`, `JsonValue.isArray`        | tells objects and arrays apart              |
| `JsonValue.entries`, `JsonValue.values`          | lists an object's present entries or values |

## Examples

### Mark sensitive fields

A sensitive field stays out of logs, and events encrypt a personal one under its person's key.

```ts
import { schema } from "@destack/schema";

// declare a call with a personal address and a secret token
const Call = schema.object({
    address: schema.sensitive(schema.string(), "personal"),
    token: schema.sensitive(schema.string()),
});

/** Write a call to the log without its sensitive fields. */
export function logCall(call: schema.Infer<typeof Call>) {
    console.log(schema.redact(Call, call));
}
```

### Check a schema change

`compareJsonSchemas` tells whether stored values still parse after a field's schema changes.

```ts
import { compareJsonSchemas, type JsonSchema } from "@destack/schema";

/** Report whether every value the old schema accepted still parses with the new one. */
export function isCompatible(before: JsonSchema, after: JsonSchema): boolean {
    const comparison = compareJsonSchemas(before, after);

    return comparison === "same" || comparison === "wider";
}
```

### Derive a stable identifier

`Identifier.hash` gives the same identifier for a name everywhere, such as a predefined row's.

```ts
import { Identifier, type Uuid } from "@destack/schema";
import { BunEnvironment } from "@destack/environment/bun";

/** The namespace of this package's predefined roles. */
const ROLES: Uuid = "6f9d1c2e-8a4b-4c3d-9e5f-1a2b3c4d5e6f";

/** Derive the owner role's identifier, the same in every database. */
export async function ownerRoleId() {
    return await Identifier.hash("role", ROLES, "owner", BunEnvironment.system.crypto);
}
```

### Schedule in a time zone

`TimeZone.next` finds the next of some times of day in a zone, across summer time changes.

```ts
import { TimeZone } from "@destack/schema";

/** Find when the next digest goes out in Vienna, at nine or at five. */
export function nextDigest(now: number): number {
    return TimeZone.next("Europe/Vienna", ["09:00", "17:00"], now);
}
```

### Stamp a dev version

`Version.dev` writes a dev build's version from its release, time and build key, ordered before the release.

```ts
import { Version } from "@destack/schema";

// stamp a dev build, and order it before its release
export const dev = Version.dev("2026.11.0", Date.UTC(2026, 10, 12, 10, 30), "0123456789abcdef");
export const order = Version.compare(dev, "2026.11.0");
```

## Comparison

### Zod

Schemas are Zod 4's, with the limits a schema needs to travel as JSON Schema.

| Zod                         | `@destack/schema`                   | Difference                                                       |
| --------------------------- | ----------------------------------- | ---------------------------------------------------------------- |
| `z.object`, `z.string`, …   | `schema.object`, `schema.string`, … | The same.                                                        |
| `z.toJSONSchema`            | `toJsonSchema`                      | Refuses custom refinements, which Zod leaves out of JSON Schema. |
| none                        | `fromJsonSchema`                    | Turns JSON Schema back into a schema.                            |
| `z.string().regex(pattern)` | the same                            | Runs on RE2.                                                     |
| `.meta`, registries         | `sensitive`, `redact`, `mask`       | Marks secrets and personal data.                                 |

### Temporal

Time values follow Temporal's names, as plain JSON instead of objects.

| Temporal                 | `@destack/schema` | Difference                                                  |
| ------------------------ | ----------------- | ----------------------------------------------------------- |
| `Temporal.Instant`       | `Instant`         | A number of epoch milliseconds.                             |
| `Temporal.Duration`      | `Duration`        | An object of optional units, without weeks or larger units. |
| `Temporal.PlainDate`     | `PlainDate`       | `{ year, month, day }` in the ISO calendar.                 |
| `Temporal.PlainTime`     | `PlainTime`       | `HH:MM`, without seconds.                                   |
| `Temporal.ZonedDateTime` | `TimeZone` verbs  | Converts instants to and from a zone's dates and times.     |
