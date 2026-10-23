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

## Domain errors

A `DomainError` is a package's failure that knows the `Failure` its caller receives, its `reason` a cause within the package's domain.

```ts
import { DomainError, type Failure } from "@destack/error";

class NoteError extends Error implements DomainError {
    toCallError(): Failure {
        return { code: "NOT_FOUND", reason: "NOTE_NOT_FOUND", domain: "@example/notes", message: this.message };
    }
}

if (DomainError.is(error)) {
    error.toCallError(); // { code: "NOT_FOUND", reason: "NOTE_NOT_FOUND", domain: "@example/notes", message: "no note-0199…" }
}
```

## Failures

`Failure` is a call error as the wire sends it: `Failure.of` describes any thrown value, one without a package's reason as HTTP's, and `Failure.error` raises a received failure again.

```ts
import { Failure } from "@destack/error";

Failure.of(new NoteError("no note-0199…")); // { code: "NOT_FOUND", reason: "NOTE_NOT_FOUND", domain: "@example/notes", … }
Failure.http("BAD_GATEWAY", "bad gateway"); // { code: "BAD_GATEWAY", reason: "BAD_GATEWAY", domain: "http", … }
throw Failure.error(await Failure.read(response)); // a domain error with the failure's call error
```
