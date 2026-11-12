# @destack/error

Declare the errors a package's callers receive, each with a stable reason, an HTTP code and typed data.

## Overview

`@destack/error` is gRPC's error model over HTTP: declared reason families, and one `Failure` shape on the wire.

You declare and throw errors with a family:

- `ErrorFamily`: a package's reasons with the schema of their data, declared with `defineErrorFamily`.
- `Reason`: what failed and how, such as `NOTE_NOT_FOUND`.

Callers receive and check failures:

- `Failure`: an error as a call sends it, with its code, reason, domain, message and data.

Declare a family, throw an error by reason, and check it after a call:

```ts
import { defineErrorFamily, Failure } from "@destack/error";
import { schema } from "@destack/schema";

// declare the package's reasons and their data
export const NoteError = defineErrorFamily({
    reasons: {
        NOTE_NOT_FOUND: {},
        NOTE_BLOCKED: { data: schema.object({ holder: schema.string() }) },
    },
});

// create an error of a reason with its data
const error = NoteError.of("NOTE_BLOCKED", "cannot edit note 'Plans': 'ada' is editing it", {
    data: { holder: "ada" },
});

// send it as a failure, and raise it again on the caller's side
const failure = Failure.of(error);
const raised = Failure.error(failure);

// check the reason, which types its data
if (NoteError.is(raised, "NOTE_BLOCKED")) {
    console.log(raised.data.holder);
}
```

## Reference

### ErrorFamily

`defineErrorFamily({ reasons, audience })` declares a package's family, and the build passes the package's name as its domain.
A `"host"` family's errors reach callers as internal errors, and a `"caller"` family's as declared.

| Verb                            | What it does                                                         |
| ------------------------------- | -------------------------------------------------------------------- |
| `of(reason, message, { data })` | creates an error of a reason, checking its data against the schema   |
| `is(error, reason)`             | reports whether an error has the family's reason, and types its data |
| `describe()`                    | lists the reasons with their codes, as the package manifest has them |
| `reason`                        | the schema of the family's reason names, such as for a column        |

### Reason

A reason is `<NOUN>_<CONDITION>`, and its condition sets the code and the shape of the message.
A message is one lowercase sentence for developers, with each given value quoted.

| Condition         | Code                    | Message                                              |
| ----------------- | ----------------------- | ---------------------------------------------------- |
| `NOT_FOUND`       | `NOT_FOUND`             | `<noun> '<value>' not found`                         |
| `EXISTS`          | `CONFLICT`              | `<noun> '<value>' already exists`                    |
| `CONFLICT`        | `CONFLICT`              | `cannot <verb> <noun> '<value>': <why>`              |
| `INVALID`         | `BAD_REQUEST`           | `<noun> '<value>' is invalid: <what> must …`         |
| `REQUIRED`        | `BAD_REQUEST`           | `<what> is required`                                 |
| `UNAUTHENTICATED` | `UNAUTHORIZED`          | `authentication required: <why>`                     |
| `DENIED`          | `FORBIDDEN`             | `<noun> '<value>' is forbidden: <who> cannot <verb>` |
| `BLOCKED`         | `PRECONDITION_FAILED`   | `cannot <verb> <noun> '<value>': <why>`              |
| `EXHAUSTED`       | `TOO_MANY_REQUESTS`     | `<noun> '<value>' exceeds <limit>`                   |
| `UNSUPPORTED`     | `NOT_IMPLEMENTED`       | `<thing> is not supported`                           |
| `MOVED`           | `MISDIRECTED_REQUEST`   | `<noun> '<value>' moved to '<where>'`                |
| `UNAVAILABLE`     | `SERVICE_UNAVAILABLE`   | `<noun> '<value>' is unavailable: <why>`             |
| `UNREACHABLE`     | `BAD_GATEWAY`           | `could not <verb> <noun> '<value>'`                  |
| `CANCELLED`       | `CLIENT_CLOSED_REQUEST` | `<what> was cancelled`                               |
| `BROKEN`          | `INTERNAL_SERVER_ERROR` | `<noun> '<value>' <fact that must never hold>`       |

### Failure

A failure is the JSON a call answers with: `{ code, reason, domain, message, data }`.
Its code is an HTTP status name, and an error without a reason gets the `http` domain with its code as reason.

| Verb                           | What it does                                                            |
| ------------------------------ | ----------------------------------------------------------------------- |
| `Failure.of(error)`            | describes any thrown value, an undeclared one as an internal error      |
| `Failure.error(failure)`       | raises a received failure again, so `is` checks it as the thrown error  |
| `Failure.read(response)`       | reads a failed response's failure, else one from its status and text    |
| `Failure.http(code, message)`  | creates a failure without a reason                                      |
| `Failure.status(failure)`      | reads the HTTP status of its code                                       |
| `Failure.isDeclared(error)`    | reports whether a thrown value has a failure other than an internal one |
| `Failure.isTransient(failure)` | reports whether a later attempt may succeed                             |

`isTransient` answers yes for `CONFLICT`, `MOVED` and `UNAVAILABLE`, and for `EXHAUSTED` only with a `retryAfter` in its data, as in AIP-194.

## Examples

### Read a failed response

`Failure.read` reads the failure of a response, and `Failure.error` raises it with its reason intact.

```ts
import { Failure } from "@destack/error";

/** Read a note's JSON, raising the failure the server answered with. */
export async function fetchNote(url: string): Promise<unknown> {
    // raise a failed response as its failure
    const response = await fetch(url);
    if (!response.ok) {
        throw Failure.error(await Failure.read(response));
    }

    return await response.json();
}
```

### Retry a lost race

`Failure.isTransient` tells a lost race or a busy peer from a failure that stays.

```ts
import { Failure } from "@destack/error";

/** Run an attempt, once more when it failed in a way a later attempt may not. */
export async function runTwice<Value>(attempt: () => Promise<Value>): Promise<Value> {
    try {
        return await attempt();
    } catch (error) {
        // give up on a failure that stays
        if (!Failure.isTransient(Failure.of(error))) {
            throw error;
        }

        return await attempt();
    }
}
```

### Keep a failure from callers

A `"host"` family's errors reach callers as internal errors, while the host's logs keep the reason.

```ts
import { defineErrorFamily } from "@destack/error";

// declare failures only the host should read
export const StorageError = defineErrorFamily({
    reasons: { DISK_BROKEN: {} },
    audience: "host",
});

/** Refuse a write the disk lost, which callers see as an internal error. */
export function refuseLostWrite(path: string): never {
    throw StorageError.of("DISK_BROKEN", `file '${path}' lost its last write`);
}
```

## Comparison

### gRPC ErrorInfo

Reasons, domains and data basically follow gRPC's `ErrorInfo`, as AIP-193 describes it.
Codes are HTTP status names instead of gRPC codes, and a reason's condition picks its code.

| gRPC                 | `@destack/error`       | Difference                                                      |
| -------------------- | ---------------------- | --------------------------------------------------------------- |
| `ErrorInfo.reason`   | `reason`               | Ends with a condition, so `NOTE_NOT_FOUND` is a 404 everywhere. |
| `ErrorInfo.domain`   | `domain`               | The package's name, which the build passes.                     |
| `ErrorInfo.metadata` | `data`                 | Any JSON, checked by the reason's schema and typed by `is`.     |
| `Status.code`        | `code`                 | An HTTP status name, as in oRPC and RFC 9110.                   |
| `RetryInfo`          | `retryAfter` in `data` | Read by `isTransient` for an `EXHAUSTED` reason.                |
