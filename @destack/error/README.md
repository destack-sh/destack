# @destack/error

Name the call errors Destack's callers receive: the codes, the HTTP status each declares, and the report a failure converts to.

## Codes

`CALL_ERROR_STATUSES` gives each `CallErrorCode` its HTTP status: HTTP reason names for HTTP meanings, Destack names for Destack's own.

```ts
import { CALL_ERROR_STATUSES, type CallErrorCode } from "@destack/error";

CALL_ERROR_STATUSES.TOO_MANY_REQUESTS; // 429
CALL_ERROR_STATUSES.INSUFFICIENT_GRANT; // 403
const code: CallErrorCode = "MISDIRECTED_REQUEST"; // 421
```

## Domain errors

A `DomainError` is a package's failure that knows the `CallErrorReport` its caller receives, which `CallError.of` in `@destack/runtime` answers without the package depending on the runtime.

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
