# @destack/error

Name the call errors Destack's callers receive: the codes, the HTTP status each declares, the report a failure converts to, and the failure the wire sends.

## Codes

`CALL_ERROR_STATUSES` gives each `CallErrorCode` its HTTP status: HTTP reason names for HTTP meanings, Destack names for Destack's own.

```ts
import { CALL_ERROR_STATUSES, type CallErrorCode } from "@destack/error";

CALL_ERROR_STATUSES.TOO_MANY_REQUESTS; // 429
CALL_ERROR_STATUSES.INSUFFICIENT_GRANT; // 403
const code: CallErrorCode = "MISDIRECTED_REQUEST"; // 421
```

## Domain errors

A `DomainError` is a package's failure that knows the `CallErrorReport` its caller receives, which `CallError.of` in `@destack/object` answers without the package depending on objects.

```ts
import { DomainError, type CallErrorReport } from "@destack/error";

class NoteError extends Error implements DomainError {
    toCallError(): CallErrorReport {
        return { code: "NOT_FOUND", message: this.message };
    }
}

if (DomainError.is(error)) {
    error.toCallError(); // { code: "NOT_FOUND", message: "no note-0199…" }
}
```

## Failures

`Failure` is a call error as the wire sends it: `Failure.of` describes any thrown value, a domain error as its call error, and `Failure.error` raises a received failure again.

```ts
import { Failure } from "@destack/error";

Failure.of(new NoteError("no note-0199…")); // { code: "NOT_FOUND", status: 404, message: "no note-0199…" }
throw Failure.error(Failure.parse(await response.json())); // a domain error with the failure's call error
```
