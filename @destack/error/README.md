# @destack/error

Name the call errors Destack's callers receive as gRPC's `ErrorInfo` does (AIP-193): an HTTP code, a stable `reason` within its `domain`, a message and details.

## Codes

`CALL_ERROR_STATUSES` gives each `CallErrorCode` its HTTP status: oRPC's codes, and RFC 9110's names for the statuses oRPC lacks.

```ts
import { CALL_ERROR_STATUSES, type CallErrorCode } from "@destack/error";

CALL_ERROR_STATUSES.TOO_MANY_REQUESTS; // 429
CALL_ERROR_STATUSES.MISDIRECTED_REQUEST; // 421
const code: CallErrorCode = "PAYMENT_REQUIRED"; // 402
```

## Families

`defineErrorFamily` declares a package's reasons and the code each answers.

```ts
import { defineErrorFamily } from "@destack/error";

const NoteError = defineErrorFamily(import.meta.destack.package, {
    NOTE_NOT_FOUND: { code: "NOT_FOUND" },
    NOTE_LOCKED: { code: "CONFLICT", data: schema.object({ holder: schema.string() }) },
});

throw NoteError.of("NOTE_LOCKED", "note Plans is locked by Ada: wait until Ada closes it", {
    data: { holder: "ada" },
});
if (NoteError.is(error, "NOTE_LOCKED")) error.data.holder; // "ada", raised here or returned by a call
NoteError.reason; // the schema of the reason names
```

## Messages

A message is one sentence a person reads, saying what failed, why, and the fix when there is one, with each named thing shown by its name.

```ts
// a named thing by its name: a package's name, a space's address, an installation's or account's name
throw StackError.of("VERSION_BOUND", `secret version ${version} is bound by ${name}`);
// not: "secret version 3 is bound by installation-01a1b530-4894-7379-9216-ab9b9e3857f1"

// a thing known only by its identifier, such as a build, by its identifier
throw StackError.of("BUILD_EXPIRED", `build ${build.id} expired`);

// an identifier a caller needs in data, as AIP-193 keeps identifiers in ErrorInfo's metadata
throw NoteError.of("NOTE_LOCKED", "note Plans is locked by Ada", { data: { holder: "ada" } });

// a broken invariant as a TypeError, keeping the identifiers a developer looks up
throw new TypeError(`installation ${installationId} manages itself through a cycle`);
```

## Domain errors

A `DomainError` is a package's failure that knows the `Failure` its caller receives, its `reason` a cause within the package's domain.

```ts
import { DomainError, type Failure } from "@destack/error";

class NoteError extends Error implements DomainError {
    toCallError(): Failure {
        return {
            code: "NOT_FOUND",
            reason: "NOTE_NOT_FOUND",
            domain: "@example/notes",
            message: this.message,
        };
    }
}

if (DomainError.is(error)) {
    error.toCallError(); // { code: "NOT_FOUND", reason: "NOTE_NOT_FOUND", domain: "@example/notes", message: "no note Plans" }
}
```

## Failures

`Failure` is a call error as the wire sends it: `Failure.of` describes any thrown value, one without a package's reason as HTTP's, and `Failure.error` raises a received failure again.

```ts
import { Failure } from "@destack/error";

Failure.of(new NoteError("no note Plans")); // { code: "NOT_FOUND", reason: "NOTE_NOT_FOUND", domain: "@example/notes", … }
Failure.http("BAD_GATEWAY", "bad gateway"); // { code: "BAD_GATEWAY", reason: "BAD_GATEWAY", domain: "http", … }
throw Failure.error(await Failure.read(response)); // a domain error with the failure's call error
```
